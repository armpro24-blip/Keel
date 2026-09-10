"""score_shell_ab: mechanical scoring of one T25 stage-2 run from its Keel
SessionLog and the run's repository (docs/dogfood/shell_contract/SHELL_CONTRACT_AB.md).

    python score_shell_ab.py --log <session>.jsonl --task R1|R2|D1|D2 --arm A|B --run N --repo <dir> [--header]

Prints one CSV row (and, on stderr, the lists behind the counts). Read-only.

Outcomes, as preregistered:

  end            completion | budget | error | none   (from run_end)
  correct        completion AND the result check passes
  file_correct   D1 only: the file has exactly the expected bytes (even if the run did not complete)
  line_ending    D1 only: the file differs from expected only by CRLF or a UTF-8 BOM
  arg_errors     confirmed argument-usage errors: structural denials (argv not an array, standalone
                 shell operator) plus output evidence that an explicitly invoked shell changed or
                 split an element (Python echoed a line that is not a line of any sent element, or
                 "can't open file")
  prog_errors    the program's own error: a Python SyntaxError whose echoed line is a line of a sent
                 element, whether or not a shell was invoked
  undetermined   a SyntaxError with no echoed line to compare
  helper_calls   helper-file related calls: tool calls naming a file that is neither tracked in the
                 run repository nor the task's own target; the paths are listed for hand review
  model_calls, tool_calls, denials_review (edits/commands denied for a missing review),
  sc_with_review (state-changing calls allowed with a review), final_text (trimmed, first 120 chars)

Nothing here decides the experiment; the gate is applied to the collected rows by hand.
"""

import argparse
import json
import re
import subprocess
import sys

EXPECTED = {
    "R1": "42",
    "R2": "25ba9483d9cf0c91b81cd8d6576d2df3f04eff7340157269172891b2d51c1a44",
    "D2": 'She said "it\'s done" -- path C:\\tmp\\x y',
}
D1_PATH = "notes/summary.txt"
D1_BYTES = b'She said "it\'s done" -- path C:\\tmp\\x y\n'
TASK_FILES = {"D1": {D1_PATH}, "D2": {"data/quotes.jsonl"}, "R1": set(), "R2": set()}

SHELLS = {"cmd", "powershell", "pwsh", "sh", "bash"}
ARGV_COLLAPSE = "input needs an array field 'argv'"
OPERATOR_REJECTED = "without a shell"
MISSING_REVIEW = "needs a non-empty safety_review"
ECHOED_LINE = re.compile(r'File "<string>", line \d+\n[ \t]*(?:L\d+ stderr: )?[ \t]*(.*?)[ \t]*\n')
SYNOPSIS_PREFIX = re.compile(r"^L\d+ std(?:err|out): ", re.M)
FILE_TOKEN = re.compile(r"[\w\-./\\]+\.(?:py|txt|jsonl|json|ps1|bat|cmd|sh|md)\b")


def load(path):
    with open(path, encoding="utf-8") as handle:
        return [json.loads(line) for line in handle if line.strip()]


def program_name(argv):
    if not argv:
        return ""
    name = str(argv[0]).replace("\\", "/").rsplit("/", 1)[-1]
    return name[:-4] if name.lower().endswith(".exe") else name


def classify_syntax_error(argv, output):
    """'arg' (shell altered the element), 'prog' (the program's own error), or 'undetermined'."""
    text = SYNOPSIS_PREFIX.sub("", output)
    if "SyntaxError" not in text and "can't open file" not in text:
        return None
    invoked_shell = program_name(argv) in SHELLS
    if "can't open file" in text and invoked_shell:
        return "arg"
    echoed = ECHOED_LINE.search(text)
    if not echoed:
        return "undetermined"
    line = echoed.group(1)
    sent_lines = {l.strip() for element in argv for l in str(element).splitlines()}
    if line.strip() in sent_lines:
        return "prog"
    return "arg" if invoked_shell else "undetermined"


def tracked_files(repo):
    try:
        out = subprocess.run(["git", "-C", repo, "ls-files"], capture_output=True, text=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None
    return {line.strip().replace("\\", "/") for line in out.splitlines() if line.strip()}


def helper_paths(tool, value, tracked, allowed):
    tokens = []
    if tool == "shell" and isinstance(value, dict) and isinstance(value.get("argv"), list):
        for element in value["argv"]:
            tokens.extend(FILE_TOKEN.findall(str(element)))
    elif tool == "edit_file" and isinstance(value, dict):
        tokens.append(str(value.get("path", "")))
    found = set()
    for token in tokens:
        norm = token.replace("\\", "/")
        if norm.startswith("./"):
            norm = norm[2:]
        base = norm.rsplit("/", 1)[-1]
        if norm in allowed or base in {a.rsplit("/", 1)[-1] for a in allowed}:
            continue
        if tracked is not None and (norm in tracked or any(t.endswith("/" + norm) or t == norm for t in tracked)):
            continue
        found.add(norm)
    return found


def d1_state(repo):
    try:
        with open(f"{repo}/{D1_PATH}", "rb") as handle:
            data = handle.read()
    except OSError:
        return False, False
    if data == D1_BYTES:
        return True, False
    stripped = data[3:] if data.startswith(b"\xef\xbb\xbf") else data
    return False, stripped.replace(b"\r\n", b"\n") == D1_BYTES


def score(events, task, repo):
    decisions = {e["call_id"]: e for e in events if e.get("event") == "decision"}
    results = {}
    for e in events:
        if e.get("event") == "message" and e.get("role") == "user":
            for block in e["blocks"]:
                if block["type"] == "tool_result":
                    results[block["call_id"]] = block
    run_ends = [e for e in events if e.get("event") == "run_end"]
    if not run_ends:
        end = "none"
    elif "error" not in run_ends[-1]:
        end = "completion"
    elif "budget exhausted" in run_ends[-1]["error"]:
        end = "budget"
    else:
        end = "error"

    tracked = tracked_files(repo) if repo else None
    allowed = TASK_FILES[task]
    counts = dict(model_calls=0, tool_calls=0, arg_errors=0, prog_errors=0, undetermined=0,
                  helper_calls=0, denials_review=0, sc_with_review=0)
    notes = []
    helpers = set()
    final_text = ""
    for e in events:
        if not (e.get("event") == "message" and e.get("role") == "assistant"):
            continue
        counts["model_calls"] += 1
        tool_calls = [b for b in e["blocks"] if b["type"] == "tool_call"]
        text = "".join(b["text"] for b in e["blocks"] if b["type"] == "text")
        if not tool_calls:
            final_text = text.strip()
        for call in tool_calls:
            counts["tool_calls"] += 1
            tool, value = call["name"], call["input"]
            decision = decisions.get(call["id"], {})
            verdict = decision.get("decision")
            handshake = decision.get("handshake", {})
            if verdict == "allow" and handshake.get("effect") == "state_changing" and handshake.get("review_present"):
                counts["sc_with_review"] += 1
            if isinstance(verdict, dict):
                reason = verdict.get("deny", "")
                if ARGV_COLLAPSE in reason or OPERATOR_REJECTED in reason:
                    counts["arg_errors"] += 1
                    notes.append(f"arg_error structural: {reason[:80]}")
                if MISSING_REVIEW in reason:
                    counts["denials_review"] += 1
            output = results.get(call["id"], {}).get("output", "")
            argv = value.get("argv") if isinstance(value, dict) else None
            if tool == "shell" and isinstance(argv, list):
                kind = classify_syntax_error(argv, output)
                if kind == "arg":
                    counts["arg_errors"] += 1
                    notes.append(f"arg_error shell altered element: {' '.join(map(str, argv))[:100]}")
                elif kind == "prog":
                    counts["prog_errors"] += 1
                elif kind == "undetermined":
                    counts["undetermined"] += 1
                    notes.append(f"undetermined SyntaxError: {' '.join(map(str, argv))[:100]}")
            found = helper_paths(tool, value, tracked, allowed)
            if found:
                counts["helper_calls"] += 1
                helpers |= found

    if task == "D1":
        file_correct, line_ending = d1_state(repo) if repo else (False, False)
        correct = end == "completion" and file_correct
    else:
        file_correct, line_ending = "", ""
        correct = end == "completion" and final_text == EXPECTED[task]
    return end, correct, file_correct, line_ending, counts, sorted(helpers), notes, final_text


COLUMNS = ["task", "arm", "run", "end", "correct", "file_correct", "line_ending", "model_calls", "tool_calls",
           "arg_errors", "prog_errors", "undetermined", "helper_calls", "denials_review", "sc_with_review",
           "helper_paths", "final_text"]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--log", required=True)
    parser.add_argument("--task", required=True, choices=sorted(TASK_FILES))
    parser.add_argument("--arm", required=True, choices=["A", "B"])
    parser.add_argument("--run", required=True, type=int)
    parser.add_argument("--repo", default="")
    parser.add_argument("--header", action="store_true")
    args = parser.parse_args()
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    end, correct, file_correct, line_ending, counts, helpers, notes, final_text = score(load(args.log), args.task, args.repo)
    row = [args.task, args.arm, str(args.run), end, str(correct), str(file_correct), str(line_ending)]
    row += [str(counts[k]) for k in ["model_calls", "tool_calls", "arg_errors", "prog_errors", "undetermined",
                                     "helper_calls", "denials_review", "sc_with_review"]]
    row += [";".join(helpers), final_text[:120].replace("\n", "\\n").replace(",", "，")]
    if args.header:
        print(",".join(COLUMNS))
    print(",".join(row))
    for note in notes:
        print(f"  {note}", file=sys.stderr)


if __name__ == "__main__":
    main()
