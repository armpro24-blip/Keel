"""Gate C: can the tool-call channel carry a file body directly in a
top-level `content` string, byte for byte, with no writer program?

    python gate_c.py --base-url http://HOST:8000/v1 --model MODEL --out DIR [--runs 10] [--dry-run]

Diagnostic only; production Keel is not involved and has no write_file tool.
The request uses the same system prompt as Gate B (~/agent/AGENTS.md
verbatim + blank line + <keel_host> block, ask mode) and the same payload
(imported from gate_b.py so it is identical by construction). The only tool
exposed is the diagnostic write_file below. No sampling parameters. Every raw
response is saved to DIR/response_NN.json, the request to DIR/request.json,
scores to DIR/scores.txt.

Gate metric: content_byte_exact, i.e. arguments["content"].encode("utf-8")
== PAYLOAD.encode("utf-8") after ordinary JSON parsing. No newline or CRLF
normalization participates in the gate; a "trailing newline only" column is
printed as a diagnostic.
"""

import argparse
import difflib
import json
import os
import sys
import urllib.request
from datetime import datetime, timezone

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "stdin"))
from gate_b import ASK_MODE, PAYLOAD  # noqa: E402

WRITE_FILE_TOOL = {
    "type": "function",
    "function": {
        "name": "write_file",
        "description": "Write the supplied UTF-8 content to the specified file.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "content": {"type": "string"},
            },
            "required": ["path", "content"],
        },
    },
}

TASK = (
    "Call write_file exactly once with path tally/cli.py and content equal to the "
    "following file body, exactly as given.\n\n"
    "```python\n" + PAYLOAD + "```"
)


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
        "tools": [WRITE_FILE_TOOL],
    }


def looks_like_collapse(raw_arguments, parsed):
    """Does this resemble the L1 failure (structured arguments folded into a string)?"""
    if parsed is None or not isinstance(parsed, dict):
        return True
    for value in parsed.values():
        if isinstance(value, str) and value.lstrip().startswith("{") and '"content"' in value:
            return True
    return False


def score(body):
    row = {
        "valid_write_file_call": False,
        "path_exact": False,
        "content_is_string": False,
        "content_byte_exact": False,
        "trailing_newline_only_diff": False,
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
    if call.get("name") != "write_file":
        row["note"] = f"first call is {call.get('name')!r}"
        return row
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as error:
        parsed = None
        row["note"] = f"arguments not JSON: {error}"
    row["looks_like_l1_collapse"] = looks_like_collapse(raw, parsed)
    if not isinstance(parsed, dict):
        row["arguments_prefix"] = raw[:300]
        return row
    row["valid_write_file_call"] = True
    row["path_exact"] = parsed.get("path") == "tally/cli.py"
    content = parsed.get("content")
    row["content_is_string"] = isinstance(content, str)
    if row["content_is_string"]:
        got = content.encode("utf-8")
        want = PAYLOAD.encode("utf-8")
        row["content_bytes"] = len(got)
        row["content_byte_exact"] = got == want
        if not row["content_byte_exact"]:
            row["trailing_newline_only_diff"] = got.rstrip(b"\n") == want.rstrip(b"\n")
            diff = list(
                difflib.unified_diff(
                    PAYLOAD.splitlines(), content.splitlines(), "payload", "content", lineterm="", n=0
                )
            )
            row["diff"] = diff[2:12]
    if not row["path_exact"]:
        row["path"] = parsed.get("path")
    if not (row["path_exact"] and row["content_byte_exact"]):
        row["arguments_prefix"] = raw[:300]
    return row


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--runs", type=int, default=10)
    parser.add_argument("--agents-md", default=os.path.expanduser("~/agent/AGENTS.md"))
    parser.add_argument("--cwd", default=r"C:\Users\LM\Desktop\tally-l1")
    parser.add_argument("--unix", action="store_true")
    parser.add_argument("--api-key", default=os.environ.get("OPENAI_API_KEY", "dummy"))
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    os.makedirs(args.out, exist_ok=True)
    with open(args.agents_md, encoding="utf-8") as handle:
        agents_md = handle.read()
    request = build_request(args.model, agents_md, args.cwd, not args.unix)
    with open(os.path.join(args.out, "request.json"), "w", encoding="utf-8") as handle:
        json.dump(request, handle, ensure_ascii=False, indent=1)
    print(f"request written: {len(json.dumps(request))} bytes; payload {len(PAYLOAD.encode())} bytes, {PAYLOAD.count(chr(10))} lines")
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
        row = score(body)
        rows.append(row)
        print(f"run {index:02d}: " + json.dumps({k: v for k, v in row.items() if k not in ('usage', 'diff')}, ensure_ascii=False))
        for line in row.get("diff", []):
            print("    " + line)

    keys = ["valid_write_file_call", "path_exact", "content_is_string", "content_byte_exact"]
    lines = ["metric                         score"]
    for key in keys:
        lines.append(f"{key:<30} {sum(1 for r in rows if r.get(key))}/{len(rows)}")
    lines.append(f"{'trailing_newline_only_diff':<30} {sum(1 for r in rows if r.get('trailing_newline_only_diff'))}/{len(rows)}  (diagnostic only)")
    lines.append(f"{'looks_like_l1_collapse':<30} {sum(1 for r in rows if r.get('looks_like_l1_collapse'))}/{len(rows)}")
    table = "\n".join(lines)
    print(table)
    with open(os.path.join(args.out, "scores.txt"), "w", encoding="utf-8") as handle:
        handle.write(table + "\n\n" + "\n".join(json.dumps(r, ensure_ascii=False) for r in rows) + "\n")


if __name__ == "__main__":
    sys.exit(main())
