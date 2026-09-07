//! Keel: a minimal, rigorous agent harness that gives PIRA agency.
//!
//! Current scope (see `PLAN.md` §7): M0 and M1 are complete; M2 is in
//! progress. Keel owns the agent loop, drives one real model over a
//! synchronous HTTP round trip, reads and validates the installed PIRA, and
//! puts PIRA's policy in front of the model.
//!
//! ```text
//! user input -> model -> tool call -> tool execution -> observation -> model -> final answer
//! ```
//!
//! Module map:
//! - [`cli`]: argument parsing for the binary, kept here so it is testable.
//! - [`message`]: provider-neutral message types shared by every component.
//! - [`model`]: the model boundary (`Model` trait) and the scripted `FakeModel`.
//! - [`openai`]: the first real adapter, OpenAI Chat Completions.
//! - [`tool`]: the tool boundary (`Tool` trait), the registry, and the `EchoTool`.
//! - [`agent`]: the `AgentLoop`, the only place that drives a conversation.
//! - [`workspace`]: workspace identity (same rule as `pira_ctx`) and boundary.
//! - [`pira`]: the installed PIRA: policy sources, contract checks, `pira.lock`.
//! - [`context`]: the system instruction: `AGENTS.md` verbatim plus the host block.
//! - [`loader`]: the `read_pira_policy` tool, PIRA's module-loading exception.
//! - [`session`]: one id per process, exported as `PIRA_CTX_THREAD_ID`.
//!
//! Not yet present: the shell tool, permissions, and the session log.

pub mod agent;
pub mod cli;
pub mod context;
pub mod loader;
pub mod message;
pub mod model;
pub mod openai;
pub mod pira;
pub mod session;
pub mod tool;
pub mod workspace;
