"""The job state machine: apply events to jobs.

```text
(absent) --created--> queued --started--> running --completed--> completed
                                             \\--failed-----> failed
```

Every other combination is a ``TransitionError``. The error carries the
event's line number and job so the CLI can report it as data trouble, not a
crash.
"""

from .model import Job


class TransitionError(ValueError):
    """An event that is not valid for the job's current status."""


def _reject(event, job_status):
    raise TransitionError(
        f"line {event.line}: job {event.job}: cannot apply {event.kind!r} "
        f"to a job that is {job_status}"
    )


def apply(job, event):
    """Apply ``event`` to ``job`` (``None`` when the job does not exist yet).

    Returns the job, which is mutated in place when it already exists.
    """
    if event.kind == "created":
        if job is not None:
            _reject(event, job.status)
        job = Job(id=event.job, created_at=event.ts)
        job.history.append(event.kind)
        return job

    if job is None:
        raise TransitionError(
            f"line {event.line}: job {event.job}: {event.kind!r} before 'created'"
        )

    if event.kind == "started":
        if job.status != "queued":
            _reject(event, job.status)
        job.status = "running"
        job.started_at = event.ts
        job.finished_at = None
        job.worker = event.worker
    elif event.kind == "completed":
        if job.status != "running":
            _reject(event, job.status)
        job.status = "completed"
        job.finished_at = event.ts
    elif event.kind == "failed":
        if job.status != "running":
            _reject(event, job.status)
        job.status = "failed"
        job.finished_at = event.ts
        job.reason = event.reason
    else:
        _reject(event, job.status)
    job.history.append(event.kind)
    return job


def build_jobs(events):
    """Replay ``events`` in order; returns ``{job_id: Job}`` in first-seen order."""
    jobs = {}
    for event in events:
        jobs[event.job] = apply(jobs.get(event.job), event)
    return jobs
