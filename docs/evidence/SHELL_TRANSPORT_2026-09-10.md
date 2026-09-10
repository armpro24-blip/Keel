# Shell-tool argument transport: stage 1 of T25 (2026-09-10)

Protocol: `docs/dogfood/shell_contract/SHELL_CONTRACT_AB.md`, stage 1. Two
runs of the same ignored tests, no model involved: the Keel author's machine
first, then the lab's experiment machine. Lab delivery stored in
`docs/dogfood/shell_contract/stage1_2026-09-10/`.

## Verdict

**The transport is not at fault.** On both machines every legal argument
arrived unchanged (7-element payload with a space, embedded `"`, a
backslash, an apostrophe, `=` with a space, an empty string, a trailing
backslash; in `exact` and default `pira_ctx` mode; and a `-c` program with
both quote kinds, a raw backslash, and a newline). The stop rule is not
triggered; stage 2 may go to review.

**The four `SyntaxError` calls of the L2-R2 loop (68, 74, 77, 80) are all
invalid one-line Python, none is shell re-parsing.** Each was a direct
`["python", "-c", <program>]` with no shell; in 74, 77, 80 the traceback
echoes the whole program byte for byte as sent, with `^^^` under a `try` or
`def` that follows `;`; in 68 the outer program arrived intact and the error
is inside its own `exec()` string (`try: … except SystemExit: pass` on one
line). The shell re-parsing class in L2-R2 is calls 39, 54, 92 (`cmd /C
python -c "…"`), whose shape the fifth test reproduces: `cmd` re-parses the
element and Python sees `"import` as an unterminated string.

## Lab run (experiment machine: Keel `4f512a0`, runtime `822fdbc`, pira_ctx 1.9.0, Python 3.12.10)

```text
$ cargo test --test shell_transport 2>&1 | grep 'test result'
test result: ok. 0 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`shell_transport.txt` (verbatim in the delivery directory):

```text
running 5 tests
compound-statement observation:
[stderr]
  File "<string>", line 1
    from datetime import datetime; T=datetime(2026,9,1); def job(n): return n
                                                         ^^^
SyntaxError: invalid syntax
[exit 1]
test compound_statement_after_semicolon_is_pythons_error_not_transports ... ok
program observation:
a b c"d e\f
second line
[exit 0]
test program_text_with_quotes_and_backslashes_arrives_unchanged ... ok
exact mode observation:
["a b", "c\"d\"e", "e\\f", "it's", "--flag=x y", "", "trailing\\"]
[exit 0]
test payload_arrives_unchanged_in_exact_mode ... ok
default mode observation:
Captured: 20260910-193354-3a1929beca3d (exit 0):
Warning: display-control characters in PROGRAM output were sanitized.
PROGRAM data:
L1 stdout: ["a b", "c\"d\"e", "e\\f", "it's", "--flag=x y", "", "trailing\\"] 
[exit 0]
test payload_arrives_unchanged_in_default_mode ... ok
cmd /C observation (is_error=true):
[stderr]
  File "<string>", line 1
    "import
    ^
SyntaxError: unterminated string literal (detected at line 1)
[exit 1]
test cmd_slash_c_reparses_the_element_under_cmd_rules ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

Line for line the same observations as the author's run below; only the
`pira_ctx` result ID and the timing differ.

### Classification of calls 68, 74, 77, 80 (lab; source `l2r2_calls_68_80.txt`)

| Call | Form | Class | Basis (the traceback line) |
|---|---|---|---|
| 68 | `python -c …`, no shell, default mode | invalid one-line Python | the error is inside the program's own `exec()` string: `try: main(list([…])) except SystemExit: pass` with `^^^^^^` under `except`; the outer program reached Python intact (`File "<string>", line 1, in <module>` precedes the inner `File "<string>", line 1`) |
| 74 | `[exact] python -c …`, no shell | invalid one-line Python | the echoed line equals the sent program; `^^^` at column 205, the `try` after `;` |
| 77 | `[exact] python -c …`, no shell | invalid one-line Python | echoed line equals the sent program; `^^^` at column 145, the `def` after `;` |
| 80 | `[exact] python -c …`, no shell | invalid one-line Python | echoed line equals the sent program; `^^^` at column 138, the `def` after `;` |

The protocol's "calls 68 and 80 (and probably 74, 77)" becomes definite:
four of four. Side observation from the lab, not affecting the class: call
68's program also calls `__enter__` on a tuple and would have failed after a
syntax fix.

### Lab's script observation, resolved

The lab noticed `repeated identical calls 20` in this ranged run against `5`
in the T24 delivery and asked whether the range filter changes the count. It
does not: the summary is always computed over the whole log. The difference
is the `repeat` key change recorded in the T24 evidence (the T24 trace was
produced with the old key that included the intent prose; the lab's checkout
at `4f512a0` compares the command only), so 20 is the full-run count under
the new key. The self-check now asserts that a ranged run reports the same
repeat count as a full run. `helper-related calls 0` is because `--helpers`
was not passed for this excerpt.

## Author's run (Windows 11, pira_ctx 1.8.0, Python 3.14; first data point)

```text
$ cargo test --test shell_transport -- --ignored --nocapture
program observation:
a b c"d e\f
second line
[exit 0]
test program_text_with_quotes_and_backslashes_arrives_unchanged ... ok
default mode observation:
Captured: 20260910-181841-1e12fbf7ff48 (exit 0):
Warning: display-control characters in PROGRAM output were sanitized.
PROGRAM data:
L1 stdout: ["a b", "c\"d\"e", "e\\f", "it's", "--flag=x y", "", "trailing\\"] 
[exit 0]
test payload_arrives_unchanged_in_default_mode ... ok
exact mode observation:
["a b", "c\"d\"e", "e\\f", "it's", "--flag=x y", "", "trailing\\"]
[exit 0]
test payload_arrives_unchanged_in_exact_mode ... ok
cmd /C observation (is_error=true):
[stderr]
  File "<string>", line 1
    "import
    ^
SyntaxError: unterminated string literal (detected at line 1)
[exit 1]
test cmd_slash_c_reparses_the_element_under_cmd_rules ... ok
compound-statement observation:
[stderr]
  File "<string>", line 1
    from datetime import datetime; T=datetime(2026,9,1); def job(n): return n
                                                         ^^^
SyntaxError: invalid syntax
[exit 1]
test compound_statement_after_semicolon_is_pythons_error_not_transports ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
```

Test-writing note: the first version of the two payload tests compared the
child's `json.dumps` text against serde's compact serialization and failed
on spacing alone while the data was identical; the tests compare parsed
arrays. Recorded so that failure is not mistaken for a transport finding.

## What this settles for stage 2

- The description experiment tests a description, not a transport bug:
  the mechanical channel is faithful on two PIRA versions and two Pythons.
- "Shell re-parsing" and "invalid one-line Python" are distinguishable
  mechanically by whether argv invoked a shell (`cmd`, `powershell`, `sh`,
  `bash`); the amended stage-2 protocol scores them that way, the second
  class never counting against either arm.
- The lab's review comments on the stage-2 draft (R2's dependence on
  `.gitattributes` and `init_l2.sh`, D1's LF/BOM wording, gate rounding,
  per-task reporting, the redirection stance in B) are taken into the
  amended draft in `SHELL_CONTRACT_AB.md`, which still awaits the user's
  approval before anything runs.
