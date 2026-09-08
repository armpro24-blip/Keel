"""full_mode_check: handshake invariants of a full-mode session, from the
SessionLog and the visible transcript.

    python full_mode_check.py <session>.jsonl <transcript.txt>

Read-only. For every shell and edit_file decision it reports whether the
call was state-changing (edit_file by contract; shell by the model's
declared effect), whether a review was present, the decision, and whether
the decision event precedes the tool_result of the same call_id in the log.
It counts `Safety:` announcements and `approve?` prompts in the transcript
(at line start, or directly after the REPL's `> ` prompt), denials
for a missing review, and recoveries (a later allowed call to the same tool
with the same path or argv after such a denial). Ordering between the
announcement and execution is by construction of the loop (decide, which
announces, runs before execute) and is pinned by tests; the log shows the
decision → result order per call.
"""

import json
import re
import sys


def main(log_path, transcript_path):
    events = [json.loads(l) for l in open(log_path, encoding="utf-8") if l.strip()]
    transcript = open(transcript_path, encoding="utf-8", errors="replace").read()

    decisions = {}  # call_id -> (index, event)
    results = {}  # call_id -> index
    for index, event in enumerate(events):
        if event.get("event") == "decision":
            decisions[event["call_id"]] = (index, event)
        elif event.get("event") == "message":
            for block in event.get("blocks", []):
                if block.get("type") == "tool_result":
                    results.setdefault(block["call_id"], index)

    rows = []
    state_changing = denied_missing_review = allowed_with_review = ordering_violations = 0
    outside = 0
    denials = []
    for call_id, (index, event) in decisions.items():
        tool = event["tool"]
        handshake = event.get("handshake") or {}
        decision = event["decision"]
        allowed = decision == "allow"
        reason = "" if allowed else decision.get("deny", "")
        sc = tool == "edit_file" or handshake.get("effect") == "state_changing"
        review = handshake.get("review_present")
        result_index = results.get(call_id)
        ordered = result_index is not None and index < result_index
        if not ordered:
            ordering_violations += 1
        if sc:
            state_changing += 1
        if "safety_review" in reason:
            denied_missing_review += 1
            denials.append((call_id, tool, event.get("input")))
        if sc and allowed and review:
            allowed_with_review += 1
        if sc and allowed and not review:
            rows.append(f"!! {call_id} {tool}: allowed state-changing call WITHOUT review (handshake={handshake})")
        rows.append(
            f"{call_id} {tool:<9} sc={str(sc):<5} review={str(review):<5} {'allow' if allowed else 'deny'} "
            f"decision_idx={index} result_idx={result_index} ordered={ordered}"
            + (f"  reason={reason[:70]}" if reason else "")
        )

    # Recovery: after a denial for a missing review, a later allowed call to the same tool
    # with the same path (edit_file) or argv (shell).
    recoveries = 0
    for call_id, tool, inp in denials:
        denied_index = decisions[call_id][0]
        key = (inp or {}).get("path") if tool == "edit_file" else json.dumps((inp or {}).get("argv"))
        for other_id, (other_index, other) in decisions.items():
            if other_index <= denied_index or other["tool"] != tool or other["decision"] != "allow":
                continue
            other_key = (other.get("input") or {}).get("path") if tool == "edit_file" else json.dumps((other.get("input") or {}).get("argv"))
            if other_key == key:
                recoveries += 1
                break

    # An announcement or prompt printed right after the REPL prompt shares its
    # line with the "> " marker (L1-R2 transcript line 13), so both forms count.
    safety_lines = len(re.findall(r"^(?:> )?Safety: ", transcript, flags=re.M))
    approvals = len(re.findall(r"^(?:> )?approve\? ", transcript, flags=re.M))
    outside = transcript.count("(outside the workspace)")

    print("\n".join(rows))
    print()
    print(f"decisions {len(decisions)}  state-changing proposed {state_changing}")
    print(f"allowed state-changing with review {allowed_with_review}")
    print(f"denied for missing review {denied_missing_review}  recovered {recoveries}")
    print(f"Safety: announcements in transcript {safety_lines}  (expected == allowed state-changing with review in full mode)")
    print(f"approve? prompts in transcript {approvals}  outside-workspace markers {outside}")
    print(f"decision->result ordering violations {ordering_violations}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
