# Shell-tool argument transport: first data point (Keel author's machine, 2026-09-10)

Protocol: `docs/dogfood/shell_contract/SHELL_CONTRACT_AB.md`, stage 1.
This is the author's own run, not the lab's: Windows 11, PIRA `pira_ctx`
1.8.0, Python 3.14, Keel working tree at the T25 commit (runtime unchanged
since `822fdbc`). The lab run on the experiment machine (PIRA 1.9.0,
Python 3.12.10) is still required before stage 2 can be considered.

## Result: the transport does not change legal arguments

```text
$ cargo test --test shell_transport 2>&1 | grep 'test result'
test result: ok. 0 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.00s

$ cargo test --test shell_transport -- --ignored --nocapture
running 5 tests
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

| Check | Outcome |
|---|---|
| 7-element payload (space, embedded `"`, backslash, apostrophe, `=` with space, empty string, trailing backslash), exact mode | received byte for byte |
| same payload, default mode through the `pira_ctx` synopsis | received byte for byte; the synopsis showed the whole line |
| `-c` program with both quote kinds, a raw backslash, and a newline | executed as written |
| `def` after `;` on one line | Python's own `SyntaxError` at `def job`: the argument arrived intact, the program was invalid |
| explicit `cmd /C` with the program as one element containing `"` | `cmd` re-parsed the element; Python saw `"import` (the exact failure of L2-R2 calls 39, 54, 92) |

Reading: on this machine the Keel → `pira_ctx` → child path is faithful for
every payload tried, so the stop rule of stage 1 is not triggered here. The
two failure shapes seen in L2-R2 are reproduced with their causes: an
explicit shell applying its own quoting to an element, and invalid one-line
Python. Neither is a transport defect. Whether the same holds with PIRA
1.9.0 and Python 3.12 on the lab machine is the lab's stage-1 run.

One test-writing note: the first version of the two payload tests compared
the child's `json.dumps` text against serde's compact serialization and
failed on spacing alone while the data was identical; the tests now compare
parsed arrays. Recorded so the failure is not mistaken for a transport
finding.
