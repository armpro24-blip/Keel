//! SessionLog: append-only provenance for one session (PLAN.md §5.9).
//!
//! Every message the loop appends, every gate decision, and every run outcome
//! is written as one JSON line, in order, as it happens. The log exists so a
//! session can be reconstructed and inspected afterwards; it is never fed
//! back to the model and it is not an execution script. PIRA's activity
//! memory (`pira_ctx`) records what commands did; this records what the
//! conversation did. The two are complementary, as PIRA's README describes.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::agent::{Decision, ToolGate};
use crate::message::{Block, Message, Provenance, Role, ToolCall};
use crate::pira::{home_dir, sha256_hex, PiraError};

pub struct SessionLog {
    path: PathBuf,
    file: File,
    /// The most recent write failure, surfaced by `take_failure` so the host
    /// can warn without the logger printing anything itself.
    failure: Option<String>,
}

impl SessionLog {
    /// Create `<dir>/<session_id>.jsonl` and open it for appending. Failing
    /// to open is an error for the caller: a session without evidence should
    /// not start quietly.
    pub fn open(dir: &Path, session_id: &str) -> Result<SessionLog, String> {
        fs::create_dir_all(dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
        let path = dir.join(format!("{session_id}.jsonl"));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
        Ok(SessionLog {
            path,
            file,
            failure: None,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one event. `event` must be a JSON object; a timestamp in Unix
    /// milliseconds is added as `t`. Write failures are remembered, not
    /// raised, so a full disk cannot abort the conversation mid-turn.
    pub fn record(&mut self, mut event: Value) {
        if let Some(object) = event.as_object_mut() {
            object.insert("t".to_string(), json!(unix_millis()));
        }
        let line = event.to_string();
        let result = self
            .file
            .write_all(line.as_bytes())
            .and_then(|()| self.file.write_all(b"\n"))
            .and_then(|()| self.file.flush());
        if let Err(error) = result {
            self.failure = Some(format!("cannot append to {}: {error}", self.path.display()));
        }
    }

    pub fn take_failure(&mut self) -> Option<String> {
        self.failure.take()
    }
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// `~/.keel/sessions/<workspace-hash>/`, one directory per workspace so a
/// workspace's sessions sit together, mirroring PIRA's workspace scoping.
pub fn default_sessions_dir(workspace_root: &Path) -> Result<PathBuf, PiraError> {
    let workspace_hash = &sha256_hex(workspace_root.display().to_string().as_bytes())[..16];
    Ok(home_dir()?
        .join(".keel")
        .join("sessions")
        .join(workspace_hash))
}

/// A message as it appears in the log.
pub fn message_to_json(message: &Message) -> Value {
    let role = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };
    let blocks: Vec<Value> = message
        .blocks
        .iter()
        .map(|block| match block {
            Block::Text(text) => json!({ "type": "text", "text": text }),
            Block::ToolCall { id, name, input } => {
                json!({ "type": "tool_call", "id": id, "name": name, "input": input })
            }
            Block::ToolResult {
                call_id,
                output,
                is_error,
                provenance,
            } => {
                let provenance = match provenance {
                    Provenance::Observation => json!("observation"),
                    Provenance::PiraPolicy { source } => json!({ "pira_policy": source }),
                };
                json!({
                    "type": "tool_result",
                    "call_id": call_id,
                    "output": output,
                    "is_error": is_error,
                    "provenance": provenance,
                })
            }
        })
        .collect();
    json!({ "event": "message", "role": role, "blocks": blocks })
}

/// Read every event of a log file, in order. Used to reconstruct a session;
/// a malformed line is an error, not something to skip silently.
pub fn read_events(path: &Path) -> Result<Vec<Value>, String> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut events = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let event: Value = serde_json::from_str(&line).map_err(|error| {
            format!(
                "{} line {} is not valid JSON: {error}",
                path.display(),
                index + 1
            )
        })?;
        events.push(event);
    }
    Ok(events)
}

/// One readable line per event, for `keel log show`. Long outputs are shown
/// in full: this is inspection, and truncation would hide evidence.
pub fn render_event(event: &Value) -> String {
    let kind = event.get("event").and_then(Value::as_str).unwrap_or("?");
    match kind {
        "message" => {
            let role = event.get("role").and_then(Value::as_str).unwrap_or("?");
            let lines: Vec<String> = event
                .get("blocks")
                .and_then(Value::as_array)
                .map(|blocks| blocks.iter().map(render_block).collect())
                .unwrap_or_default();
            format!("[{role}]\n{}", lines.join("\n"))
        }
        "decision" => format!(
            "[decision] {} {} -> {}",
            event.get("call_id").and_then(Value::as_str).unwrap_or("?"),
            event.get("tool").and_then(Value::as_str).unwrap_or("?"),
            event
                .get("decision")
                .map(Value::to_string)
                .unwrap_or_default()
        ),
        other => {
            let mut rest = event.clone();
            if let Some(object) = rest.as_object_mut() {
                object.remove("event");
                object.remove("t");
            }
            format!("[{other}] {rest}")
        }
    }
}

fn render_block(block: &Value) -> String {
    let text = |key: &str| block.get(key).and_then(Value::as_str).unwrap_or("");
    match block.get("type").and_then(Value::as_str) {
        Some("text") => format!("  text: {}", text("text")),
        Some("tool_call") => format!(
            "  tool_call {}: {}({})",
            text("id"),
            text("name"),
            block.get("input").map(Value::to_string).unwrap_or_default()
        ),
        Some("tool_result") => {
            let origin = match block.get("provenance") {
                Some(Value::Object(map)) => map
                    .get("pira_policy")
                    .and_then(Value::as_str)
                    .map(|source| format!(", policy={source}"))
                    .unwrap_or_default(),
                _ => String::new(),
            };
            format!(
                "  tool_result {} (error={}{origin}): {}",
                text("call_id"),
                block
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                text("output")
            )
        }
        _ => format!("  {block}"),
    }
}

/// A gate that records every decision before returning it, so the log shows
/// what was asked and what was decided even when nothing executed.
pub struct LoggedGate<'a> {
    inner: &'a mut dyn ToolGate,
    log: &'a mut SessionLog,
}

impl<'a> LoggedGate<'a> {
    pub fn new(inner: &'a mut dyn ToolGate, log: &'a mut SessionLog) -> LoggedGate<'a> {
        LoggedGate { inner, log }
    }
}

impl ToolGate for LoggedGate<'_> {
    fn decide(&mut self, call: &ToolCall) -> Decision {
        let decision = self.inner.decide(call);
        let verdict = match &decision {
            Decision::Allow => json!("allow"),
            Decision::Deny(reason) => json!({ "deny": reason }),
        };
        self.log.record(json!({
            "event": "decision",
            "call_id": call.id,
            "tool": call.name,
            "input": call.input,
            "decision": verdict,
        }));
        decision
    }
}
