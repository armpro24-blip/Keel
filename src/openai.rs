//! The first real model adapter: OpenAI Chat Completions with function calling.
//!
//! Two layers, kept apart so the wire mapping is testable without a network:
//! - `to_wire` / `from_wire` translate between Keel's neutral messages and the
//!   Chat Completions JSON shapes;
//! - `OpenAiChatModel::complete` does one synchronous HTTP round trip.
//!
//! Chat Completions was chosen over the newer Responses API because it is the
//! shape most OpenAI-compatible servers also speak, which serves "plug in an
//! LLM and it works" at no extra cost. Synchronous by design (PLAN.md §5.2).

use std::time::Duration;

use serde_json::{json, Value};

use crate::log::SessionLog;
use crate::message::{Block, Message, Provenance, Role, ToolSpec};
use crate::model::{Model, ModelError};

pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// Upper bound for one round trip. Generous because a long completion is
/// legitimate; the bound exists so a dead server cannot hang the REPL forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

/// Build the request body for `POST {base_url}/chat/completions`.
///
/// `system` becomes the leading `system` message; see `user_wire_messages`
/// and `assistant_wire_message` for the per-role mapping. `tools` is omitted
/// from the body when empty.
pub fn to_wire(
    model: &str,
    system: &str,
    messages: &[Message],
    tools: &[ToolSpec],
) -> Result<Value, ModelError> {
    let mut wire_messages = vec![json!({ "role": "system", "content": system })];
    for message in messages {
        match message.role {
            Role::User => wire_messages.extend(user_wire_messages(message)?),
            Role::Assistant => wire_messages.push(assistant_wire_message(message)?),
        }
    }

    let mut body = json!({ "model": model, "messages": wire_messages });
    if !tools.is_empty() {
        let wire_tools: Vec<Value> = tools.iter().map(tool_wire).collect();
        body["tools"] = Value::Array(wire_tools);
    }
    Ok(body)
}

/// A user message becomes one wire message per block: `Text` → `user`,
/// `ToolResult` → `tool` (the API wants one `tool` message per result).
/// The wire format has neither an error flag nor a provenance field, so both
/// are rendered into the content: an error result is prefixed with
/// `[tool error]`; a PIRA policy result is wrapped in a `<pira_policy>` frame
/// that names its source path, which is what PIRA's trust rule keys on.
fn user_wire_messages(message: &Message) -> Result<Vec<Value>, ModelError> {
    let mut wire = Vec::with_capacity(message.blocks.len());
    for block in &message.blocks {
        match block {
            Block::Text(text) => wire.push(json!({ "role": "user", "content": text })),
            Block::ToolResult {
                call_id,
                output,
                is_error,
                provenance,
            } => {
                let content = match provenance {
                    Provenance::PiraPolicy { source } => {
                        format!("<pira_policy source=\"{source}\">\n{output}\n</pira_policy>")
                    }
                    Provenance::Observation if *is_error => format!("[tool error] {output}"),
                    Provenance::Observation => output.clone(),
                };
                wire.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": content,
                }));
            }
            Block::ToolCall { .. } => {
                return Err(ModelError::InvalidTranscript(
                    "user message cannot contain a tool call".to_string(),
                ));
            }
        }
    }
    Ok(wire)
}

/// An assistant message carries its text as `content` (null when empty) and
/// its tool calls as `tool_calls`, with `arguments` serialized to a JSON string.
fn assistant_wire_message(message: &Message) -> Result<Value, ModelError> {
    let mut tool_calls = Vec::new();
    for block in &message.blocks {
        match block {
            Block::Text(_) => {}
            Block::ToolCall { id, name, input } => tool_calls.push(json!({
                "id": id,
                "type": "function",
                "function": { "name": name, "arguments": input.to_string() },
            })),
            Block::ToolResult { .. } => {
                return Err(ModelError::InvalidTranscript(
                    "assistant message cannot contain a tool result".to_string(),
                ));
            }
        }
    }

    let text = message.text();
    let content = if text.is_empty() {
        Value::Null
    } else {
        Value::String(text)
    };
    let mut wire = json!({ "role": "assistant", "content": content });
    if !tool_calls.is_empty() {
        wire["tool_calls"] = Value::Array(tool_calls);
    }
    Ok(wire)
}

fn tool_wire(tool: &ToolSpec) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.input_schema,
        },
    })
}

/// Turn a Chat Completions response body into one assistant message.
///
/// An `error` object in the body is reported as `ModelError::Provider`, as is
/// a tool call whose `arguments` is not valid JSON: the failure stays visible
/// instead of being smoothed over.
pub fn from_wire(body: &Value) -> Result<Message, ModelError> {
    if let Some(error) = body.get("error") {
        let detail = error
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| error.to_string());
        return Err(ModelError::Provider(detail));
    }

    let wire_message = body.pointer("/choices/0/message").ok_or_else(|| {
        ModelError::Provider(format!("response has no choices[0].message: {body}"))
    })?;

    let mut blocks = Vec::new();
    if let Some(text) = wire_message.get("content").and_then(Value::as_str) {
        if !text.is_empty() {
            blocks.push(Block::Text(text.to_string()));
        }
    }

    if let Some(calls) = wire_message.get("tool_calls").and_then(Value::as_array) {
        for call in calls {
            let id = required_str(call, "id")?;
            let name = required_str(&call["function"], "name")?;
            let arguments = call
                .pointer("/function/arguments")
                .and_then(Value::as_str)
                .unwrap_or("");
            let input = if arguments.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(arguments).map_err(|error| {
                    ModelError::Provider(format!(
                        "tool call {id} for {name} has malformed arguments ({error}): {arguments}"
                    ))
                })?
            };
            blocks.push(Block::ToolCall {
                id: id.to_string(),
                name: name.to_string(),
                input,
            });
        }
    }

    Ok(Message {
        role: Role::Assistant,
        blocks,
    })
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, ModelError> {
    value.get(key).and_then(Value::as_str).ok_or_else(|| {
        ModelError::Provider(format!("tool call is missing string field {key}: {value}"))
    })
}

pub struct OpenAiChatModel {
    agent: ureq::Agent,
    base_url: String,
    api_key: String,
    model: String,
    /// When set, every request body sent and every response body received is
    /// appended verbatim. This is the instruction-path audit's first step
    /// (PLAN.md §2 "Own the instruction path"): what the model was actually
    /// sent, byte for byte, not what Keel meant to send.
    wire_log: Option<SessionLog>,
}

impl OpenAiChatModel {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        // Non-2xx responses are read as bodies, not raised as transport errors,
        // so the provider's own error message reaches `from_wire`.
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build();
        OpenAiChatModel {
            agent: config.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
            wire_log: None,
        }
    }

    /// Record exact request and response bodies to `log`.
    pub fn record_wire_to(&mut self, log: SessionLog) {
        self.wire_log = Some(log);
    }

    /// The most recent failure to write the wire log, if any.
    pub fn take_wire_log_failure(&mut self) -> Option<String> {
        self.wire_log.as_mut().and_then(SessionLog::take_failure)
    }

    /// Read `OPENAI_API_KEY` (required) and `OPENAI_BASE_URL` (optional).
    pub fn from_env(model: String) -> Result<Self, String> {
        let api_key =
            std::env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY is not set".to_string())?;
        let base_url =
            std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Ok(OpenAiChatModel::new(api_key, base_url, model))
    }

    fn post_chat_completion(&self, body: &Value) -> Result<Value, ModelError> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut response = self
            .agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send_json(body)
            .map_err(|error| ModelError::Provider(format!("HTTP request failed: {error}")))?;
        let status = response.status().as_u16();
        let value: Value = response.body_mut().read_json().map_err(|error| {
            ModelError::Provider(format!("HTTP {status}: unreadable body: {error}"))
        })?;
        if !(200..300).contains(&status) && value.get("error").is_none() {
            return Err(ModelError::Provider(format!("HTTP {status}: {value}")));
        }
        Ok(value)
    }
}

impl Model for OpenAiChatModel {
    fn complete(
        &mut self,
        system: &str,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<Message, ModelError> {
        let body = to_wire(&self.model, system, messages, tools)?;
        if let Some(log) = &mut self.wire_log {
            log.record(json!({ "event": "request", "body": body }));
        }
        let response = self.post_chat_completion(&body);
        if let Some(log) = &mut self.wire_log {
            match &response {
                Ok(body) => log.record(json!({ "event": "response", "body": body })),
                Err(error) => log.record(json!({
                    "event": "response_error",
                    "error": error.to_string(),
                })),
            }
        }
        from_wire(&response?)
    }
}
