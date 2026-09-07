//! Tests for the PermissionEngine: approval paths and the pre-execution
//! handshake (PLAN.md §5.6; invariants 5, 7, and the handshake invariants).

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::TempDir;
use keel::agent::{Decision, Hooks};
use keel::message::ToolCall;
use keel::permission::{ApprovalMode, Approver, PermissionEngine};
use keel::workspace::Workspace;
use serde_json::{json, Value};

/// Records every prompt and announcement; answers prompts with a fixed verdict.
#[derive(Default)]
struct Surface {
    prompts: Vec<String>,
    announcements: Vec<String>,
}

struct Recorder {
    surface: Rc<RefCell<Surface>>,
    verdict: bool,
}

impl Approver for Recorder {
    fn approve(&mut self, summary: &str) -> bool {
        self.surface.borrow_mut().prompts.push(summary.to_string());
        self.verdict
    }

    fn announce(&mut self, text: &str) {
        self.surface
            .borrow_mut()
            .announcements
            .push(text.to_string());
    }
}

fn engine(
    mode: ApprovalMode,
    root: &std::path::Path,
    verdict: bool,
) -> (PermissionEngine, Rc<RefCell<Surface>>) {
    let surface = Rc::new(RefCell::new(Surface::default()));
    let engine = PermissionEngine::new(
        mode,
        Workspace::at(root),
        vec!["read_pira_policy".to_string()],
        Box::new(Recorder {
            surface: Rc::clone(&surface),
            verdict,
        }),
    );
    (engine, surface)
}

fn call(name: &str, input: Value) -> ToolCall {
    ToolCall {
        id: "c".to_string(),
        name: name.to_string(),
        input,
    }
}

fn shell_call(effect: &str, review: Option<&str>, workdir: Option<&str>) -> ToolCall {
    let mut input = json!({
        "argv": ["cmd", "/C", "echo hello > f.txt"],
        "intent": "Create f.txt",
        "effect": effect
    });
    if let Some(review) = review {
        input["safety_review"] = json!(review);
    }
    if let Some(workdir) = workdir {
        input["workdir"] = json!(workdir);
    }
    call("shell", input)
}

fn outside_dir() -> &'static str {
    if cfg!(windows) {
        "C:\\Windows"
    } else {
        "/usr"
    }
}

#[test]
fn policy_loading_never_asks_or_announces_in_any_mode() {
    let dir = TempDir::new("perm-policy");
    for mode in [ApprovalMode::Ask, ApprovalMode::Full] {
        let (mut engine, surface) = engine(mode, &dir.path, false);
        let decision = engine.decide(&call("read_pira_policy", json!({ "name": "coding" })));
        assert_eq!(decision, Decision::Allow);
        assert!(surface.borrow().prompts.is_empty());
        assert!(surface.borrow().announcements.is_empty());
    }
}

#[test]
fn ask_mode_asks_for_every_action_and_honors_the_answer() {
    let dir = TempDir::new("perm-ask");
    let (mut allowing, surface) = engine(ApprovalMode::Ask, &dir.path, true);
    assert_eq!(
        allowing.decide(&shell_call("state_changing", Some("Writes f.txt."), None)),
        Decision::Allow
    );
    let prompt = surface.borrow().prompts[0].clone();
    assert!(
        prompt.starts_with("pira_ctx --intent \"Create f.txt\" -- cmd /C \"echo hello > f.txt\""),
        "{prompt}"
    );
    assert!(prompt.contains("\n  effect: state_changing"), "{prompt}");
    assert!(prompt.contains("\n  Safety: Writes f.txt."), "{prompt}");
    assert!(
        surface.borrow().announcements.is_empty(),
        "ask mode shows the review in the prompt, not as an announcement"
    );

    let (mut denying, _) = engine(ApprovalMode::Ask, &dir.path, false);
    match denying.decide(&shell_call("read_only", None, None)) {
        Decision::Deny(reason) => assert!(reason.contains("declined"), "{reason}"),
        other => panic!("expected Deny, got {other:?}"),
    }
}

#[test]
fn ask_mode_does_not_require_a_review_for_a_state_changing_command() {
    // Host approval is the guarantee on this path; PIRA's review rule is for
    // the no-approval path, and Keel must not strengthen it silently.
    let dir = TempDir::new("perm-ask-noreview");
    let (mut engine, surface) = engine(ApprovalMode::Ask, &dir.path, true);

    let decision = engine.decide(&shell_call("state_changing", None, None));

    assert_eq!(decision, Decision::Allow);
    let prompt = surface.borrow().prompts[0].clone();
    assert!(prompt.contains("effect: state_changing"), "{prompt}");
    assert!(!prompt.contains("Safety:"), "{prompt}");
}

#[test]
fn full_mode_runs_read_only_commands_without_asking_or_announcing() {
    let dir = TempDir::new("perm-full-read");
    let (mut engine, surface) = engine(ApprovalMode::Full, &dir.path, false);

    assert_eq!(
        engine.decide(&shell_call("read_only", None, None)),
        Decision::Allow
    );
    assert_eq!(
        engine.decide(&shell_call(
            "read_only",
            Some("unneeded review"),
            Some("src")
        )),
        Decision::Allow
    );
    assert!(surface.borrow().prompts.is_empty());
    assert!(surface.borrow().announcements.is_empty());
}

#[test]
fn full_mode_announces_the_review_before_allowing_a_state_changing_command() {
    let dir = TempDir::new("perm-full-write");
    let (mut engine, surface) = engine(ApprovalMode::Full, &dir.path, false);

    let decision = engine.decide(&shell_call(
        "state_changing",
        Some("  Creates f.txt in the workspace; reversible by deletion.  "),
        None,
    ));

    assert_eq!(decision, Decision::Allow);
    assert_eq!(
        surface.borrow().announcements,
        vec!["Safety: Creates f.txt in the workspace; reversible by deletion.".to_string()]
    );
    assert!(surface.borrow().prompts.is_empty());
}

#[test]
fn full_mode_refuses_a_state_changing_command_without_a_review() {
    let dir = TempDir::new("perm-full-noreview");
    let (mut engine, surface) = engine(ApprovalMode::Full, &dir.path, false);

    for review in [None, Some("   ")] {
        match engine.decide(&shell_call("state_changing", review, None)) {
            Decision::Deny(reason) => {
                assert!(reason.contains("safety_review"), "{reason}");
                assert!(reason.contains("Full-Permission Behavior"), "{reason}");
            }
            other => panic!("expected Deny, got {other:?}"),
        }
    }
    assert!(
        surface.borrow().announcements.is_empty(),
        "nothing is announced for a refused call"
    );
    assert!(surface.borrow().prompts.is_empty());
}

#[test]
fn a_workdir_outside_the_workspace_asks_even_in_full_mode_and_needs_no_review() {
    let dir = TempDir::new("perm-outside");
    let (mut declining, surface) = engine(ApprovalMode::Full, &dir.path, false);

    let decision = declining.decide(&shell_call("state_changing", None, Some(outside_dir())));

    assert!(matches!(decision, Decision::Deny(_)), "{decision:?}");
    let prompt = surface.borrow().prompts[0].clone();
    assert!(prompt.contains("(outside the workspace)"), "{prompt}");
    assert!(prompt.contains("effect: state_changing"), "{prompt}");
    assert!(surface.borrow().announcements.is_empty());

    let (mut approving, surface) = engine(ApprovalMode::Full, &dir.path, true);
    approving.decide(&shell_call(
        "state_changing",
        Some("Lists a directory."),
        Some(outside_dir()),
    ));
    assert!(surface.borrow().prompts[0].contains("Safety: Lists a directory."));
}

#[test]
fn malformed_shell_input_is_denied_with_the_parser_message_and_surfaces_no_review() {
    // `Allow` means "may execute"; a request that cannot be parsed never
    // gets it. The parser's message is the denial reason, so the model sees
    // `not executed: <message>` and no review is shown for a call that
    // could not run.
    let dir = TempDir::new("perm-malformed");
    for mode in [ApprovalMode::Full, ApprovalMode::Ask] {
        let (mut engine, surface) = engine(mode, &dir.path, true);

        let missing_effect = engine.decide(&call(
            "shell",
            json!({ "argv": ["cmd", "/C", "echo hi > f"], "intent": "i", "safety_review": "Writes f." }),
        ));
        let string_argv = engine.decide(&call(
            "shell",
            json!({ "argv": "echo hi", "intent": "i", "effect": "read_only" }),
        ));

        match missing_effect {
            Decision::Deny(reason) => assert!(reason.contains("'effect'"), "{reason}"),
            other => panic!("expected Deny, got {other:?}"),
        }
        match string_argv {
            Decision::Deny(reason) => assert!(reason.contains("'argv'"), "{reason}"),
            other => panic!("expected Deny, got {other:?}"),
        }
        assert!(surface.borrow().prompts.is_empty());
        assert!(surface.borrow().announcements.is_empty());
    }
}
