//! Tests for the system instruction, the host block, and session ids
//! (PLAN.md §4.3, §5.8, §5.10; invariants 4–5).

use std::path::{Path, PathBuf};

use keel::context::{active_policies, utc_date, ContextManager, HostInfo};
use keel::message::{Block, Message, Provenance, Role};
use keel::pira::sha256_hex;
use keel::session::{SessionId, THREAD_ID_ENV};

fn host() -> HostInfo {
    HostInfo {
        cwd: PathBuf::from("/work/keel/src"),
        workspace_root: PathBuf::from("/work/keel"),
        approval_mode: "ask".to_string(),
        unix_time: 1_788_739_200, // 2026-09-07 00:00:00 UTC
    }
}

#[test]
fn system_instruction_is_agents_md_verbatim_followed_by_the_host_block() {
    let agents_md = "# PIRA\n31415926535897932384626433832795\n".to_string();
    let context = ContextManager::new(agents_md.clone(), &host());

    let system = context.system_instruction();

    assert!(
        system.starts_with(&agents_md),
        "AGENTS.md must lead unchanged"
    );
    assert_eq!(
        sha256_hex(context.agents_md().as_bytes()),
        sha256_hex(agents_md.as_bytes())
    );
    let block = context.host_block();
    assert!(system.ends_with(block));
    assert!(block.starts_with("<keel_host>\n"));
    assert!(block.contains("\ndate: 2026-09-07\n"));
    assert!(block.contains("\nworkspace_root: /work/keel\n"));
    assert!(block.contains("\napproval_mode: ask\n"));
    assert!(block.ends_with("</keel_host>\n"));
}

#[test]
fn utc_date_handles_epoch_leap_days_and_year_ends() {
    assert_eq!(utc_date(0), "1970-01-01");
    assert_eq!(utc_date(951_782_400), "2000-02-29");
    assert_eq!(utc_date(1_788_739_200), "2026-09-07");
    assert_eq!(utc_date(1_788_739_200 + 86_399), "2026-09-07");
    assert_eq!(utc_date(1_767_225_599), "2025-12-31");
    assert_eq!(utc_date(1_767_225_600), "2026-01-01");
}

#[test]
fn active_policies_lists_successful_policy_sources_once_in_order() {
    let policy = |source: &str| Block::ToolResult {
        call_id: "c".to_string(),
        output: "text".to_string(),
        is_error: false,
        provenance: Provenance::PiraPolicy {
            source: source.to_string(),
        },
    };
    let transcript = vec![
        Message::user_text("hi"),
        Message {
            role: Role::User,
            blocks: vec![
                policy("~/agent/modules/RESEARCH_POLICY.md"),
                policy("~/agent/modules/CODING_STYLE.md"),
                Block::ToolResult {
                    call_id: "e".to_string(),
                    output: "echo".to_string(),
                    is_error: false,
                    provenance: Provenance::Observation,
                },
            ],
        },
        Message {
            role: Role::User,
            blocks: vec![policy("~/agent/modules/RESEARCH_POLICY.md")],
        },
    ];

    assert_eq!(
        active_policies(&transcript),
        vec![
            "~/agent/modules/RESEARCH_POLICY.md".to_string(),
            "~/agent/modules/CODING_STYLE.md".to_string(),
        ]
    );
}

#[test]
fn session_ids_are_16_hex_characters_and_distinct() {
    let a = SessionId::new(Path::new("/work/keel"));
    let b = SessionId::new(Path::new("/work/keel"));

    assert_eq!(a.as_str().len(), 16);
    assert!(a.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a, b, "two sessions in one process must not collide");
    assert_eq!(THREAD_ID_ENV, "PIRA_CTX_THREAD_ID");
}
