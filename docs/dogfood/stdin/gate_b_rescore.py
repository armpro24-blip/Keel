"""Gate B re-scoring: does the model's stdin program actually produce the
payload byte for byte?

    python gate_b_rescore.py DIR

For each DIR/response_NN.json, take tool_calls[0].function.arguments.stdin,
run `python -` with exactly those bytes on stdin inside a fresh temporary
directory that contains an empty `tally/` folder, then compare the bytes of
the `tally/cli.py` it wrote with the payload from gate_b.py.

Why: gate_b.py scored "materially complete" by looking for every payload line
verbatim inside the stdin text. The task told the model to send a writer
program, so the payload sits inside a Python string literal and lines with
triple quotes, backslashes, or \\n are legitimately escaped there. Executing
the program measures the thing the gate asked about: whether the file the
model would have written equals the payload.

The programs are model output. They are run in a throwaway directory with a
30 s timeout; the ten seen so far only write tally/cli.py. Inspect
response_NN.json first if that is not acceptable.
"""

import difflib
import json
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gate_b import PAYLOAD  # noqa: E402

EXPECTED = PAYLOAD.encode("utf-8")


def normalized(data):
    return data.replace(b"\r\n", b"\n").rstrip(b"\n")


def main(directory):
    exact = normalized_ok = ran = 0
    exit0_wrong = no_file = nonzero_exit = 0
    rows = []
    for index in range(1, 11):
        path = os.path.join(directory, f"response_{index:02d}.json")
        if not os.path.exists(path):
            continue
        body = json.load(open(path, encoding="utf-8"))
        try:
            arguments = json.loads(body["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"])
            stdin = arguments["stdin"]
            argv = arguments["argv"]
        except (KeyError, IndexError, TypeError, json.JSONDecodeError) as error:
            rows.append(f"run {index:02d}: no usable stdin ({error})")
            continue
        workdir = tempfile.mkdtemp(prefix=f"gate-b-{index:02d}-")
        os.mkdir(os.path.join(workdir, "tally"))
        try:
            result = subprocess.run(
                argv, input=stdin.encode("utf-8"), cwd=workdir, capture_output=True, timeout=30
            )
        except subprocess.TimeoutExpired:
            rows.append(f"run {index:02d}: TIMEOUT")
            continue
        ran += 1
        target = os.path.join(workdir, "tally", "cli.py")
        written = open(target, "rb").read() if os.path.exists(target) else None
        if result.returncode != 0:
            nonzero_exit += 1
        if written is None:
            no_file += 1
            rows.append(
                f"run {index:02d}: exit {result.returncode}; no tally/cli.py written; "
                f"stderr: {result.stderr.decode('utf-8', 'replace').strip().splitlines()[-1:]}"
            )
            continue
        is_exact = written == EXPECTED
        is_norm = normalized(written) == normalized(EXPECTED)
        exact += is_exact
        normalized_ok += is_norm
        if result.returncode == 0 and not is_exact:
            exit0_wrong += 1
        line = f"run {index:02d}: exit {result.returncode}; wrote {len(written)} bytes; exact={is_exact} normalized={is_norm}"
        if not is_norm:
            diff = list(
                difflib.unified_diff(
                    EXPECTED.decode("utf-8").splitlines(),
                    written.decode("utf-8", "replace").splitlines(),
                    "payload",
                    "written",
                    lineterm="",
                    n=0,
                )
            )
            line += "\n    " + "\n    ".join(diff[2:10])
        elif not is_exact:
            line += f" (differs only in line endings or trailing newline: written ends {written[-4:]!r})"
        rows.append(line)
    print("\n".join(rows))
    print(f"\nprograms run {ran}/10; file byte-exact {exact}/10 (gate metric); equal after CRLF/trailing-newline normalization {normalized_ok}/10 (diagnostic only)")
    print(f"exited 0 but wrote incorrect bytes: {exit0_wrong}/10; failed to create tally/cli.py: {no_file}/10; non-zero exit: {nonzero_exit}/10")
    print("A non-zero exit with a SyntaxError, or a diff confined to lines containing quotes, backslashes or braces, points at the model's own escaping layer inside the writer program; judge from the rows above.")


if __name__ == "__main__":
    main(sys.argv[1])
