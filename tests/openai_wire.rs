//! Wire-mapping tests for the OpenAI Chat Completions adapter. No network.

use keel::message::{Block, Message, Role, ToolSpec};
use keel::model::ModelError;
use keel::openai::{from_wire, to_wire};
use serde_json::json;

fn echo_spec() -> ToolSpec {
    ToolSpec {
        name: "echo".to_string(),
        description: "Returns the input.".to_string(),
        input_schema: json!({ "type": "object" }),
    }
}

#[test]
fn request_maps_system_user_assistant_and_tool_results() {
    let transcript = vec![
        Message::user_text("hi"),
        Message {
            role: Role::Assistant,
            blocks: vec![
                Block::Text("calling".to_string()),
                Block::ToolCall {
                    id: "call-1".to_string(),
                    name: "echo".to_string(),
                    input: json!({"n": 1}),
                },
            ],
        },
        Message {
            role: Role::User,
            blocks: vec![Block::ToolResult {
                call_id: "call-1".to_string(),
                output: "{\"n\":1}".to_string(),
                is_error: false,
            }],
        },
    ];

    let body = to_wire("test-model", "sys", &transcript, &[echo_spec()]).unwrap();

    assert_eq!(
        body,
        json!({
            "model": "test-model",
            "messages": [
                { "role": "system", "content": "sys" },
                { "role": "user", "content": "hi" },
                {
                    "role": "assistant",
                    "content": "calling",
                    "tool_calls": [{
                        "id": "call-1",
                        "type": "function",
                        "function": { "name": "echo", "arguments": "{\"n\":1}" }
                    }]
                },
                { "role": "tool", "tool_call_id": "call-1", "content": "{\"n\":1}" }
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "echo",
                    "description": "Returns the input.",
                    "parameters": { "type": "object" }
                }
            }]
        })
    );
}

#[test]
fn request_omits_tools_when_none_and_nulls_empty_assistant_content() {
    let transcript = vec![Message {
        role: Role::Assistant,
        blocks: vec![Block::ToolCall {
            id: "c".to_string(),
            name: "echo".to_string(),
            input: json!({}),
        }],
    }];

    let body = to_wire("m", "sys", &transcript, &[]).unwrap();

    assert!(body.get("tools").is_none());
    assert_eq!(body["messages"][1]["content"], json!(null));
}

#[test]
fn request_marks_error_results_in_content() {
    let transcript = vec![Message {
        role: Role::User,
        blocks: vec![Block::ToolResult {
            call_id: "c".to_string(),
            output: "boom".to_string(),
            is_error: true,
        }],
    }];

    let body = to_wire("m", "sys", &transcript, &[]).unwrap();

    assert_eq!(body["messages"][1]["content"], json!("[tool error] boom"));
}

#[test]
fn response_with_text_only_is_a_final_answer() {
    let body = json!({
        "choices": [{ "message": { "role": "assistant", "content": "hello" } }]
    });

    let message = from_wire(&body).unwrap();

    assert_eq!(message, Message::assistant_text("hello"));
}

#[test]
fn response_tool_calls_parse_arguments_into_json() {
    let body = json!({
        "choices": [{ "message": {
            "role": "assistant",
            "content": null,
            "tool_calls": [
                { "id": "call-1", "type": "function",
                  "function": { "name": "echo", "arguments": "{\"text\":\"hi\"}" } },
                { "id": "call-2", "type": "function",
                  "function": { "name": "echo", "arguments": "" } }
            ]
        } }]
    });

    let message = from_wire(&body).unwrap();

    assert_eq!(message.role, Role::Assistant);
    assert_eq!(
        message.blocks,
        vec![
            Block::ToolCall {
                id: "call-1".to_string(),
                name: "echo".to_string(),
                input: json!({"text": "hi"}),
            },
            Block::ToolCall {
                id: "call-2".to_string(),
                name: "echo".to_string(),
                input: json!({}),
            },
        ]
    );
}

#[test]
fn response_with_malformed_arguments_is_a_visible_error() {
    let body = json!({
        "choices": [{ "message": {
            "role": "assistant",
            "tool_calls": [{ "id": "c", "type": "function",
                             "function": { "name": "echo", "arguments": "{not json" } }]
        } }]
    });

    let error = from_wire(&body).unwrap_err();

    match error {
        ModelError::Provider(detail) => assert!(detail.contains("malformed arguments"), "{detail}"),
        other => panic!("expected Provider error, got {other:?}"),
    }
}

#[test]
fn api_error_body_becomes_provider_error() {
    let body = json!({ "error": { "message": "Incorrect API key provided", "type": "invalid_request_error" } });

    let error = from_wire(&body).unwrap_err();

    assert_eq!(
        error,
        ModelError::Provider("Incorrect API key provided".to_string())
    );
}
