# L2 frozen task text

Sent to the model as the first and only task message, exactly as below (the
REPL reads one line per message, so the operator joins the paragraphs with
single spaces, content unchanged, as in L1).

```text
queuewatch currently handles created, started, completed and failed job events. Production logs can now contain retry events (`"event": "retry"`), and `python -m queuewatch report data/production_2026-09.jsonl` fails on them.

Add retry support. A retry is valid only for a job whose current status is failed: it increments the job's attempt count and returns the job to queued for a new attempt, so the job can be started and then completed or failed again. A retry in any other status is an invalid transition and must be reported the same way existing invalid transitions are: a normal error message naming the line and the job, a non-zero exit, and no traceback.

Also add a `--status STATUS` option to the `report` command that limits the per-job rows to jobs whose current status is STATUS (one of queued, running, completed, failed). The summary line must still count all jobs. An unknown STATUS must be a usage error. Without `--status`, report output must be unchanged.

Update the README for both changes and add tests for them. All existing tests must keep passing and must not be removed or renamed. Verify with `python -m unittest discover -s tests -v`. Finish with a short summary of what you changed and the test result.
```
