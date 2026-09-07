//! The model boundary: the whole conversation in, one assistant message out.
//!
//! The trait is synchronous by design (PLAN.md §5.2). Async arrives only when
//! streaming, concurrent I/O, or cancellation creates a demonstrated need.

use std::collections::VecDeque;
use std::fmt;

use crate::message::{Message, ToolSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// A `FakeModel` was asked for more replies than it was scripted with.
    ScriptExhausted,
    /// A real adapter failed. The string is the provider's own message.
    Provider(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::ScriptExhausted => write!(f, "fake model script exhausted"),
            ModelError::Provider(message) => write!(f, "model provider error: {message}"),
        }
    }
}

impl std::error::Error for ModelError {}

pub trait Model {
    /// Produce the next assistant message.
    ///
    /// `system` is the system instruction text, `messages` the full transcript
    /// so far, and `tools` the tools the model may call. The model has no
    /// memory of its own: everything it knows is in these arguments.
    fn complete(
        &mut self,
        system: &str,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<Message, ModelError>;
}

/// A model that replays scripted replies and records what it was shown.
///
/// `seen` lets tests assert that observations actually reached the model
/// instead of trusting the loop's own transcript.
pub struct FakeModel {
    replies: VecDeque<Message>,
    /// One entry per `complete` call: a copy of the transcript passed in.
    pub seen: Vec<Vec<Message>>,
}

impl FakeModel {
    pub fn new(replies: Vec<Message>) -> Self {
        FakeModel {
            replies: replies.into(),
            seen: Vec::new(),
        }
    }
}

impl Model for FakeModel {
    fn complete(
        &mut self,
        _system: &str,
        messages: &[Message],
        _tools: &[ToolSpec],
    ) -> Result<Message, ModelError> {
        self.seen.push(messages.to_vec());
        self.replies.pop_front().ok_or(ModelError::ScriptExhausted)
    }
}
