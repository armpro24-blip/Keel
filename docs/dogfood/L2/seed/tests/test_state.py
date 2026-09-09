import unittest
from datetime import datetime

from queuewatch.events import Event
from queuewatch.state import TransitionError, apply, build_jobs

T0 = datetime(2026, 9, 1, 8, 0, 0)


def ev(line, job, kind, seconds=0, **extra):
    return Event(line, T0.replace(second=seconds), job, kind, **extra)


def replay(*events):
    return build_jobs(list(events))


class LifecycleTests(unittest.TestCase):
    def test_created_makes_a_queued_job_on_attempt_one(self):
        job = apply(None, ev(1, "a", "created"))
        self.assertEqual((job.id, job.status, job.attempt), ("a", "queued", 1))
        self.assertEqual(job.created_at, T0)
        self.assertEqual(job.history, ["created"])

    def test_started_records_worker_and_time(self):
        job = replay(ev(1, "a", "created"), ev(2, "a", "started", 5, worker="w1"))["a"]
        self.assertEqual(job.status, "running")
        self.assertEqual(job.worker, "w1")
        self.assertEqual(job.started_at, T0.replace(second=5))
        self.assertIsNone(job.finished_at)

    def test_completed_finishes_a_running_job(self):
        job = replay(ev(1, "a", "created"), ev(2, "a", "started", 5), ev(3, "a", "completed", 9))["a"]
        self.assertEqual(job.status, "completed")
        self.assertEqual(job.finished_at, T0.replace(second=9))
        self.assertTrue(job.is_terminal())

    def test_failed_records_the_reason(self):
        job = replay(
            ev(1, "a", "created"), ev(2, "a", "started", 5), ev(3, "a", "failed", 9, reason="oom")
        )["a"]
        self.assertEqual((job.status, job.reason), ("failed", "oom"))
        self.assertTrue(job.is_terminal())

    def test_history_lists_every_applied_event(self):
        job = replay(ev(1, "a", "created"), ev(2, "a", "started", 1), ev(3, "a", "completed", 2))["a"]
        self.assertEqual(job.history, ["created", "started", "completed"])


class InvalidTransitionTests(unittest.TestCase):
    def assert_rejected(self, events, message):
        with self.assertRaises(TransitionError) as caught:
            replay(*events)
        self.assertEqual(str(caught.exception), message)

    def test_event_before_created(self):
        self.assert_rejected([ev(1, "a", "started")], "line 1: job a: 'started' before 'created'")

    def test_created_twice(self):
        self.assert_rejected(
            [ev(1, "a", "created"), ev(2, "a", "created")],
            "line 2: job a: cannot apply 'created' to a job that is queued",
        )

    def test_completed_without_started(self):
        self.assert_rejected(
            [ev(1, "a", "created"), ev(2, "a", "completed")],
            "line 2: job a: cannot apply 'completed' to a job that is queued",
        )

    def test_started_twice(self):
        self.assert_rejected(
            [ev(1, "a", "created"), ev(2, "a", "started"), ev(3, "a", "started")],
            "line 3: job a: cannot apply 'started' to a job that is running",
        )

    def test_events_after_a_terminal_status(self):
        self.assert_rejected(
            [ev(1, "a", "created"), ev(2, "a", "started"), ev(3, "a", "failed"), ev(4, "a", "started")],
            "line 4: job a: cannot apply 'started' to a job that is failed",
        )
        self.assert_rejected(
            [ev(1, "a", "created"), ev(2, "a", "started"), ev(3, "a", "completed"), ev(4, "a", "completed")],
            "line 4: job a: cannot apply 'completed' to a job that is completed",
        )


class BuildJobsTests(unittest.TestCase):
    def test_jobs_are_kept_in_first_seen_order_and_interleave(self):
        jobs = replay(
            ev(1, "b", "created"),
            ev(2, "a", "created"),
            ev(3, "b", "started", 1),
            ev(4, "a", "started", 2),
            ev(5, "b", "completed", 3),
        )
        self.assertEqual(list(jobs), ["b", "a"])
        self.assertEqual(jobs["b"].status, "completed")
        self.assertEqual(jobs["a"].status, "running")

    def test_empty_log_gives_no_jobs(self):
        self.assertEqual(replay(), {})


if __name__ == "__main__":
    unittest.main()
