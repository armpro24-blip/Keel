"""Token usage per model call from a Keel wire capture (local only).

    python usage_from_wire.py <session>.wire.jsonl

Prints only numbers taken from each response's ``usage`` object, so the
output can be shared while the wire file itself stays on the machine.
"""

import json
import sys


def main(path):
    prompt, completion = [], []
    for line in open(path, encoding="utf-8"):
        if not line.strip():
            continue
        event = json.loads(line)
        if event.get("event") != "response":
            continue
        usage = event["body"].get("usage") or {}
        prompt.append(usage.get("prompt_tokens", 0))
        completion.append(usage.get("completion_tokens", 0))
    print("call  prompt  completion")
    for index, (p, c) in enumerate(zip(prompt, completion), start=1):
        print(f"{index:>4}  {p:>6}  {c:>10}")
    if prompt:
        print(f"calls {len(prompt)}  prompt max {max(prompt)}  prompt sum {sum(prompt)}  completion sum {sum(completion)}")


if __name__ == "__main__":
    main(sys.argv[1])
