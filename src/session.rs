//! Session identity (PLAN.md §5.10).
//!
//! One Keel process is one session. The id is exported to every subprocess
//! as `PIRA_CTX_THREAD_ID`, the first variable `pira_ctx` consults when it
//! decides which thread a command belongs to, so PIRA's activity memory
//! groups Keel's commands the way it groups Codex's.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::pira::sha256_hex;

/// The environment variable `pira_ctx` reads first for thread identity.
pub const THREAD_ID_ENV: &str = "PIRA_CTX_THREAD_ID";

/// Distinguishes ids created within one clock tick of each other.
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

impl SessionId {
    /// A fresh id: 16 hex characters derived from the clock, the process id,
    /// a process-wide sequence number, and the workspace root. `pira_ctx`
    /// hashes it again and never stores the raw value, so it needs to be
    /// unique, not secret.
    pub fn new(workspace_root: &Path) -> SessionId {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let seed = format!(
            "{nanos}\n{}\n{sequence}\n{}",
            std::process::id(),
            workspace_root.display()
        );
        SessionId(sha256_hex(seed.as_bytes())[..16].to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
