//! The PermissionEngine: host approval enforcement (PLAN.md §5.6).
//!
//! PIRA states the principle ("never run destructive commands without
//! explicit permission"); this engine decides whether a tool call executes.
//! Trusted PIRA policy loading is read-only and never asks. Actions follow
//! the approval mode. A working directory outside the workspace asks in every
//! mode, because PIRA makes the workspace the default allowed scope with the
//! platform temp directory as the only standing exception.
//!
//! The engine does not judge destructive risk; that semantic review stays
//! with PIRA (`Safety:` in full mode). It only asks the user or does not.

use crate::agent::{Decision, ToolGate};
use crate::message::ToolCall;
use crate::shell::{self, ShellRequest};
use crate::workspace::{PathScope, Workspace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalMode {
    /// Every action needs the user's confirmation.
    Ask,
    /// Actions run without asking; PIRA's full-permission rules apply to the
    /// model. Keel provides no sandbox.
    Full,
}

impl ApprovalMode {
    /// The wording the host block gives the model (PLAN.md §4.3-1).
    pub fn describe(self) -> &'static str {
        match self {
            ApprovalMode::Ask => {
                "ask (the user confirms each action before it runs; loading PIRA policy needs no confirmation)"
            }
            ApprovalMode::Full => {
                "full (no approval prompts and no sandbox; PIRA's full-permission rules apply)"
            }
        }
    }
}

/// Whoever answers approval prompts: the person at the REPL, or a test.
pub trait Approver {
    /// `summary` describes what would run and where. `true` allows it.
    fn approve(&mut self, summary: &str) -> bool;
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
}

impl ToolGate for PermissionEngine {
    fn decide(&mut self, call: &ToolCall) -> Decision {
        if self.read_only.contains(&call.name) {
            return Decision::Allow;
        }

        if call.name == shell::TOOL_NAME {
            // Malformed input is the tool's error to report, not a permission
            // question; let it through so the model sees the validation message.
            let Ok(request) = ShellRequest::parse(&call.input) else {
                return Decision::Allow;
            };
            let workdir = shell::resolve_workdir(self.workspace.root(), &request);
            let summary = format!(
                "{}\n  in {}",
                shell::display_command(&shell::wrap_command(&request)),
                workdir.display()
            );
            if self.workspace.classify_resolved(&workdir) == PathScope::Outside {
                return self.ask(&format!("{summary}\n  (outside the workspace)"));
            }
            return match self.mode {
                ApprovalMode::Ask => self.ask(&summary),
                ApprovalMode::Full => Decision::Allow,
            };
        }

        match self.mode {
            ApprovalMode::Ask => self.ask(&format!("{}({})", call.name, call.input)),
            ApprovalMode::Full => Decision::Allow,
        }
    }
}
