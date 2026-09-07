//! Provider-neutral message types.
//!
//! The agent loop and the tools never see a provider's wire format. A model
//! adapter translates between these types and its provider (PLAN.md §5.1).
//! Keeping this boundary neutral is what lets Keel accept a different model
//! later without touching the loop.

use serde_json::Value;

/// Who produced a message.
///
/// Tool results travel in a `User` message. This follows the convention of
/// current tool-use APIs: the harness, acting on the user's side of the
/// conversation, reports what the tools observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

/// One unit of content inside a message.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Text(String),
    /// A request from the model to run a tool.
    ToolCall {
        /// Pairs the call with its result. Assigned by the model adapter.
        id: String,
        name: String,
        input: Value,
    },
    /// The observation returned to the model for one `ToolCall`.
    ToolResult {
        call_id: String,
        output: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub role: Role,
    pub blocks: Vec<Block>,
}

impl Message {
    pub fn user_text(text: impl Into<String>) -> Self {
        Message {
            role: Role::User,
            blocks: vec![Block::Text(text.into())],
        }
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Message {
            role: Role::Assistant,
            blocks: vec![Block::Text(text.into())],
        }
    }

    /// Concatenated text blocks in order. Non-text blocks are skipped.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for block in &self.blocks {
            if let Block::Text(text) = block {
                out.push_str(text);
            }
        }
        out
    }

    /// Tool calls in declaration order. The loop executes them in this order.
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        for block in &self.blocks {
            if let Block::ToolCall { id, name, input } = block {
                calls.push(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                });
            }
        }
        calls
    }
}

/// An owned copy of one `Block::ToolCall`, convenient for dispatch.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// What the model is told about a tool.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema describing `ToolCall::input`.
    pub input_schema: Value,
}
