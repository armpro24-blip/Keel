"""The job record the state machine maintains."""

from dataclasses import dataclass, field
from datetime import datetime
from typing import Optional

#: Job statuses, in the order the summary line lists them.
STATUSES = ("queued", "running", "completed", "failed")


@dataclass
class Job:
    id: str
    status: str = "queued"
    #: Which attempt the job is on; the first attempt is 1.
    attempt: int = 1
    created_at: Optional[datetime] = None
    started_at: Optional[datetime] = None
    finished_at: Optional[datetime] = None
    worker: str = ""
    reason: str = ""
    history: list = field(default_factory=list)

    def is_terminal(self):
        return self.status in ("completed", "failed")
