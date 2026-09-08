# Gate A raw output: pira_ctx stdin passthrough (local, 2026-09-08)

Machine: the author's Windows machine (not the lab). `pira_ctx 1.8.0`,
Python 3.14.0. Script: `docs/dogfood/stdin/gate_a.py`, run from the Keel
checkout; the workspace for the runs is a fresh temp directory the script
created. Output verbatim.

```text
workdir: C:\Users\RL_Carla\AppData\Local\Temp\keel-gate-a-j33273yy

=== A1 wrapped, exact bytes as specified (no trailing newline) ===
argv: ['pira_ctx', '--intent', 'Verify stdin passthrough', '--', 'python', '-']
stdin bytes: b'print("KEEL_STDIN_OK")'
exit: 0
stdout: 'Captured: 20260908-073134-06b7ad504439 (exit 0):\nWarning: display-control characters in PROGRAM output were sanitized.\nPROGRAM data:\nL1 stdout: KEEL_STDIN_OK \n'
stderr: ''

=== A2 wrapped, same script with a trailing newline ===
argv: ['pira_ctx', '--intent', 'Verify stdin passthrough with newline', '--', 'python', '-']
stdin bytes: b'print("KEEL_STDIN_OK")\n'
exit: 0
stdout: 'Captured: 20260908-073134-0256c09c5cbb (exit 0):\nWarning: display-control characters in PROGRAM output were sanitized.\nPROGRAM data:\nL1 stdout: KEEL_STDIN_OK \n'
stderr: ''

=== A3 control: python - directly, no pira_ctx ===
argv: ['python', '-']
stdin bytes: b'print("KEEL_STDIN_OK")'
exit: 0
stdout: 'KEEL_STDIN_OK\r\n'
stderr: ''

=== A4 wrapped, multi-line UTF-8 script that writes a file and reports its bytes ===
argv: ['pira_ctx', '--intent', 'Verify multi-line UTF-8 stdin through pira_ctx', '--', 'python', '-']
stdin bytes: b"import sys\nbody = 'caf\xc3\xa9 \xe2\x80\x94 \xe4\xb8\xad\xe6\x96\x87\\n'\nwith open('gate_a_out.txt', 'w', encoding='utf-8', newline='') as f:\n    f.write(body)\ndata = open('gate_a_out.txt', 'rb').read()\nprint('KEEL_STDIN_OK', len(data), data.hex())\n"
exit: 0
stdout: 'Captured: 20260908-073135-c9b5449ef5c0 (exit 0):\nWarning: display-control characters in PROGRAM output were sanitized.\nPROGRAM data:\nL1 stdout: KEEL_STDIN_OK 17 636166c3a920e2809420e4b8ade696870a \n'
stderr: ''

=== A5 wrapped, child that ignores stdin (must not hang) ===
argv: ['pira_ctx', '--intent', 'Verify a child that ignores stdin', '--', 'python', '-c', "print('ignored stdin')"]
stdin bytes: b'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx... (200000 bytes total)'
exit: 0
stdout: 'Captured: 20260908-073135-f750d8ad1771 (exit 0):\nWarning: display-control characters in PROGRAM output were sanitized.\nPROGRAM data:\nL1 stdout: ignored stdin \n'
stderr: ''

=== pira_ctx history for this workspace ===
history_hits=4 scanned=4 shown=4 complete=1 skipped=0 omitted_scopes=0 scope=current-thread offset=0 lookback=all since_ms=none until_ms=none order=newest-first
age | exit | result | intent
0s | 0 | 20260908-073135-f750d8ad1771 | Verify a child that ignores stdin
0s | 0 | 20260908-073135-c9b5449ef5c0 | Verify multi-line UTF-8 stdin through pira_ctx
0s | 0 | 20260908-073134-0256c09c5cbb | Verify stdin passthrough with newline
0s | 0 | 20260908-073134-06b7ad504439 | Verify stdin passthrough

gate_a_out.txt bytes: 636166c3a920e2809420e4b8ade696870a
```

Reading: A4's reported hex is the UTF-8 encoding of `café — 中文\n`
(`63 61 66 c3a9 20 e28094 20 e4b8ad e69687 0a`, 17 bytes), so the script
reached the child unchanged through `pira_ctx`. Pass criterion (`stdout`
contains `KEEL_STDIN_OK`, exit 0) met in A1, A2, A4; A3 is the unwrapped
control; A5 shows no hang when the child never reads stdin. Lab
confirmation on `pira_ctx 1.9.0` pending.
