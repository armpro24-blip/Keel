//! Deterministic tests for the agent loop (PLAN.md §7 M0, §9 invariants 1–2).

use keel::agent::{AgentLoop, LoopError};
use keel::message::{Block, Message, Provenance, Role};
use keel::model::{FakeModel, ModelError};
use keel::tool::{DuplicateToolName, EchoTool, ToolRegistry};
use serde_json::{json, Value};

fn tool_call(id: &str, name: &str, input: Value) -> Block {
    Block::ToolCall {
        id: id.to_string(),
        name: name.to_string(),
        input,
    }
}

fn assistant_calls(blocks: Vec<Block>) -> Message {
    Message {
        role: Role::Assistant,
        blocks,
    }
}

fn agent(max_turns: usize) -> AgentLoop {
    AgentLoop {
        system: "test system".to_string(),
        max_turns,
    }
}

fn echo_registry() -> ToolRegistry {
    let mut tools = ToolRegistry::new();
    tools.register(Box::new(EchoTool)).unwrap();
    tools
}

#[test]
fn tool_round_trip_reaches_final_answer() {
    let mut model = FakeModel::new(vec![
        assistant_calls(vec![tool_call("call-1", "echo", json!({"text": "hello"}))]),
        Message::assistant_text("done"),
    ]);
    let mut tools = echo_registry();
    let mut transcript = Vec::new();

    let outcome = agent(4)
        .run(&mut model, &mut tools, &mut transcript, "hi")
        .unwrap();

    assert_eq!(outcome.final_text, "done");
    assert_eq!(outcome.turns, 2);
    // user input, assistant call, tool results, assistant final answer
    assert_eq!(transcript.len(), 4);

    // The observation reached the model on its second call.
    assert_eq!(model.seen.len(), 2);
    let observed = model.seen[1].last().unwrap();
    assert_eq!(observed.role, Role::User);
    assert_eq!(
        observed.blocks,
        vec![Block::ToolResult {
            call_id: "call-1".to_string(),
            output: json!({"text": "hello"}).to_string(),
            is_error: false,
            provenance: Provenance::Observation,
        }]
    );
}

#[test]
fn direct_answer_needs_no_tools() {
    let mut model = FakeModel::new(vec![Message::assistant_text("42")]);
    let mut tools = echo_registry();
    let mut transcript = Vec::new();

    let outcome = agent(1)
        .run(&mut model, &mut tools, &mut transcript, "answer")
        .unwrap();

    assert_eq!(outcome.final_text, "42");
    assert_eq!(outcome.turns, 1);
    assert_eq!(transcript.len(), 2);
}

#[test]
fn transcript_carries_over_between_runs() {
    let mut model = FakeModel::new(vec![
        Message::assistant_text("first"),
        Message::assistant_text("second"),
    ]);
    let mut tools = echo_registry();
    let mut transcript = Vec::new();
    let agent = agent(1);

    agent
        .run(&mut model, &mut tools, &mut transcript, "one")
        .unwrap();
    agent
        .run(&mut model, &mut tools, &mut transcript, "two")
        .unwrap();

    assert_eq!(transcript.len(), 4);
    // The second call saw the whole first exchange plus the new input.
    assert_eq!(model.seen[1].len(), 3);
    assert_eq!(model.seen[1][0].text(), "one");
    assert_eq!(model.seen[1][1].text(), "first");
    assert_eq!(model.seen[1][2].text(), "two");
}

#[test]
fn max_turns_is_an_explicit_fuse() {
    let endless_call = || assistant_calls(vec![tool_call("c", "echo", json!({}))]);
    let mut model = FakeModel::new(vec![endless_call(), endless_call(), endless_call()]);
    let mut tools = echo_registry();
    let mut transcript = Vec::new();

    let error = agent(2)
        .run(&mut model, &mut tools, &mut transcript, "loop")
        .unwrap_err();

    assert_eq!(error, LoopError::MaxTurnsExceeded { max_turns: 2 });
    // user input + 2 × (assistant call, tool results) stay with the caller
    assert_eq!(transcript.len(), 5);
    assert_eq!(model.seen.len(), 2, "the fuse stops further model calls");
}

#[test]
fn unknown_tool_becomes_an_error_observation() {
    let mut model = FakeModel::new(vec![
        assistant_calls(vec![tool_call("call-1", "nope", json!({}))]),
        Message::assistant_text("recovered"),
    ]);
    let mut tools = echo_registry();
    let mut transcript = Vec::new();

    let outcome = agent(3)
        .run(&mut model, &mut tools, &mut transcript, "go")
        .unwrap();

    assert_eq!(outcome.final_text, "recovered");
    match &transcript[2].blocks[0] {
        Block::ToolResult {
            call_id,
            output,
            is_error,
            ..
        } => {
            assert_eq!(call_id, "call-1");
            assert!(*is_error);
            assert!(output.contains("unknown tool: nope"), "got {output}");
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

#[test]
fn multiple_calls_in_one_turn_return_results_in_declaration_order() {
    let mut model = FakeModel::new(vec![
        assistant_calls(vec![
            tool_call("a", "echo", json!({"n": 1})),
            tool_call("b", "echo", json!({"n": 2})),
            tool_call("c", "echo", json!({"n": 3})),
        ]),
        Message::assistant_text("ok"),
    ]);
    let mut tools = echo_registry();
    let mut transcript = Vec::new();

    agent(2)
        .run(&mut model, &mut tools, &mut transcript, "batch")
        .unwrap();

    let results = &transcript[2].blocks;
    let ids: Vec<&str> = results
        .iter()
        .map(|block| match block {
            Block::ToolResult { call_id, .. } => call_id.as_str(),
            other => panic!("expected ToolResult, got {other:?}"),
        })
        .collect();
    assert_eq!(ids, vec!["a", "b", "c"]);

    let outputs: Vec<&str> = results
        .iter()
        .map(|block| match block {
            Block::ToolResult { output, .. } => output.as_str(),
            other => panic!("expected ToolResult, got {other:?}"),
        })
        .collect();
    assert_eq!(outputs, vec![r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#]);
}

#[test]
fn exhausted_script_is_a_model_error() {
    let mut model = FakeModel::new(vec![]);
    let mut tools = echo_registry();
    let mut transcript = Vec::new();

    let error = agent(1)
        .run(&mut model, &mut tools, &mut transcript, "x")
        .unwrap_err();

    assert_eq!(error, LoopError::Model(ModelError::ScriptExhausted));
    assert_eq!(transcript.len(), 1, "the user input stays with the caller");
}

#[test]
fn duplicate_tool_names_are_rejected() {
    let mut tools = echo_registry();

    let error = tools.register(Box::new(EchoTool)).unwrap_err();

    assert_eq!(error, DuplicateToolName("echo".to_string()));
    assert_eq!(tools.specs().len(), 1);
}
