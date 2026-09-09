# queuewatch

Reconstruct the state of batch jobs from an event log and report on it.

## Event log

A JSON Lines file, one object per line, in time order:

```json
{"ts": "2026-09-01T08:00:05", "job": "build-api", "event": "started", "worker": "w1"}
```

Fields: `ts` (`YYYY-MM-DDTHH:MM:SS`), `job` (identifier), `event` (kind),
and optionally `worker` (on `started`) and `reason` (on `failed`).

Event kinds and the transitions they are valid for:

```text
(no job) --created--> queued --started--> running --completed--> completed
                                             \--failed-----> failed
```

A job's `attempt` is 1 from creation. Any event that does not fit the job's
current status, an unknown event kind, or a malformed line stops the run
with a message naming the line and the job; nothing is guessed.

## Usage

```text
python -m queuewatch report FILE
python -m queuewatch check FILE
```

`report` prints one row per job, sorted by job id, then a summary line that
counts every job by status:

```text
$ python -m queuewatch report data/events.jsonl
job        status     attempt  duration
build-api  completed  1        0:03:12
build-web  failed     1        0:03:35
deploy     queued     1        -
docs       running    1        -
lint       completed  1        0:00:39
test-e2e   failed     1        0:03:38
test-unit  completed  1        0:04:30
total 7  queued 1  running 1  completed 3  failed 2
```

`duration` is the time from the job's `started` to its `completed` or
`failed` event; `-` when the job has not finished.

`check` validates the log and prints `ok: N events, M jobs`.

Exit codes: 0 on success; 1 for a data error (unreadable file, malformed
line, invalid transition), reported on stderr as `queuewatch: <message>`;
2 for a usage error.

## Development

```text
python -m unittest discover -s tests -v
```

Standard library only.
