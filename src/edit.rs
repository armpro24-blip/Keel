//! The `edit_file` tool: replace exactly one occurrence of a byte sequence
//! in an existing file (PLAN.md §5.10).
//!
//! Why this exists: L1 showed that an argv-only shell gives the model no
//! reliable way to modify an existing file, and the gated experiments showed
//! that a writer program (Gate B) and a whole-file body (Gate C) both lose
//! bytes in the model's hands while an exact `old_text` → `new_text`
//! replacement (Gate D) does not (`docs/evidence/EDIT_FILE_GATE_D_*`).
//!
//! The tool works on bytes. `old_text` and `new_text` are used as their
//! UTF-8 bytes and matched against the file's bytes, so every byte outside
//! the replaced block, line endings included, is preserved by construction
//! and the file is never decoded. Zero or several matches are failures the
//! model sees; Keel never guesses which block was meant.
//!
//! Deliberately absent: fuzzy or regex matching, replace-all, whitespace or
//! newline normalization, file or directory creation, diff preview, atomic
//! replacement (PIRA: known ceiling; a sibling temp file plus rename is the
//! upgrade path).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::message::ToolSpec;
use crate::tool::{Tool, ToolResult};

pub const TOOL_NAME: &str = "edit_file";

/// A validated request from the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditFileRequest {
    /// Relative to the workspace root, or absolute.
    pub path: String,
    /// Non-empty; must occur exactly once in the file.
    pub old_text: String,
    /// May be empty (deletes the block); must differ from `old_text`.
    pub new_text: String,
    /// The model's pre-execution review; Keel validates presence only.
    pub safety_review: Option<String>,
}

impl EditFileRequest {
    /// The model's review with surrounding whitespace removed; `None` when
    /// absent or blank.
    pub fn review(&self) -> Option<&str> {
        self.safety_review
            .as_deref()
            .map(str::trim)
            .filter(|review| !review.is_empty())
    }

    /// Validate the tool input. Every failure is a message the model can act on.
    pub fn parse(input: &Value) -> Result<EditFileRequest, String> {
        let required_string = |key: &str| -> Result<String, String> {
            input
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("input needs a string field '{key}'"))
        };
        let path = required_string("path")?;
        if path.trim().is_empty() {
            return Err("'path' must not be empty".to_string());
        }
        let old_text = required_string("old_text")?;
        if old_text.is_empty() {
            return Err("'old_text' must not be empty".to_string());
        }
        let new_text = required_string("new_text")?;
        if new_text == old_text {
            return Err("'old_text' and 'new_text' are identical; nothing to change".to_string());
        }
        let safety_review = match input.get("safety_review") {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) => Some(text.clone()),
            Some(_) => return Err("'safety_review' must be a string".to_string()),
        };
        Ok(EditFileRequest {
            path,
            old_text,
            new_text,
            safety_review,
        })
    }
}

/// The file a request edits: `path` relative to the workspace root (an
/// absolute `path` stands on its own).
pub fn resolve_path(workspace_root: &Path, request: &EditFileRequest) -> PathBuf {
    workspace_root.join(&request.path)
}

/// One line for the approval prompt: the size of the change, never its body
/// (a multi-kilobyte body is unreviewable at the terminal; the log has it).
pub fn describe_change(request: &EditFileRequest) -> String {
    format!(
        "replaces {} bytes ({} lines) with {} bytes ({} lines)",
        request.old_text.len(),
        request.old_text.lines().count(),
        request.new_text.len(),
        request.new_text.lines().count()
    )
}

/// Byte offsets of every non-overlapping occurrence of `needle` in `haystack`.
fn occurrences(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut found = Vec::new();
    if needle.is_empty() || needle.len() > haystack.len() {
        return found;
    }
    let mut index = 0;
    while index + needle.len() <= haystack.len() {
        if &haystack[index..index + needle.len()] == needle {
            found.push(index);
            index += needle.len();
        } else {
            index += 1;
        }
    }
    found
}

fn has_crlf(bytes: &[u8]) -> bool {
    bytes.windows(2).any(|pair| pair == b"\r\n")
}

/// Apply `request` to the bytes of an existing file. Pure apart from the
/// file system, so the semantics are testable without the tool wrapper.
pub fn apply(path: &Path, shown_path: &str, request: &EditFileRequest) -> ToolResult {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => return ToolResult::error(format!("no such file: {shown_path} ({error})")),
    };
    if !metadata.is_file() {
        return ToolResult::error(format!("not a regular file: {shown_path}"));
    }
    let before = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return ToolResult::error(format!("cannot read {shown_path}: {error}")),
    };

    let old = request.old_text.as_bytes();
    let new = request.new_text.as_bytes();
    let matches = occurrences(&before, old);
    let start = match matches.as_slice() {
        [start] => *start,
        [] => {
            let mut message = format!("old_text not found in {shown_path}");
            match (has_crlf(&before), has_crlf(old)) {
                (true, false) => {
                    message.push_str(" (the file uses CRLF line endings and old_text does not)")
                }
                (false, true) => {
                    message.push_str(" (old_text uses CRLF line endings and the file does not)")
                }
                _ => {}
            }
            return ToolResult::error(message);
        }
        several => {
            return ToolResult::error(format!(
                "old_text occurs {} times in {shown_path}; include more surrounding text so it occurs once",
                several.len()
            ));
        }
    };

    let mut after = Vec::with_capacity(before.len() - old.len() + new.len());
    after.extend_from_slice(&before[..start]);
    after.extend_from_slice(new);
    after.extend_from_slice(&before[start + old.len()..]);
    if let Err(error) = fs::write(path, &after) {
        return ToolResult::error(format!("cannot write {shown_path}: {error}"));
    }
    ToolResult::ok(format!(
        "replaced 1 occurrence in {shown_path}: {} bytes ({} lines) -> {} bytes ({} lines); file {} -> {} bytes",
        old.len(),
        request.old_text.lines().count(),
        new.len(),
        request.new_text.lines().count(),
        before.len(),
        after.len()
    ))
}

/// The tool the model sees. It never decides whether an edit may run; the
/// PermissionEngine does that before `execute` is reached.
pub struct EditFileTool {
    workspace_root: PathBuf,
}

impl EditFileTool {
    pub fn new(workspace_root: PathBuf) -> EditFileTool {
        EditFileTool { workspace_root }
    }
}

impl Tool for EditFileTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: TOOL_NAME.to_string(),
            description:
                "Replace one exact block of text in an existing file. old_text must occur \
                          exactly once byte for byte; bytes outside that block, including line \
                          endings, are untouched. Read the file first and copy old_text exactly. \
                          Relative paths resolve against the workspace root; outside-workspace \
                          paths require host approval. To create a file or run a program, use \
                          shell. In full-permission/no-approval mode a safety_review is required \
                          before the edit runs."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Existing file to edit. Relative paths resolve against the workspace root; paths outside the workspace require host approval."
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The exact existing text to replace; must occur exactly once."
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The exact replacement text; empty deletes the block."
                    },
                    "safety_review": {
                        "type": "string",
                        "description": "Required in full-permission/no-approval mode: the review PIRA's Full-Permission Behavior requires before this edit. Keel shows it as 'Safety: ...' before executing."
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        }
    }

    fn execute(&mut self, input: &Value) -> ToolResult {
        let request = match EditFileRequest::parse(input) {
            Ok(request) => request,
            Err(message) => return ToolResult::error(message),
        };
        let path = resolve_path(&self.workspace_root, &request);
        apply(&path, &request.path, &request)
    }
}
