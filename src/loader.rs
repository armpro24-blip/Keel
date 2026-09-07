//! The PIRA policy loader tool (PLAN.md §5.5).
//!
//! PIRA `master` exempts "commands that only load PIRA modules" from the
//! `pira_ctx` wrapping rule. This tool is the runtime form of that exception:
//! it reads exactly one declared policy source, returns its exact bytes, and
//! never starts a process. Only names the routing table declares are
//! accepted; that check is the loader's trust boundary. Ordinary file reads
//! do not go through here and keep PIRA's shell semantics.

use serde_json::{json, Value};

use crate::message::ToolSpec;
use crate::pira::{PiraInstall, Policy};
use crate::tool::{Tool, ToolResult};

pub const TOOL_NAME: &str = "read_pira_policy";

pub struct PolicyLoader {
    install: PiraInstall,
    policy: Policy,
}

impl PolicyLoader {
    pub fn new(install: PiraInstall, policy: Policy) -> PolicyLoader {
        PolicyLoader { install, policy }
    }
}

impl Tool for PolicyLoader {
    fn spec(&self) -> ToolSpec {
        let names: Vec<&str> = self
            .policy
            .sources
            .iter()
            .map(|source| source.name.as_str())
            .collect();
        ToolSpec {
            name: TOOL_NAME.to_string(),
            description: format!(
                "Read one PIRA policy source exactly, by the name PIRA's routing table uses \
                 ({}). Use this, not a shell command, to load a PIRA module or the user profile.",
                names.join(", ")
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "enum": names }
                },
                "required": ["name"]
            }),
        }
    }

    fn execute(&mut self, input: &Value) -> ToolResult {
        let Some(name) = input.get("name").and_then(Value::as_str) else {
            return ToolResult::error("input needs a string field 'name'");
        };
        let Some(source) = self.policy.source(name) else {
            return ToolResult::error(format!(
                "'{name}' is not a policy source declared by AGENTS.md"
            ));
        };
        match self.install.read(source) {
            Ok(text) => ToolResult::policy(text, Policy::display_path(source)),
            Err(error) => ToolResult::error(error.to_string()),
        }
    }
}
