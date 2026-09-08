"""Gate B: can the serving stack place a multi-line payload in a dedicated
`stdin` string field while keeping `argv` a JSON array?

    python gate_b.py --base-url http://HOST:8000/v1 --model MODEL --out DIR [--runs 10] [--dry-run]

Diagnostic only; production Keel is not involved. The request is what Keel
would send at the start of an `ask`-mode session in the L1 repository:
system = ~/agent/AGENTS.md verbatim + one blank line + the <keel_host>
block; tools = read_pira_policy (names parsed from the routing table) and
shell, whose schema is the current Keel schema plus one property:

    "stdin": {"type": "string", "description": "Optional UTF-8 text supplied
    verbatim to the program's standard input. Use for non-interactive programs
    that read from stdin."}

Nothing else in the request differs from Keel's. No sampling parameters are
set (Keel sets none). Every raw response body is saved to DIR/response_NN.json
and the request to DIR/request.json; the score table is printed and saved to
DIR/scores.txt.
"""

import argparse
import json
import os
import re
import sys
import urllib.request
from datetime import datetime, timezone

# The payload: a plausible cli.py with --month, 54 lines (1,835 bytes), single and double
# quotes, an f-string with braces, a regex, backslashes. Comparable to what
# the model tried to write in L1.
PAYLOAD = '''"""Command-line entry point: ``python -m tally report FILE [--top N] [--month YYYY-MM]``."""

import argparse
import re
import sys

from .records import RecordError, read_records
from .report import render, totals_by_category


def month(text):
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
    report.add_argument("--month", type=month, metavar="YYYY-MM", help="only records in this month")
    return parser


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        records = read_records(args.path)
    except (OSError, RecordError) as error:
        print(f"tally: {error}", file=sys.stderr)
        return 1
    if args.month:
        records = [r for r in records if r.day.strftime("%Y-%m") == args.month]
    totals = totals_by_category(records)
    shown = totals
    if args.top is not None:
        shown = dict(list(totals.items())[: args.top])
    output = render(shown)
    if shown is not totals:
        # The total line must still cover every category, not just the shown ones.
        lines = output.split("\\n")
        lines[-1] = f"{'total':<12}{sum(totals.values()):>10.2f}"
        output = "\\n".join(lines)
    print(output)
    return 0
'''

TASK = (
    "Replace the contents of tally/cli.py with exactly the following file body. "
    "Use the shell tool once: run python with argv [\"python\", \"-\"] and put a "
    "short Python program on stdin that writes the body below to tally/cli.py "
    "(UTF-8, LF line endings). The file body must be reproduced exactly, byte for "
    "byte, including the docstring, blank lines and indentation.\n\n"
    "```python\n" + PAYLOAD + "```"
)

SHELL_HINT_WINDOWS = (
    'request one explicitly: ["powershell","-Command","..."] or ["cmd","/C","..."]; '
    "cmd built-ins such as dir, type, copy are not programs"
)
SHELL_HINT_UNIX = 'request one explicitly: ["sh","-c","..."] or ["bash","-lc","..."]'

ASK_MODE = (
    "ask (the user confirms each action before it runs; a safety_review you supply "
    "is shown to the user; loading PIRA policy needs no confirmation)"
)


def shell_tool(windows):
    hint = SHELL_HINT_WINDOWS if windows else SHELL_HINT_UNIX
    return {
        "type": "function",
        "function": {
            "name": "shell",
            "description": (
                "Run a program with arguments. argv is executed directly, with no shell: "
                "redirection, pipes, and && are not interpreted, and passing them as arguments "
                f"is rejected. If you need a shell, {hint}. Every command runs through pira_ctx with "
                "your intent, except PIRA internal tools, which run directly. Returns stdout, "
                "stderr, and the exit code. In full-permission/no-approval mode a state_changing "
                "command needs a safety_review before it runs."
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "argv": {
                        "type": "array",
                        "items": {"type": "string"},
                        "minItems": 1,
                        "description": "Program followed by its arguments.",
                    },
                    "intent": {
                        "type": "string",
                        "description": "One line, at most 256 UTF-8 bytes: prospective action + target + purpose (pira_ctx --intent).",
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["auto", "check", "capture", "exact"],
                        "description": "pira_ctx mode; omit for auto.",
                    },
                    "interest": {
                        "type": "string",
                        "description": "Regex whose matching output lines must dominate the pira_ctx synopsis.",
                    },
                    "workdir": {
                        "type": "string",
                        "description": "Working directory relative to the workspace root; default is the root. Directories outside the workspace need the user's confirmation.",
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Kill the command after this many seconds. Omit to wait.",
                    },
                    "effect": {
                        "type": "string",
                        "enum": ["read_only", "state_changing"],
                        "description": "Your judgment of this command as issued: read_only if it changes no file, repository, tool, user, or system state; otherwise state_changing.",
                    },
                    "safety_review": {
                        "type": "string",
                        "description": "Required when effect is state_changing in full-permission/no-approval mode: the review PIRA's Full-Permission Behavior requires before this command. Keel shows it as 'Safety: ...' before executing.",
                    },
                    # The one addition under test.
                    "stdin": {
                        "type": "string",
                        "description": "Optional UTF-8 text supplied verbatim to the program's standard input. Use for non-interactive programs that read from stdin.",
                    },
                },
                "required": ["argv", "intent", "effect"],
            },
        },
    }


def loader_tool(names):
    return {
        "type": "function",
        "function": {
            "name": "read_pira_policy",
            "description": (
                "Read one PIRA policy source exactly, by the name PIRA's routing table uses "
                f"({', '.join(names)}). Use this, not a shell command, to load a PIRA module or the user profile."
            ),
            "parameters": {
                "type": "object",
                "properties": {"name": {"type": "string", "enum": names}},
                "required": ["name"],
            },
        },
    }


def routing_names(agents_md):
    section = agents_md.split("## Module Loading and Routing", 1)[1].split("\n## ", 1)[0]
    return re.findall(r"^- `([A-Za-z_]+)`: `~/", section, flags=re.M)


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
    system = agents_md + "\n" + host_block
    return {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": TASK},
        ],
        "tools": [loader_tool(routing_names(agents_md)), shell_tool(windows)],
    }


def score(body):
    """One row of the score table plus notes."""
    row = {
        "valid_shell_call": False,
        "argv_array": False,
        "stdin_string": False,
        "stdin_complete": False,
        "fields_separate": False,
        "note": "",
    }
    try:
        message = body["choices"][0]["message"]
    except (KeyError, IndexError, TypeError):
        row["note"] = "no choices[0].message"
        return row
    calls = message.get("tool_calls") or []
    if not calls:
        row["note"] = "no tool call (content=%r)" % ((message.get("content") or "")[:80],)
        return row
    call = calls[0]["function"]
    if call.get("name") != "shell":
        row["note"] = f"first call is {call.get('name')!r}"
        return row
    try:
        args = json.loads(call.get("arguments") or "")
    except json.JSONDecodeError as error:
        row["note"] = f"arguments not JSON: {error}"
        return row
    if not isinstance(args, dict):
        row["note"] = "arguments not an object"
        return row
    row["valid_shell_call"] = True
    argv = args.get("argv")
    row["argv_array"] = isinstance(argv, list) and all(isinstance(a, str) for a in argv) and len(argv) >= 1
    stdin = args.get("stdin")
    row["stdin_string"] = isinstance(stdin, str)
    if row["stdin_string"]:
        # Materially complete: every non-blank payload line appears in stdin
        # (the model may wrap the body in its own writer program).
        lines = [line for line in PAYLOAD.splitlines() if line.strip()]
        missing = [line for line in lines if line not in stdin]
        row["stdin_complete"] = not missing
        if missing:
            row["note"] = f"{len(missing)}/{len(lines)} payload lines missing from stdin; first: {missing[0][:60]!r}"
        row["stdin_bytes"] = len(stdin.encode("utf-8"))
    row["fields_separate"] = (
        isinstance(args.get("intent"), str)
        and args.get("effect") in ("read_only", "state_changing")
        and not (isinstance(argv, str))
    )
    row["argv"] = argv if isinstance(argv, list) else repr(argv)[:80]
    row["effect"] = args.get("effect")
    row["finish"] = body["choices"][0].get("finish_reason")
    row["usage"] = body.get("usage")
    return row


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--runs", type=int, default=10)
    parser.add_argument("--agents-md", default=os.path.expanduser("~/agent/AGENTS.md"))
    parser.add_argument("--cwd", default=r"C:\Users\LM\Desktop\tally-l1")
    parser.add_argument("--unix", action="store_true", help="linux host block and shell hint")
    parser.add_argument("--api-key", default=os.environ.get("OPENAI_API_KEY", "dummy"))
    parser.add_argument("--dry-run", action="store_true", help="write request.json only")
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
        data = json.dumps(request).encode("utf-8")
        http = urllib.request.Request(
            f"{args.base_url.rstrip('/')}/chat/completions",
            data=data,
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {args.api_key}"},
        )
        try:
            with urllib.request.urlopen(http, timeout=600) as response:
                body = json.loads(response.read().decode("utf-8"))
        except Exception as error:  # record and continue; the raw error is evidence too
            body = {"error": str(error)}
        with open(os.path.join(args.out, f"response_{index:02d}.json"), "w", encoding="utf-8") as handle:
            json.dump(body, handle, ensure_ascii=False, indent=1)
        row = score(body)
        rows.append(row)
        print(f"run {index:02d}: " + json.dumps({k: v for k, v in row.items() if k != 'usage'}, ensure_ascii=False))

    keys = ["valid_shell_call", "argv_array", "stdin_string", "stdin_complete", "fields_separate"]
    lines = ["metric                         score"]
    for key in keys:
        lines.append(f"{key:<30} {sum(1 for r in rows if r.get(key))}/{len(rows)}")
    table = "\n".join(lines)
    print(table)
    with open(os.path.join(args.out, "scores.txt"), "w", encoding="utf-8") as handle:
        handle.write(table + "\n\n" + "\n".join(json.dumps(r, ensure_ascii=False) for r in rows) + "\n")


if __name__ == "__main__":
    sys.exit(main())
