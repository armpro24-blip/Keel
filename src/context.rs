//! ContextManager: assembles the system instruction (PLAN.md §5.8, §4.3).
//!
//! The system instruction is PIRA's `AGENTS.md` byte for byte, followed by
//! the host block: the only place Keel tells the model anything about itself.
//! The block is short and its fields are exactly those listed in PLAN.md
//! §4.3-1; it exists because PIRA asks the model to assess the approval mode
//! and establish the workspace boundary at session start, and only the host
//! knows those facts.

use std::path::{Path, PathBuf};

use crate::message::{Block, Message, Provenance};

/// Facts about the host that the model needs (PLAN.md §4.3-1).
pub struct HostInfo {
    pub cwd: PathBuf,
    pub workspace_root: PathBuf,
    /// Human-readable description of the approval mode in force.
    pub approval_mode: String,
    /// Unix seconds, for the date line.
    pub unix_time: u64,
}

pub struct ContextManager {
    agents_md: String,
    host_block: String,
}

impl ContextManager {
    pub fn new(agents_md: String, host: &HostInfo) -> ContextManager {
        ContextManager {
            agents_md,
            host_block: host_block(host),
        }
    }

    /// `AGENTS.md` unchanged, one blank line, then the host block.
    pub fn system_instruction(&self) -> String {
        let mut text = self.agents_md.clone();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
        text.push_str(&self.host_block);
        text
    }

    pub fn agents_md(&self) -> &str {
        &self.agents_md
    }

    pub fn host_block(&self) -> &str {
        &self.host_block
    }
}

fn host_block(host: &HostInfo) -> String {
    format!(
        "<keel_host>\nharness: keel {}\nplatform: {} {}\ndate: {}\ncwd: {}\nworkspace_root: {}\napproval_mode: {}\n</keel_host>\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        utc_date(host.unix_time),
        display(&host.cwd),
        display(&host.workspace_root),
        host.approval_mode,
    )
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

/// PIRA policy sources currently in the transcript, in first-load order and
/// without duplicates. Compaction (M4) uses this to know what to restore.
pub fn active_policies(transcript: &[Message]) -> Vec<String> {
    let mut sources: Vec<String> = Vec::new();
    for message in transcript {
        for block in &message.blocks {
            if let Block::ToolResult {
                provenance: Provenance::PiraPolicy { source },
                is_error: false,
                ..
            } = block
            {
                if !sources.iter().any(|known| known == source) {
                    sources.push(source.clone());
                }
            }
        }
    }
    sources
}

/// `YYYY-MM-DD` in UTC for a Unix timestamp. Plain arithmetic (the
/// days-to-civil algorithm) so the date line needs no dependency.
pub fn utc_date(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
