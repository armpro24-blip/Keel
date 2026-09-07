//! Tests for the SessionLog (PLAN.md §5.9; invariant 4).

mod common;

use common::TempDir;
use keel::agent::{AllowAll, Decision, Hooks};
use keel::log::{message_to_json, read_events, render_event, Recorder, SessionLog};
use keel::message::{Block, Message, Provenance, Role, ToolCall};
use serde_json::json;

#[test]
fn events_are_appended_in_order_with_timestamps_and_read_back() {
    let dir = TempDir::new("log-order");
    let mut log = SessionLog::open(&dir.path, "abc123").unwrap();
    assert!(log.path().ends_with("abc123.jsonl"));

    log.record(json!({ "event": "session_start", "mode": "ask" }));
    log.record(message_to_json(&Message::user_text("hi")));
    log.record(json!({ "event": "session_end" }));
    assert!(log.take_failure().is_none());

    let events = read_events(log.path()).unwrap();
    let kinds: Vec<&str> = events
        .iter()
        .map(|event| event["event"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, vec!["session_start", "message", "session_end"]);
    assert!(events.iter().all(|event| event["t"].as_u64().is_some()));
    assert_eq!(events[1]["blocks"][0]["text"], "hi");
}

#[test]
fn messages_serialize_every_block_kind_and_provenance() {
    let message = Message {
        role: Role::Assistant,
        blocks: vec![
            Block::Text("calling".to_string()),
            Block::ToolCall {
                id: "c1".to_string(),
                name: "shell".to_string(),
                input: json!({ "argv": ["git", "status"] }),
            },
        ],
    };
    let results = Message {
        role: Role::User,
        blocks: vec![
            Block::ToolResult {
                call_id: "c1".to_string(),
                output: "[exit 0]".to_string(),
                is_error: false,
                provenance: Provenance::Observation,
            },
            Block::ToolResult {
                call_id: "c2".to_string(),
                output: "# CODING_STYLE".to_string(),
                is_error: false,
                provenance: Provenance::PiraPolicy {
                    source: "~/agent/modules/CODING_STYLE.md".to_string(),
                },
            },
        ],
    };

    let call = message_to_json(&message);
    assert_eq!(call["role"], "assistant");
    assert_eq!(
        call["blocks"][0],
        json!({ "type": "text", "text": "calling" })
    );
    assert_eq!(call["blocks"][1]["name"], "shell");
    assert_eq!(call["blocks"][1]["input"]["argv"][0], "git");

    let observed = message_to_json(&results);
    assert_eq!(observed["blocks"][0]["provenance"], "observation");
    assert_eq!(
        observed["blocks"][1]["provenance"],
        json!({ "pira_policy": "~/agent/modules/CODING_STYLE.md" })
    );

    let rendered = render_event(&observed);
    assert!(
        rendered.contains("tool_result c1 (error=false): [exit 0]"),
        "{rendered}"
    );
    assert!(
        rendered.contains("policy=~/agent/modules/CODING_STYLE.md"),
        "{rendered}"
    );
}

/// Denies everything, to show the log records denials without execution.
struct DenyAll;

impl Hooks for DenyAll {
    fn decide(&mut self, _call: &ToolCall) -> Decision {
        Decision::Deny("test".to_string())
    }
}

#[test]
fn recorder_logs_messages_and_decisions_in_order_and_passes_decisions_through() {
    let dir = TempDir::new("log-gate");
    let mut log = SessionLog::open(&dir.path, "gate").unwrap();
    let call = ToolCall {
        id: "c1".to_string(),
        name: "shell".to_string(),
        input: json!({
            "argv": ["cmd", "/C", "echo hi > f"],
            "intent": "i",
            "effect": "state_changing",
            "safety_review": "Writes f; reversible."
        }),
    };

    let mut allow = AllowAll;
    let mut deny = DenyAll;
    let allowed = {
        let mut recorder = Recorder::new(&mut allow, &mut log);
        recorder.on_message(&Message::user_text("go"));
        recorder.decide(&call)
    };
    let denied = Recorder::new(&mut deny, &mut log).decide(&call);

    assert_eq!(allowed, Decision::Allow);
    assert_eq!(denied, Decision::Deny("test".to_string()));
    let events = read_events(log.path()).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["event"], "message");
    assert_eq!(events[1]["event"], "decision");
    assert_eq!(events[1]["tool"], "shell");
    assert_eq!(events[1]["decision"], "allow");
    assert_eq!(events[2]["decision"], json!({ "deny": "test" }));
    assert!(render_event(&events[2]).starts_with("[decision] c1 shell -> "));
    assert!(
        render_event(&events[1]).ends_with(
            r#" handshake={"effect":"state_changing","review_present":true,"review_source":"model","review_validated":"presence_only"}"#
        ),
        "{}",
        render_event(&events[1])
    );
    assert_eq!(
        events[1]["handshake"],
        json!({
            "effect": "state_changing",
            "review_present": true,
            "review_source": "model",
            "review_validated": "presence_only"
        })
    );
}

#[test]
fn a_malformed_line_is_an_error_not_skipped() {
    let dir = TempDir::new("log-malformed");
    let path = dir.path.join("bad.jsonl");
    std::fs::write(&path, "{\"event\":\"ok\"}\nnot json\n").unwrap();

    let error = read_events(&path).unwrap_err();

    assert!(error.contains("line 2"), "{error}");
}
