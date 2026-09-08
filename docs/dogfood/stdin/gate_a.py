"""Gate A: does pira_ctx forward stdin to the wrapped child unchanged?

    python gate_a.py [WORKDIR]

Runs each case with subprocess.run (no shell, no quoting), feeding exact
bytes to stdin, and prints argv, the stdin bytes, stdout, stderr, and the
exit code verbatim. WORKDIR defaults to a fresh temporary directory so the
pira_ctx history entries land in a throwaway workspace. Nothing else is
changed.
"""

import os
import subprocess
import sys
import tempfile

CASES = [
    (
        "A1 wrapped, exact bytes as specified (no trailing newline)",
        ["pira_ctx", "--intent", "Verify stdin passthrough", "--", "python", "-"],
        b'print("KEEL_STDIN_OK")',
    ),
    (
        "A2 wrapped, same script with a trailing newline",
        ["pira_ctx", "--intent", "Verify stdin passthrough with newline", "--", "python", "-"],
        b'print("KEEL_STDIN_OK")\n',
    ),
    (
        "A3 control: python - directly, no pira_ctx",
        ["python", "-"],
        b'print("KEEL_STDIN_OK")',
    ),
    (
        "A4 wrapped, multi-line UTF-8 script that writes a file and reports its bytes",
        ["pira_ctx", "--intent", "Verify multi-line UTF-8 stdin through pira_ctx", "--", "python", "-"],
        (
            "import sys\n"
            "body = 'caf\u00e9 \u2014 \u4e2d\u6587\\n'\n"
            "with open('gate_a_out.txt', 'w', encoding='utf-8', newline='') as f:\n"
            "    f.write(body)\n"
            "data = open('gate_a_out.txt', 'rb').read()\n"
            "print('KEEL_STDIN_OK', len(data), data.hex())\n"
        ).encode("utf-8"),
    ),
    (
        "A5 wrapped, child that ignores stdin (must not hang)",
        ["pira_ctx", "--intent", "Verify a child that ignores stdin", "--", "python", "-c", "print('ignored stdin')"],
        b"x" * 200_000,
    ),
]


def main():
    workdir = sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp(prefix="keel-gate-a-")
    print(f"workdir: {workdir}")
    for title, argv, stdin in CASES:
        print(f"\n=== {title} ===")
        print(f"argv: {argv}")
        shown = stdin if len(stdin) <= 400 else stdin[:40] + b"... (" + str(len(stdin)).encode() + b" bytes total)"
        print(f"stdin bytes: {shown!r}")
        try:
            result = subprocess.run(
                argv, input=stdin, cwd=workdir, capture_output=True, timeout=60
            )
        except subprocess.TimeoutExpired as error:
            print(f"TIMEOUT after 60 s; partial stdout={error.stdout!r} stderr={error.stderr!r}")
            continue
        print(f"exit: {result.returncode}")
        print(f"stdout: {result.stdout.decode('utf-8', 'replace')!r}")
        print(f"stderr: {result.stderr.decode('utf-8', 'replace')!r}")
    print("\n=== pira_ctx history for this workspace ===")
    history = subprocess.run(
        ["pira_ctx", "history", "--limit", "10"], cwd=workdir, capture_output=True, text=True
    )
    print(history.stdout)
    if history.stderr:
        print(f"stderr: {history.stderr}")
    out = os.path.join(workdir, "gate_a_out.txt")
    if os.path.exists(out):
        print(f"gate_a_out.txt bytes: {open(out, 'rb').read().hex()}")


if __name__ == "__main__":
    main()
