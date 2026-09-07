//! Tests for the PermissionEngine (PLAN.md §5.6; invariants 5, 7).

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::TempDir;
use keel::agent::{Decision, ToolGate};
use keel::message::ToolCall;
use keel::permission::{ApprovalMode, Approver, PermissionEngine};
use keel::workspace::Workspace;
use serde_json::json;

/// Records every prompt and answers with a fixed verdict.
struct Recorder {
    prompts: Rc<RefCell<Vec<String>>>,
    verdict: bool,
}

impl Approver for Recorder {
    fn approve(&mut self, summary: &str) -> bool {
        self.prompts.borrow_mut().push(summary.to_string());
        self.verdict
    }
}

fn engine(
    mode: ApprovalMode,
    root: &std::path::Path,
    verdict: bool,
) -> (PermissionEngine, Rc<RefCell<Vec<String>>>) {
    let prompts = Rc::new(RefCell::new(Vec::new()));
    let engine = PermissionEngine::new(
        mode,
        Workspace::at(root),
        vec!["read_pira_policy".to_string()],
        Box::new(Recorder {
            prompts: Rc::clone(&prompts),
            verdict,
        }),
    );
    (engine, prompts)
}

fn call(name: &str, input: serde_json::Value) -> ToolCall {
    ToolCall {
        id: "c".to_string(),
        name: name.to_string(),
        input,
    }
}

fn shell_call(workdir: Option<&str>) -> ToolCall {
    let mut input = json!({ "argv": ["git", "status"], "intent": "Inspect status" });
    if let Some(workdir) = workdir {
        input["workdir"] = json!(workdir);
    }
    call("shell", input)
}

#[test]
fn policy_loading_never_asks_in_any_mode() {
    let dir = TempDir::new("perm-policy");
    for mode in [ApprovalMode::Ask, ApprovalMode::Full] {
        let (mut engine, prompts) = engine(mode, &dir.path, false);
        let decision = engine.decide(&call("read_pira_policy", json!({ "name": "coding" })));
        assert_eq!(decision, Decision::Allow);
        assert!(prompts.borrow().is_empty());
    }
}

#[test]
fn ask_mode_asks_for_every_action_and_honors_the_answer() {
    let dir = TempDir::new("perm-ask");
    let (mut allowing, prompts) = engine(ApprovalMode::Ask, &dir.path, true);
    assert_eq!(allowing.decide(&shell_call(None)), Decision::Allow);
    let summary = prompts.borrow()[0].clone();
    assert!(
        summary.starts_with("pira_ctx --intent Inspect status -- git status"),
        "{summary}"
    );
    assert!(summary.contains("\n  in "), "{summary}");

    let (mut denying, _) = engine(ApprovalMode::Ask, &dir.path, false);
    match denying.decide(&shell_call(None)) {
        Decision::Deny(reason) => assert!(reason.contains("declined"), "{reason}"),
        other => panic!("expected Deny, got {other:?}"),
    }
}

#[test]
fn full_mode_allows_actions_inside_the_workspace_without_asking() {
    let dir = TempDir::new("perm-full");
    let (mut engine, prompts) = engine(ApprovalMode::Full, &dir.path, false);

    assert_eq!(engine.decide(&shell_call(None)), Decision::Allow);
    assert_eq!(engine.decide(&shell_call(Some("src"))), Decision::Allow);
    assert_eq!(
        engine.decide(&call("other_tool", json!({}))),
        Decision::Allow
    );
    assert!(prompts.borrow().is_empty());
}

#[test]
fn a_workdir_outside_the_workspace_asks_even_in_full_mode() {
    let dir = TempDir::new("perm-outside");
    let (mut engine, prompts) = engine(ApprovalMode::Full, &dir.path, false);
    let outside = if cfg!(windows) { "C:\\Windows" } else { "/usr" };

    let decision = engine.decide(&shell_call(Some(outside)));

    assert!(matches!(decision, Decision::Deny(_)), "{decision:?}");
    let prompt = prompts.borrow()[0].clone();
    assert!(prompt.contains("(outside the workspace)"), "{prompt}");
}

#[test]
fn malformed_shell_input_is_left_for_the_tool_to_report() {
    let dir = TempDir::new("perm-malformed");
    let (mut engine, prompts) = engine(ApprovalMode::Ask, &dir.path, false);

    let decision = engine.decide(&call("shell", json!({ "argv": [] })));

    assert_eq!(decision, Decision::Allow);
    assert!(prompts.borrow().is_empty());
}
