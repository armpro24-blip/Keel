//! Keel: a minimal, rigorous agent harness that gives PIRA agency.
//!
//! Current scope (see `PLAN.md` §7): M0 and M1 are complete; M2 is in
//! progress. Keel owns the agent loop, drives one real model over a
//! synchronous HTTP round trip, reads and validates the installed PIRA, puts
//! PIRA's policy in front of the model, and lets it act through a shell tool
//! behind a permission gate.
//!
//! ```text
//! user input -> model -> tool call -> gate -> tool execution -> observation -> model -> final answer
//! ```
//!
//! Module map:
//! - [`cli`]: argument parsing for the binary, kept here so it is testable.
//! - [`message`]: provider-neutral message types shared by every component.
//! - [`model`]: the model boundary (`Model` trait) and the scripted `FakeModel`.
//! - [`openai`]: the first real adapter, OpenAI Chat Completions.
//! - [`tool`]: the tool boundary (`Tool` trait), the registry, and the `EchoTool`.
//! - [`agent`]: the `AgentLoop` and the `Hooks` it reports to (decisions before dispatch, messages as they join).
//! - [`workspace`]: workspace identity (same rule as `pira_ctx`) and boundary.
//! - [`pira`]: the installed PIRA: policy sources, contract checks, `pira.lock`.
//! - [`context`]: the system instruction: `AGENTS.md` verbatim plus the host block.
//! - [`loader`]: the `read_pira_policy` tool, PIRA's module-loading exception.
//! - [`session`]: one id per process, exported as `PIRA_CTX_THREAD_ID`.
//! - [`shell`]: the `shell` tool; every command through `pira_ctx` unless it is a PIRA tool.
//! - [`edit`]: the `edit_file` tool; exact one-match replacement in an existing file, on bytes.
//! - [`permission`]: the PermissionEngine, `ask` or `full`, outside-workspace always asks.
//! - [`log`]: the SessionLog, append-only JSONL provenance; reconstruct, never replay.
//!
//! Not yet present: compaction.

pub mod agent;
pub mod cli;
pub mod context;
pub mod edit;
pub mod loader;
pub mod log;
pub mod message;
pub mod model;
pub mod openai;
pub mod permission;
pub mod pira;
pub mod session;
pub mod shell;
pub mod tool;
pub mod workspace;
