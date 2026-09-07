//! Deterministic tests for the agent loop (PLAN.md §7 M0, §9 invariants 1–2).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use keel::agent::{AgentLoop, AllowAll, Decision, Hooks, LoopError};
use keel::message::{Block, Message, Provenance, Role, ToolCall, ToolSpec};
use keel::model::{FakeModel, ModelError};
use keel::tool::{DuplicateToolName, EchoTool, Tool, ToolRegistry, ToolResult};
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
        .run(&mut model, &mut tools, &mut AllowAll, &mut transcript, "hi")
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
        .run(
            &mut model,
            &mut tools,
            &mut AllowAll,
            &mut transcript,
            "answer",
        )
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
        .run(
            &mut model,
            &mut tools,
            &mut AllowAll,
            &mut transcript,
            "one",
        )
        .unwrap();
    agent
        .run(
            &mut model,
            &mut tools,
            &mut AllowAll,
            &mut transcript,
            "two",
        )
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
        .run(
            &mut model,
            &mut tools,
            &mut AllowAll,
            &mut transcript,
            "loop",
        )
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
        .run(&mut model, &mut tools, &mut AllowAll, &mut transcript, "go")
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
        .run(
            &mut model,
            &mut tools,
            &mut AllowAll,
            &mut transcript,
            "batch",
        )
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
        .run(&mut model, &mut tools, &mut AllowAll, &mut transcript, "x")
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

/// Denies everything and remembers what it was asked about.
struct DenyAll {
    asked: Vec<String>,
}

impl Hooks for DenyAll {
    fn decide(&mut self, call: &ToolCall) -> Decision {
        self.asked.push(call.name.clone());
        Decision::Deny("test policy".to_string())
    }
}

/// Counts executions through a shared cell so a test can prove a denied call
/// never ran.
struct Counting {
    executions: Rc<Cell<usize>>,
}

impl Tool for Counting {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "counting".to_string(),
            description: String::new(),
            input_schema: json!({ "type": "object" }),
        }
    }

    fn execute(&mut self, _input: &Value) -> ToolResult {
        self.executions.set(self.executions.get() + 1);
        ToolResult::ok("ran")
    }
}

#[test]
fn a_denied_call_is_not_executed_and_becomes_an_error_observation() {
    let mut model = FakeModel::new(vec![
        assistant_calls(vec![tool_call("call-1", "counting", json!({}))]),
        Message::assistant_text("understood"),
    ]);
    let executions = Rc::new(Cell::new(0));
    let mut tools = ToolRegistry::new();
    tools
        .register(Box::new(Counting {
            executions: Rc::clone(&executions),
        }))
        .unwrap();
    let mut gate = DenyAll { asked: Vec::new() };
    let mut transcript = Vec::new();

    let outcome = agent(3)
        .run(&mut model, &mut tools, &mut gate, &mut transcript, "go")
        .unwrap();

    assert_eq!(outcome.final_text, "understood");
    assert_eq!(gate.asked, vec!["counting"]);
    assert_eq!(executions.get(), 0, "a denied call must never execute");
    match &transcript[2].blocks[0] {
        Block::ToolResult {
            output, is_error, ..
        } => {
            assert!(*is_error);
            assert!(output.contains("not executed: test policy"), "{output}");
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

/// Records the role of every message the loop reports, in order.
struct Order {
    roles: Vec<Role>,
    decisions: usize,
}

impl Hooks for Order {
    fn decide(&mut self, _call: &ToolCall) -> Decision {
        self.decisions += 1;
        Decision::Allow
    }

    fn on_message(&mut self, message: &Message) {
        self.roles.push(message.role);
    }
}

#[test]
fn hooks_see_every_message_in_the_order_it_joins_the_transcript() {
    let mut model = FakeModel::new(vec![
        assistant_calls(vec![tool_call("c", "echo", json!({}))]),
        Message::assistant_text("done"),
    ]);
    let mut tools = echo_registry();
    let mut order = Order {
        roles: Vec::new(),
        decisions: 0,
    };
    let mut transcript = Vec::new();

    agent(3)
        .run(&mut model, &mut tools, &mut order, &mut transcript, "go")
        .unwrap();

    // user input, assistant call, tool results, assistant answer
    assert_eq!(
        order.roles,
        vec![Role::User, Role::Assistant, Role::User, Role::Assistant]
    );
    assert_eq!(order.decisions, 1);
    assert_eq!(order.roles.len(), transcript.len());
}

/// Hooks and a tool that write to one shared event list, so a test can see
/// the exact interleaving of gate decisions and executions.
struct Trail(Rc<RefCell<Vec<String>>>);

impl Hooks for Trail {
    fn decide(&mut self, call: &ToolCall) -> Decision {
        self.0
            .borrow_mut()
            .push(format!("decide {}", call.input["tag"]));
        Decision::Allow
    }
}

struct TrailTool(Rc<RefCell<Vec<String>>>);

impl Tool for TrailTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "trail".to_string(),
            description: String::new(),
            input_schema: json!({ "type": "object" }),
        }
    }

    fn execute(&mut self, input: &Value) -> ToolResult {
        self.0
            .borrow_mut()
            .push(format!("execute {}", input["tag"]));
        ToolResult::ok("ran")
    }
}

#[test]
fn each_call_is_decided_immediately_before_it_executes() {
    // Anything a gate surfaces for call `b` must appear after call `a` has
    // run, not before it: the loop decides and executes one call at a time,
    // in declaration order.
    let trail = Rc::new(RefCell::new(Vec::new()));
    let mut model = FakeModel::new(vec![
        assistant_calls(vec![
            tool_call("a", "trail", json!({ "tag": "a" })),
            tool_call("b", "trail", json!({ "tag": "b" })),
        ]),
        Message::assistant_text("done"),
    ]);
    let mut tools = ToolRegistry::new();
    tools
        .register(Box::new(TrailTool(Rc::clone(&trail))))
        .unwrap();
    let mut hooks = Trail(Rc::clone(&trail));
    let mut transcript = Vec::new();

    agent(3)
        .run(&mut model, &mut tools, &mut hooks, &mut transcript, "go")
        .unwrap();

    assert_eq!(
        *trail.borrow(),
        vec![
            "decide \"a\"".to_string(),
            "execute \"a\"".to_string(),
            "decide \"b\"".to_string(),
            "execute \"b\"".to_string(),
        ]
    );
}
