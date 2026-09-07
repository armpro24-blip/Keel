//! The agent loop: the one place that turns a user input into a final answer.
//!
//! Ownership (PLAN.md §3): loop execution, model invocation, and tool dispatch
//! belong to Keel, and they all happen here. The caller owns the model, the
//! tools, the transcript, and the gate, and lends them for one run. Lending
//! the transcript is what lets a REPL continue a conversation across inputs;
//! in a later slice the ContextManager takes over that ownership.

use std::fmt;

use crate::message::{Block, Message, Role, ToolCall};
use crate::model::{Model, ModelError};
use crate::tool::{ToolRegistry, ToolResult};

/// Whether a tool call may execute. Decided before dispatch, once per call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    /// Not executed; the reason becomes the model's observation.
    Deny(String),
}

/// The hook between "the model asked" and "Keel did". The PermissionEngine
/// implements it; the session log will observe it (PLAN.md §5.6, §5.9).
pub trait ToolGate {
    fn decide(&mut self, call: &ToolCall) -> Decision;
}

/// A gate that allows everything: for tests and for hosts without permissions.
pub struct AllowAll;

impl ToolGate for AllowAll {
    fn decide(&mut self, _call: &ToolCall) -> Decision {
        Decision::Allow
    }
}

pub struct AgentLoop {
    /// System instruction text passed to the model on every call.
    pub system: String,
    /// Fuse: the maximum number of model calls in one `run`.
    ///
    /// Exceeding it is an explicit error, never a silent truncation
    /// (PLAN.md §5.3, invariant 1).
    pub max_turns: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunOutcome {
    /// Text of the final assistant message.
    pub final_text: String,
    /// Number of model calls made in this run.
    pub turns: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoopError {
    /// The fuse blew. The transcript up to that point stays with the caller.
    MaxTurnsExceeded {
        max_turns: usize,
    },
    Model(ModelError),
}

impl fmt::Display for LoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoopError::MaxTurnsExceeded { max_turns } => {
                write!(f, "agent loop exceeded max_turns = {max_turns}")
            }
            LoopError::Model(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for LoopError {}

impl AgentLoop {
    /// Append `user_input` to `transcript` and drive the conversation to a
    /// final answer, appending every message produced along the way.
    ///
    /// Each turn: call the model; if it made no tool calls, its text is the
    /// final answer. Otherwise, for every tool call in declaration order, ask
    /// the gate, execute when allowed, and collect the observation; return all
    /// observations in one message and call the model again.
    ///
    /// On error the transcript keeps whatever was appended before the
    /// failure, so the caller can inspect it.
    pub fn run(
        &self,
        model: &mut dyn Model,
        tools: &mut ToolRegistry,
        gate: &mut dyn ToolGate,
        transcript: &mut Vec<Message>,
        user_input: &str,
    ) -> Result<RunOutcome, LoopError> {
        let specs = tools.specs();
        transcript.push(Message::user_text(user_input));

        for turn in 1..=self.max_turns {
            let reply = model
                .complete(&self.system, transcript, &specs)
                .map_err(LoopError::Model)?;
            let calls = reply.tool_calls();
            let reply_text = reply.text();
            transcript.push(reply);

            if calls.is_empty() {
                return Ok(RunOutcome {
                    final_text: reply_text,
                    turns: turn,
                });
            }

            // Sequential on purpose: PIRA's `pira_ctx watch --current` assumes
            // at most one live capture per thread (PLAN.md §5.3, T8).
            let mut results = Vec::with_capacity(calls.len());
            for call in calls {
                let result = match gate.decide(&call) {
                    Decision::Deny(reason) => ToolResult::error(format!("not executed: {reason}")),
                    Decision::Allow => match tools.get_mut(&call.name) {
                        Some(tool) => tool.execute(&call.input),
                        None => ToolResult::error(format!("unknown tool: {}", call.name)),
                    },
                };
                results.push(Block::ToolResult {
                    call_id: call.id,
                    output: result.output,
                    is_error: result.is_error,
                    provenance: result.provenance,
                });
            }
            transcript.push(Message {
                role: Role::User,
                blocks: results,
            });
        }

        Err(LoopError::MaxTurnsExceeded {
            max_turns: self.max_turns,
        })
    }
}
