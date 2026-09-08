"""Gate D: can the tool-call channel carry one exact-replace edit
(path, old_text, new_text) that reproduces the expected file byte for byte?

    python gate_d.py --freeze                     # write frozen/ (original, old_text, new_text, expected) and verify
    python gate_d.py --base-url URL --model M --out DIR [--runs 10] [--dry-run]

Diagnostic only; Keel has no edit_file tool. Same system prompt as Gates B
and C (~/agent/AGENTS.md verbatim + blank line + <keel_host> block, ask
mode). One tool exposed: the diagnostic edit_file below. No sampling
parameters. Raw responses saved to DIR/response_NN.json, the request to
DIR/request.json, scores to DIR/scores.txt.

The frozen edit is a real L1 change in the seed tally/cli.py: replace the
block from the imports through the end of build_parser with the same block
plus `import re`, a month() validator, and the --month option on the report
subcommand, so the expected file runs and rejects a malformed month as a
usage error. old_text occurs exactly once in the seed and ends well before
EOF. The script re-derives the frozen files from its
constants at start and refuses to run if the committed copies differ.

Primary gate metric: replacement_produces_expected_file, computed as: parse
the arguments as JSON; require old_text to occur exactly once in the frozen
seed text; replace that one occurrence with new_text; compare the result
byte for byte with the frozen expected file. No CRLF, whitespace, EOF or
trailing-newline normalization.
"""

import argparse
import difflib
import json
import os
import sys
import urllib.request
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "stdin"))
from gate_b import ASK_MODE  # noqa: E402

SEED_CLI = os.path.join(HERE, "..", "L1", "seed", "tally", "cli.py")
FROZEN = os.path.join(HERE, "frozen")

OLD_TEXT = '''import argparse
import sys

from .records import RecordError, read_records
from .report import render, totals_by_category


def build_parser():
    parser = argparse.ArgumentParser(
        prog="tally", description="Summarize CSV expense records by category."
    )
    commands = parser.add_subparsers(dest="command", required=True)
    report = commands.add_parser("report", help="print totals per category")
    report.add_argument("path", help="CSV file with date, category, amount[, note]")
    report.add_argument(
        "--top",
        type=int,
        metavar="N",
        help="show only the N largest categories (the total still covers all)",
    )
    return parser'''

NEW_TEXT = '''import argparse
import re
import sys

from .records import RecordError, read_records
from .report import render, totals_by_category


def month(text):
    """Validate a ``YYYY-MM`` month for argparse."""
    if not re.fullmatch(r"\\d{4}-(0[1-9]|1[0-2])", text):
        raise argparse.ArgumentTypeError(f"month must be YYYY-MM, got {text!r}")
    return text


def build_parser():
    parser = argparse.ArgumentParser(
        prog="tally", description="Summarize CSV expense records by category."
    )
    commands = parser.add_subparsers(dest="command", required=True)
    report = commands.add_parser("report", help="print totals per category")
    report.add_argument("path", help="CSV file with date, category, amount[, note]")
    report.add_argument(
        "--top",
        type=int,
        metavar="N",
        help="show only the N largest categories (the total still covers all)",
    )
    report.add_argument(
        "--month",
        type=month,
        metavar="YYYY-MM",
        help="only records whose date falls in this month",
    )
    return parser'''

EDIT_FILE_TOOL = {
    "type": "function",
    "function": {
        "name": "edit_file",
        "description": "Replace one exact block of text in an existing file.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "old_text": {"type": "string"},
                "new_text": {"type": "string"},
            },
            "required": ["path", "old_text", "new_text"],
        },
    },
}

TASK = (
    "In tally/cli.py, replace the following existing block of text:\n\n"
    "```python\n" + OLD_TEXT + "\n```\n\n"
    "with this text:\n\n"
    "```python\n" + NEW_TEXT + "\n```\n\n"
    "Call edit_file exactly once to make this change."
)


def read_bytes(path):
    with open(path, "rb") as handle:
        return handle.read()


def derive():
    """Original seed text, and the expected file after the frozen edit."""
    original = read_bytes(SEED_CLI).decode("utf-8")
    assert original.count(OLD_TEXT) == 1, "old_text must occur exactly once in the seed"
    assert not original.endswith(OLD_TEXT), "old_text must not touch EOF"
    expected = original.replace(OLD_TEXT, NEW_TEXT, 1)
    compile(expected, "cli.py", "exec")
    return original, expected


def freeze():
    original, expected = derive()
    os.makedirs(FROZEN, exist_ok=True)
    for name, text in [
        ("original_cli.py", original),
        ("old_text.txt", OLD_TEXT),
        ("new_text.txt", NEW_TEXT),
        ("expected_cli.py", expected),
    ]:
        with open(os.path.join(FROZEN, name), "w", encoding="utf-8", newline="") as handle:
            handle.write(text)
    print(f"frozen: original {len(original.encode())} bytes, old_text {len(OLD_TEXT.encode())} bytes ({OLD_TEXT.count(chr(10)) + 1} lines), new_text {len(NEW_TEXT.encode())} bytes ({NEW_TEXT.count(chr(10)) + 1} lines), expected {len(expected.encode())} bytes")


def verify_frozen():
    original, expected = derive()
    for name, text in [
        ("original_cli.py", original),
        ("old_text.txt", OLD_TEXT),
        ("new_text.txt", NEW_TEXT),
        ("expected_cli.py", expected),
    ]:
        committed = read_bytes(os.path.join(FROZEN, name)).decode("utf-8")
        if committed != text:
            sys.exit(f"frozen/{name} differs from the script's constants; refusing to run")
    return original, expected


def build_request(model, agents_md, cwd, windows):
    if not agents_md.endswith("\n"):
        agents_md += "\n"
    host_block = (
        "<keel_host>\nharness: keel 0.0.1\nplatform: {} x86_64\ndate: {}\ncwd: {}\n"
        "workspace_root: {}\napproval_mode: {}\n</keel_host>\n"
    ).format(
        "windows" if windows else "linux",
        datetime.now(timezone.utc).strftime("%Y-%m-%d"),
        cwd,
        cwd,
        ASK_MODE,
    )
    return {
        "model": model,
        "messages": [
            {"role": "system", "content": agents_md + "\n" + host_block},
            {"role": "user", "content": TASK},
        ],
        "tools": [EDIT_FILE_TOOL],
    }


def looks_like_collapse(parsed):
    if not isinstance(parsed, dict):
        return True
    return any(
        isinstance(v, str) and v.lstrip().startswith("{") and '"old_text"' in v for v in parsed.values()
    )


def score(body, original, expected):
    row = {
        "valid_edit_file_call": False,
        "path_exact": False,
        "old_text_byte_exact": False,
        "new_text_byte_exact": False,
        "old_text_unique_in_seed": False,
        "replacement_produces_expected_file": False,
        "looks_like_l1_collapse": False,
        "n_tool_calls": 0,
        "note": "",
    }
    try:
        choice = body["choices"][0]
        message = choice["message"]
    except (KeyError, IndexError, TypeError):
        row["note"] = "no choices[0].message: " + json.dumps(body)[:200]
        return row
    row["finish"] = choice.get("finish_reason")
    row["usage"] = body.get("usage")
    row["content_text"] = (message.get("content") or "")[:80]
    row["reasoning_chars"] = len(message.get("reasoning") or "")
    calls = message.get("tool_calls") or []
    row["n_tool_calls"] = len(calls)
    if not calls:
        row["note"] = "no tool call"
        return row
    call = calls[0]["function"]
    raw = call.get("arguments") or ""
    if call.get("name") != "edit_file":
        row["note"] = f"first call is {call.get('name')!r}"
        return row
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as error:
        parsed = None
        row["note"] = f"arguments not JSON: {error}"
    row["looks_like_l1_collapse"] = looks_like_collapse(parsed)
    if not isinstance(parsed, dict):
        row["arguments_prefix"] = raw[:300]
        return row
    row["valid_edit_file_call"] = True
    row["path_exact"] = parsed.get("path") == "tally/cli.py"
    old = parsed.get("old_text")
    new = parsed.get("new_text")
    if isinstance(old, str) and isinstance(new, str):
        row["old_text_byte_exact"] = old.encode("utf-8") == OLD_TEXT.encode("utf-8")
        row["new_text_byte_exact"] = new.encode("utf-8") == NEW_TEXT.encode("utf-8")
        occurrences = original.count(old) if old else 0
        row["old_text_occurrences"] = occurrences
        row["old_text_unique_in_seed"] = occurrences == 1
        if occurrences == 1:
            produced = original.replace(old, new, 1)
            row["replacement_produces_expected_file"] = produced.encode("utf-8") == expected.encode("utf-8")
            if not row["replacement_produces_expected_file"]:
                row["diff"] = list(
                    difflib.unified_diff(
                        expected.splitlines(), produced.splitlines(), "expected", "produced", lineterm="", n=0
                    )
                )[2:12]
        row["old_text_bytes"] = len(old.encode("utf-8"))
        row["new_text_bytes"] = len(new.encode("utf-8"))
    else:
        row["note"] = f"old_text/new_text types: {type(old).__name__}/{type(new).__name__}"
    if not row["replacement_produces_expected_file"]:
        row["arguments_prefix"] = raw[:300]
    return row


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--freeze", action="store_true")
    parser.add_argument("--base-url")
    parser.add_argument("--model")
    parser.add_argument("--out")
    parser.add_argument("--runs", type=int, default=10)
    parser.add_argument("--agents-md", default=os.path.expanduser("~/agent/AGENTS.md"))
    parser.add_argument("--cwd", default=r"C:\Users\LM\Desktop\tally-l1")
    parser.add_argument("--unix", action="store_true")
    parser.add_argument("--api-key", default=os.environ.get("OPENAI_API_KEY", "dummy"))
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    if args.freeze:
        freeze()
        return
    if not (args.base_url and args.model and args.out):
        parser.error("--base-url, --model and --out are required unless --freeze")

    original, expected = verify_frozen()
    os.makedirs(args.out, exist_ok=True)
    with open(args.agents_md, encoding="utf-8") as handle:
        agents_md = handle.read()
    request = build_request(args.model, agents_md, args.cwd, not args.unix)
    with open(os.path.join(args.out, "request.json"), "w", encoding="utf-8") as handle:
        json.dump(request, handle, ensure_ascii=False, indent=1)
    print(f"request written: {len(json.dumps(request))} bytes; old_text {len(OLD_TEXT.encode())} bytes, new_text {len(NEW_TEXT.encode())} bytes")
    if args.dry_run:
        return

    rows = []
    for index in range(1, args.runs + 1):
        http = urllib.request.Request(
            f"{args.base_url.rstrip('/')}/chat/completions",
            data=json.dumps(request).encode("utf-8"),
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {args.api_key}"},
        )
        try:
            with urllib.request.urlopen(http, timeout=600) as response:
                body = json.loads(response.read().decode("utf-8"))
        except Exception as error:
            body = {"error": str(error)}
        with open(os.path.join(args.out, f"response_{index:02d}.json"), "w", encoding="utf-8") as handle:
            json.dump(body, handle, ensure_ascii=False, indent=1)
        row = score(body, original, expected)
        rows.append(row)
        print(f"run {index:02d}: " + json.dumps({k: v for k, v in row.items() if k not in ('usage', 'diff')}, ensure_ascii=False))
        for line in row.get("diff", []):
            print("    " + line)

    keys = [
        "valid_edit_file_call",
        "path_exact",
        "old_text_byte_exact",
        "new_text_byte_exact",
        "old_text_unique_in_seed",
        "replacement_produces_expected_file",
        "looks_like_l1_collapse",
    ]
    lines = ["metric                               score"]
    for key in keys:
        tag = "  (gate metric)" if key == "replacement_produces_expected_file" else ""
        lines.append(f"{key:<36} {sum(1 for r in rows if r.get(key))}/{len(rows)}{tag}")
    table = "\n".join(lines)
    print(table)
    with open(os.path.join(args.out, "scores.txt"), "w", encoding="utf-8") as handle:
        handle.write(table + "\n\n" + "\n".join(json.dumps(r, ensure_ascii=False) for r in rows) + "\n")


if __name__ == "__main__":
    sys.exit(main())
