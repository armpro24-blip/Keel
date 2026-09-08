//! The PermissionEngine: host approval and the pre-execution handshake
//! (PLAN.md §5.6, §3).
//!
//! PIRA states the principles: never run destructive commands without
//! permission, and in full-permission/no-approval mode print a review before
//! any state-changing command. This engine decides whether a tool call
//! executes, and guarantees one thing about that review: a command the model
//! declares state-changing, on a path where no host approval would otherwise
//! happen, never runs without a non-empty model-provided review artifact, and
//! that artifact is visible before execution.
//!
//! Ownership stays split. The model classifies the effect and writes the
//! review; Keel checks neither the classification nor the review's content.
//! Trusted PIRA policy loading is read-only and never asks. A working
//! directory outside the workspace asks in every mode, because PIRA makes the
//! workspace the default allowed scope with the platform temp directory as the
//! only standing exception.

use crate::agent::{Decision, Hooks};
use crate::edit::{self, EditFileRequest};
use crate::message::ToolCall;
use crate::shell::{self, Effect, ShellRequest};
use crate::workspace::{PathScope, Workspace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalMode {
    /// Every action needs the user's confirmation.
    Ask,
    /// In-scope actions run without asking. A command the model declares
    /// state-changing runs only after the model's review has been shown.
    /// Keel provides no sandbox.
    Full,
}

impl ApprovalMode {
    /// The wording the host block gives the model (PLAN.md §4.3-1).
    pub fn describe(self) -> &'static str {
        match self {
            ApprovalMode::Ask => {
                "ask (the user confirms each action before it runs; a safety_review you supply is shown to the user; loading PIRA policy needs no confirmation)"
            }
            ApprovalMode::Full => {
                "full-permission/no-approval mode (the host asks for no ordinary approval and provides no sandbox; a command you declare state_changing runs only after the safety_review you supply has been shown, and the host checks that review's presence and order, not its content or your classification)"
            }
        }
    }
}

/// The host's two surfaces toward the person at the terminal. Both are
/// required: a host that could forget to implement `announce` would make the
/// visibility-before-execution guarantee false.
pub trait Approver {
    /// `summary` describes what would run and where. `true` allows it.
    fn approve(&mut self, summary: &str) -> bool;

    /// Show `text` before the corresponding action executes.
    fn announce(&mut self, text: &str);
}

pub struct PermissionEngine {
    mode: ApprovalMode,
    workspace: Workspace,
    /// Tools whose calls are read-only and never need approval.
    read_only: Vec<String>,
    approver: Box<dyn Approver>,
}

impl PermissionEngine {
    pub fn new(
        mode: ApprovalMode,
        workspace: Workspace,
        read_only: Vec<String>,
        approver: Box<dyn Approver>,
    ) -> PermissionEngine {
        PermissionEngine {
            mode,
            workspace,
            read_only,
            approver,
        }
    }

    pub fn mode(&self) -> ApprovalMode {
        self.mode
    }

    fn ask(&mut self, summary: &str) -> Decision {
        if self.approver.approve(summary) {
            Decision::Allow
        } else {
            Decision::Deny("the user declined this action".to_string())
        }
    }

    /// Order (PLAN.md §5.6): structural validity → which permission path
    /// applies → handshake requirement on that path → surface or ask →
    /// execute. A structurally invalid request is denied with the parser's
    /// message: it surfaces no review, enters no approval, and never reaches
    /// the tool. `Allow` therefore has one meaning: this call may execute.
    fn decide_shell(&mut self, call: &ToolCall) -> Decision {
        // `ShellRequest::parse` is the one validation authority; the engine
        // only relays its result (the tool parses again as its own defense).
        let request = match ShellRequest::parse(&call.input) {
            Ok(request) => request,
            Err(message) => return Decision::Deny(message),
        };
        let workdir = shell::resolve_workdir(self.workspace.root(), &request);
        let outside = self.workspace.classify_resolved(&workdir) == PathScope::Outside;
        let summary = format!(
            "{}\n  in {}\n  effect: {}",
            shell::display_command(&shell::wrap_command(&request)),
            workdir.display(),
            request.effect.as_str()
        );
        // The model classified the effect; only a declared state change
        // needs a review on the no-approval path.
        let review_required = request.effect == Effect::StateChanging;
        self.gate(
            summary,
            outside,
            review_required,
            "a state_changing command",
            request.review(),
        )
    }

    /// Same order as `decide_shell`. An edit is state-changing by the tool's
    /// contract, so the review is required on the no-approval path without
    /// any model-declared effect (PLAN.md §5.10).
    fn decide_edit_file(&mut self, call: &ToolCall) -> Decision {
        let request = match EditFileRequest::parse(&call.input) {
            Ok(request) => request,
            Err(message) => return Decision::Deny(message),
        };
        let path = edit::resolve_path(self.workspace.root(), &request);
        let outside = self.workspace.classify_resolved(&path) == PathScope::Outside;
        let summary = format!(
            "edit_file {}\n  in {}\n  {}",
            request.path,
            self.workspace.root().display(),
            edit::describe_change(&request)
        );
        self.gate(summary, outside, true, "an edit", request.review())
    }

    /// The tail shared by every action tool: which path applies, then the
    /// handshake on that path, then the surface.
    ///
    /// Host approval path (`ask` mode, or outside the workspace): the summary,
    /// any supplied review, and the outside marker go to the approver; a
    /// review is never required here, because host approval is the guarantee
    /// and Keel must not strengthen PIRA silently. No-approval path: a
    /// required review must be present and is announced before `Allow`.
    fn gate(
        &mut self,
        mut summary: String,
        outside: bool,
        review_required: bool,
        what: &str,
        review: Option<&str>,
    ) -> Decision {
        if self.mode == ApprovalMode::Ask || outside {
            if let Some(review) = review {
                summary.push_str(&format!("\n  Safety: {review}"));
            }
            if outside {
                summary.push_str("\n  (outside the workspace)");
            }
            return self.ask(&summary);
        }
        match (review_required, review) {
            (false, _) => Decision::Allow,
            (true, None) => Decision::Deny(format!(
                "PIRA Full-Permission Behavior: {what} needs a non-empty safety_review before \
                 it runs without host approval; resend with the review"
            )),
            (true, Some(review)) => {
                self.approver.announce(&format!("Safety: {review}"));
                Decision::Allow
            }
        }
    }
}

impl Hooks for PermissionEngine {
    fn decide(&mut self, call: &ToolCall) -> Decision {
        if self.read_only.contains(&call.name) {
            return Decision::Allow;
        }
        if call.name == shell::TOOL_NAME {
            return self.decide_shell(call);
        }
        if call.name == edit::TOOL_NAME {
            return self.decide_edit_file(call);
        }
        match self.mode {
            ApprovalMode::Ask => self.ask(&format!("{}({})", call.name, call.input)),
            ApprovalMode::Full => Decision::Allow,
        }
    }
}
