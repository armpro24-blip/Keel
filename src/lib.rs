//! Keel: a minimal, rigorous agent harness.
//!
//! Current scope (see `PLAN.md` §7, M0–M1): Keel owns the agent loop and can
//! drive one real model over a synchronous HTTP round trip.
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
//!
//! Nothing here knows about PIRA, shells, permissions, or persistence yet.
//! Those arrive in later milestones only when an observed need requires them.

pub mod agent;
pub mod message;
pub mod model;
pub mod openai;
pub mod tool;
