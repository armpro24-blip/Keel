//! The tool boundary: a named capability the model may invoke.
//!
//! There is no permission layer here yet. Later milestones put the
//! PermissionEngine between the loop and `execute` (PLAN.md §5.6); the trait
//! itself does not change for that.

use std::fmt;

use serde_json::{json, Value};

use crate::message::{Provenance, ToolSpec};

/// What a tool observed. Errors are ordinary observations for the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub output: String,
    pub is_error: bool,
    pub provenance: Provenance,
}

impl ToolResult {
    /// Ordinary output: task data.
    pub fn ok(output: impl Into<String>) -> Self {
        ToolResult {
            output: output.into(),
            is_error: false,
            provenance: Provenance::Observation,
        }
    }

    /// A failure the model should see and may recover from.
    pub fn error(output: impl Into<String>) -> Self {
        ToolResult {
            output: output.into(),
            is_error: true,
            provenance: Provenance::Observation,
        }
    }

    /// Exact text of a PIRA policy source; only the PIRA loader produces this.
    pub fn policy(output: impl Into<String>, source: impl Into<String>) -> Self {
        ToolResult {
            output: output.into(),
            is_error: false,
            provenance: Provenance::PiraPolicy {
                source: source.into(),
            },
        }
    }
}

pub trait Tool {
    fn spec(&self) -> ToolSpec;

    /// Execute one call.
    ///
    /// Tool failures are reported through `ToolResult::is_error` so the model
    /// can see and react to them. A Rust error would mean a harness fault.
    fn execute(&mut self, input: &Value) -> ToolResult;
}

/// Registering a second tool under an existing name.
///
/// Rejected rather than shadowed: a duplicate is a host configuration bug and
/// silently picking one would hide it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateToolName(pub String);

impl fmt::Display for DuplicateToolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "duplicate tool name: {}", self.0)
    }
}

impl std::error::Error for DuplicateToolName {}

/// The tools available in one session, looked up by name.
///
/// The name is recorded once at registration; it is the identity the model
/// uses, so lookups compare it directly instead of rebuilding specs.
#[derive(Default)]
pub struct ToolRegistry {
    tools: Vec<(String, Box<dyn Tool>)>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry::default()
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) -> Result<(), DuplicateToolName> {
        let name = tool.spec().name;
        if self.tools.iter().any(|(existing, _)| *existing == name) {
            return Err(DuplicateToolName(name));
        }
        self.tools.push((name, tool));
        Ok(())
    }

    /// Specs in registration order, as shown to the model.
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools.iter().map(|(_, tool)| tool.spec()).collect()
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn Tool> {
        let index = self
            .tools
            .iter()
            .position(|(existing, _)| existing == name)?;
        Some(self.tools[index].1.as_mut())
    }
}

/// The M0 deterministic tool: returns its input serialized as compact JSON.
pub struct EchoTool;

impl Tool for EchoTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "echo".to_string(),
            description: "Returns the JSON input unchanged.".to_string(),
            input_schema: json!({ "type": "object" }),
        }
    }

    fn execute(&mut self, input: &Value) -> ToolResult {
        ToolResult::ok(input.to_string())
    }
}
