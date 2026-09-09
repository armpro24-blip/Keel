"""audit_tool_protocol: structured facts about tool-call channel use in a Keel
wire capture. Read-only; prints structure, counts, and booleans, never the
text of reasoning or content.

    python audit_tool_protocol.py <session>.wire.jsonl [--call N]

For every response: index, finish_reason, number of tool_calls, content
length, reasoning length, and whether the reasoning text contains tool-call
markup (`<tool_call>` / `</tool_call>` counts, `<function=...>` names,
`<parameter=...>` names, any `<think>`/`</think>` marker). Then a summary of
how often tool-call markup appears in reasoning together with, or without,
extracted tool_calls, so the call-40 case can be compared with the rest of
the session (and with the L1 session, which had three empty turns).

With --call N it also checks response N against the last denied edit_file
in the preceding request: whether the reasoning contains that call's path,
and the first 40 characters of its old_text and new_text (booleans), and
whether the markup in the reasoning forms a complete <tool_call> block
(opening and closing tag counts equal and at least one function name).

The Qwen3/Hermes-style markup is assumed (`<tool_call>`, `<function=NAME>`,
`<parameter=NAME>`); other shapes are reported as "no known markup".
"""

import argparse
import json
import re

TOOL_OPEN = re.compile(r"<tool_call>")
TOOL_CLOSE = re.compile(r"</tool_call>")
FUNCTION = re.compile(r"<function=([A-Za-z0-9_\-]+)>")
PARAMETER = re.compile(r"<parameter=([A-Za-z0-9_\-]+)>")
THINK = re.compile(r"</?think>")
JSON_TOOL_CALL = re.compile(r'"name"\s*:\s*"(shell|edit_file|read_pira_policy)"')


def load(path):
    requests, responses = [], []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            event = json.loads(line)
            if event.get("event") == "request":
                requests.append(event["body"])
            elif event.get("event") == "response":
                responses.append(event["body"])
            elif event.get("event") == "response_error":
                responses.append({"error": event.get("error")})
    return requests, responses


def reasoning_of(message):
    return message.get("reasoning") or message.get("reasoning_content") or ""


def facts(index, body):
    if "error" in body:
        return {"call": index, "error": body["error"]}
    choice = body["choices"][0]
    message = choice["message"]
    reasoning = reasoning_of(message)
    content = message.get("content") or ""
    calls = message.get("tool_calls") or []
    return {
        "call": index,
        "finish": choice.get("finish_reason"),
        "tool_calls": len(calls),
        "tool_names": [c["function"]["name"] for c in calls],
        "content_chars": len(content),
        "reasoning_chars": len(reasoning),
        "r_tool_call_open": len(TOOL_OPEN.findall(reasoning)),
        "r_tool_call_close": len(TOOL_CLOSE.findall(reasoning)),
        "r_function_names": FUNCTION.findall(reasoning),
        "r_parameter_names": PARAMETER.findall(reasoning),
        "r_json_tool_names": JSON_TOOL_CALL.findall(reasoning),
        "r_think_markers": len(THINK.findall(reasoning)),
        "c_tool_call_tags": len(TOOL_OPEN.findall(content)) + len(TOOL_CLOSE.findall(content)),
        "c_think_markers": len(THINK.findall(content)),
        "completion_tokens": (body.get("usage") or {}).get("completion_tokens"),
    }


def last_denied_edit(request_body):
    """The most recent edit_file call whose tool result says 'not executed' in the request's messages."""
    messages = request_body.get("messages", [])
    denied = {}
    for message in messages:
        if message.get("role") == "tool" and str(message.get("content", "")).startswith("[tool error] not executed"):
            denied[message.get("tool_call_id")] = message["content"]
    last = None
    for message in messages:
        for call in message.get("tool_calls") or []:
            if call["id"] in denied and call["function"]["name"] == "edit_file":
                try:
                    last = json.loads(call["function"]["arguments"])
                except json.JSONDecodeError:
                    last = None
    return last


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("wire")
    parser.add_argument("--call", type=int)
    args = parser.parse_args()
    requests, responses = load(args.wire)
    rows = [facts(i, body) for i, body in enumerate(responses, start=1)]

    print(f"requests {len(requests)}  responses {len(responses)}")
    print("call finish      calls content reasoning r<tc> r</tc> r<function> r<parameter> think(r/c) compl")
    for r in rows:
        if "error" in r:
            print(f"{r['call']:>4} ERROR {r['error'][:60]}")
            continue
        print(
            f"{r['call']:>4} {str(r['finish']):<11} {r['tool_calls']:>5} {r['content_chars']:>7} {r['reasoning_chars']:>9} "
            f"{r['r_tool_call_open']:>5} {r['r_tool_call_close']:>6} {len(r['r_function_names']):>11} {len(r['r_parameter_names']):>12} "
            f"{r['r_think_markers']}/{r['c_think_markers']:<8} {str(r['completion_tokens']):>5}"
        )

    ok = [r for r in rows if "error" not in r]
    markup_in_reasoning = [r for r in ok if r["r_tool_call_open"] or r["r_tool_call_close"] or r["r_function_names"]]
    print()
    print(f"responses with tool-call markup in reasoning: {len(markup_in_reasoning)}/{len(ok)}")
    print(f"  ...and extracted tool_calls > 0: {sum(1 for r in markup_in_reasoning if r['tool_calls'])}")
    print(f"  ...and extracted tool_calls = 0: {sum(1 for r in markup_in_reasoning if not r['tool_calls'])}")
    empty = [r for r in ok if r["tool_calls"] == 0 and r["content_chars"] == 0]
    print(f"empty responses (no tool_calls, no content): {[r['call'] for r in empty]}")
    for r in empty:
        complete = r["r_tool_call_open"] == r["r_tool_call_close"] and r["r_tool_call_open"] >= 1 and r["r_function_names"]
        print(f"  call {r['call']}: markup open/close {r['r_tool_call_open']}/{r['r_tool_call_close']}, functions {r['r_function_names']}, parameters {r['r_parameter_names']}, complete-block={bool(complete)}")

    if args.call:
        n = args.call
        body = responses[n - 1]
        r = rows[n - 1]
        print()
        print(f"--- call {n} against the last denied edit_file in request {n} ---")
        denied = last_denied_edit(requests[n - 1]) if n - 1 < len(requests) else None
        if denied is None:
            print("no denied edit_file found in the preceding request")
        else:
            reasoning = reasoning_of(body["choices"][0]["message"])
            print(f"denied call path: {denied.get('path')}")
            print(f"reasoning contains that path: {denied.get('path', '') in reasoning}")
            print(f"reasoning contains old_text[:40]: {denied.get('old_text', '')[:40] in reasoning}")
            print(f"reasoning contains new_text[:40]: {denied.get('new_text', '')[:40] in reasoning}")
            print(f"reasoning mentions 'safety_review': {'safety_review' in reasoning}")
        print(f"markup: open {r['r_tool_call_open']} close {r['r_tool_call_close']} functions {r['r_function_names']} parameters {r['r_parameter_names']} json-names {r['r_json_tool_names']}")
        tail = reasoning_of(body["choices"][0]["message"])[-120:]
        print(f"reasoning tail markup only: {''.join(re.findall(r'</?(?:tool_call|function|parameter|think)[^>]*>', tail))}")


if __name__ == "__main__":
    main()
