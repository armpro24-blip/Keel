"""debug_trace: a call-by-call timeline of a Keel session log, for reading how
the model debugged. Read-only; prints one block per model call with every
tool call it made, the verdict, whether the result was an error, mechanical
flags, and the first lines of the observation the model saw.

    python debug_trace.py <session>.jsonl [--from N] [--to M] [--helpers a.py,b.py] [--lines K] [--width W]

Flags per tool call (all mechanical; the reading is the reviewer's):

    repeat xK    the same command was already issued K times: for shell the
                 same argv and mode (the intent prose is ignored), for
                 edit_file the same path, old_text and new_text (the review
                 text is ignored), for other tools the same input
    retrieve     a direct `pira_ctx` retrieval (search, range, transform, exec, raw,
                 list, history, recap, watch); the result IDs it names are listed
    helper       argv or edit path mentions one of the --helpers files
    id:<ID>      the observation returned a pira_ctx result ID (Captured:/Result:)

The summary counts calls, errors, repeats, retrievals, helper-related calls,
and lists every result ID the model was given together with whether any later
call retrieved from it. Nothing here scores or classifies; the protocol's
categories are applied by hand on top of this table.

Output is written as UTF-8 regardless of the console code page, so a
redirected run on Windows does not fail on characters such as `→`.
"""

import argparse
import json
import re
import sys

RESULT_ID = re.compile(r"\b\d{8}-\d{6}-[0-9a-f]{12}\b")
RETRIEVAL = {"search", "range", "transform", "exec", "raw", "list", "history", "recap", "watch"}


def load(path):
    with open(path, encoding="utf-8") as handle:
        return [json.loads(line) for line in handle if line.strip()]


def program_name(argv):
    if not argv:
        return ""
    name = str(argv[0]).replace("\\", "/").rsplit("/", 1)[-1]
    return name[:-4] if name.lower().endswith(".exe") else name


def command_key(tool, value):
    """What counts as 'the same command' for the repeat flag."""
    if isinstance(value, dict):
        if tool == "shell":
            value = {"argv": value.get("argv"), "mode": value.get("mode")}
        elif tool == "edit_file":
            value = {k: value.get(k) for k in ("path", "old_text", "new_text")}
    return tool + "\x00" + json.dumps(value, sort_keys=True, ensure_ascii=False)


def summarize_input(tool, value, width):
    if tool == "shell" and isinstance(value, dict) and isinstance(value.get("argv"), list):
        text = " ".join(str(a) for a in value["argv"])
        mode = value.get("mode")
        prefix = f"[{mode}] " if mode else ""
        return clip(prefix + text, width)
    if tool == "edit_file" and isinstance(value, dict):
        path = str(value.get("path", "")).replace("\\", "/").rsplit("/", 1)[-1]
        old = str(value.get("old_text", ""))
        new = str(value.get("new_text", ""))
        first_old = old.splitlines()[0] if old.strip() else ""
        return clip(f"{path}: {len(old)}B -> {len(new)}B  old[0]={first_old!r}", width)
    return clip(json.dumps(value, ensure_ascii=False), width)


def clip(text, width):
    text = text.replace("\n", "\\n")
    return text if len(text) <= width else text[: width - 1] + "…"


def calls_of(events):
    """Yield (call_index, text, tool_calls, results, decisions) per assistant message."""
    decisions = {e["call_id"]: e for e in events if e.get("event") == "decision"}
    results = {}
    for e in events:
        if e.get("event") == "message" and e.get("role") == "user":
            for block in e["blocks"]:
                if block["type"] == "tool_result":
                    results[block["call_id"]] = block
    index = 0
    for e in events:
        if e.get("event") == "message" and e.get("role") == "assistant":
            index += 1
            tool_calls = [b for b in e["blocks"] if b["type"] == "tool_call"]
            text = "".join(b["text"] for b in e["blocks"] if b["type"] == "text")
            yield index, text, tool_calls, results, decisions


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("log")
    parser.add_argument("--from", dest="start", type=int, default=1, help="first model call to print")
    parser.add_argument("--to", dest="end", type=int, default=None, help="last model call to print")
    parser.add_argument("--helpers", default="", help="comma-separated file names the model created (from git status)")
    parser.add_argument("--lines", type=int, default=3, help="observation lines to show per result")
    parser.add_argument("--width", type=int, default=150)
    args = parser.parse_args()

    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    helpers = [h.strip().replace("\\", "/").rsplit("/", 1)[-1] for h in args.helpers.split(",") if h.strip()]
    events = load(args.log)
    seen = {}
    given_ids = {}      # result id -> call index where the model received it
    retrieved_ids = {}  # result id -> list of call indexes that retrieved from it
    totals = {"calls": 0, "tool_calls": 0, "errors": 0, "repeats": 0, "retrievals": 0, "helper": 0, "empty": 0}

    for index, text, tool_calls, results, decisions in calls_of(events):
        totals["calls"] += 1
        in_range = index >= args.start and (args.end is None or index <= args.end)
        if not tool_calls and not text.strip():
            totals["empty"] += 1
        if in_range:
            head = f"call {index:>3}"
            if text.strip():
                head += f"  text: {clip(text.strip(), args.width - 20)}"
            print(head)
        for call in tool_calls:
            totals["tool_calls"] += 1
            tool, value = call["name"], call["input"]
            key = command_key(tool, value)
            repeats = seen.get(key, 0)
            seen[key] = repeats + 1
            decision = decisions.get(call["id"], {}).get("decision")
            verdict = "allow" if decision == "allow" else ("deny" if decision else "?")
            result = results.get(call["id"])
            is_error = bool(result and result.get("is_error"))
            output = (result or {}).get("output", "")

            flags = []
            if repeats:
                flags.append(f"repeat x{repeats}")
                totals["repeats"] += 1
            argv = value.get("argv") if isinstance(value, dict) else None
            if tool == "shell" and isinstance(argv, list) and program_name(argv) == "pira_ctx" and len(argv) > 1 and argv[1] in RETRIEVAL:
                flags.append("retrieve")
                totals["retrievals"] += 1
                for rid in RESULT_ID.findall(" ".join(str(a) for a in argv)):
                    retrieved_ids.setdefault(rid, []).append(index)
            mentions = " ".join(str(a) for a in argv) if isinstance(argv, list) else str(value.get("path", "")) if isinstance(value, dict) else ""
            if helpers and any(h in mentions.replace("\\", "/") for h in helpers):
                flags.append("helper")
                totals["helper"] += 1
            for rid in RESULT_ID.findall(output):
                given_ids.setdefault(rid, index)
                if f"id:{rid}" not in flags:
                    flags.append(f"id:{rid}")
            if is_error:
                totals["errors"] += 1

            if in_range:
                flag_text = f"  [{' '.join(flags)}]" if flags else ""
                print(f"  {tool:<16} {verdict:<5} {'ERR ' if is_error else 'ok  '}{summarize_input(tool, value, args.width)}{flag_text}")
                if verdict == "deny":
                    print(f"    deny: {clip(str(decision.get('deny', '')), args.width)}")
                for line in output.splitlines()[: args.lines]:
                    print(f"    | {clip(line, args.width)}")
        if in_range:
            print()

    print("summary")
    print(f"  model calls {totals['calls']}  tool calls {totals['tool_calls']}  error results {totals['errors']}  empty responses {totals['empty']}")
    print(f"  repeated identical calls {totals['repeats']}  pira_ctx retrievals {totals['retrievals']}  helper-related calls {totals['helper']}")
    print(f"  pira_ctx result IDs given to the model: {len(given_ids)}; retrieved from later: {sum(1 for r in given_ids if r in retrieved_ids)}")
    for rid, at in given_ids.items():
        later = retrieved_ids.get(rid, [])
        print(f"    {rid}  given at call {at}  retrieved at {later if later else 'never'}")
    for rid, at in retrieved_ids.items():
        if rid not in given_ids:
            print(f"    {rid}  retrieved at {at} but never seen in an observation")


if __name__ == "__main__":
    main()
