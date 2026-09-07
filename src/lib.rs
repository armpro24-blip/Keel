//! Keel: a minimal, rigorous agent harness that gives PIRA agency.
//!
//! Current scope (see `PLAN.md` §7): M0 and M1 are complete; M2 is in
//! progress. Keel owns the agent loop, drives one real model over a
//! synchronous HTTP round trip, and can read and validate the installed PIRA.
//!
//! ```text
//! user input -> model -> tool call -> tool execution -> observation -> model -> final answer
//! ```
//!
//! Module map:
//! - [`message`]: provider-neutral message types shared by every component.
//! - [`model`]: the model boundary (`Model` trait) and the scripted `FakeModel`.
//! - [`openai`]: the first real adapter, OpenAI Chat Completions.
//! - [`tool`]: the tool boundary (`Tool` trait), the registry, and the `EchoTool`.
//! - [`agent`]: the `AgentLoop`, the only place that drives a conversation.
//! - [`workspace`]: workspace identity (same rule as `pira_ctx`) and boundary.
//! - [`pira`]: the installed PIRA: policy sources, contract checks, `pira.lock`.
//!
//! Not yet present: the ContextManager that assembles PIRA's policy into the
//! system instruction, the shell tool, permissions, and the session log.

pub mod agent;
pub mod message;
pub mod model;
pub mod openai;
pub mod pira;
pub mod tool;
pub mod workspace;
