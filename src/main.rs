//! The `keel` binary: a stdin/stdout REPL that runs PIRA over one real model,
//! plus `keel pira check` to validate the installed PIRA (PLAN.md §7, M1–M2).
//!
//! Output convention: findings and the final `state:` line go to stdout;
//! warnings, errors, trace, and approval prompts go to stderr.

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use keel::agent::AgentLoop;
use keel::cli::{parse_args, Cli, USAGE};
use keel::context::{ContextManager, HostInfo};
use keel::loader::{self, PolicyLoader};
use keel::message::{Block, Message, Provenance, Role};
use keel::openai::OpenAiChatModel;
use keel::permission::{ApprovalMode, Approver, PermissionEngine};
use keel::pira::{
    default_lock_path, inspect, Compatibility, Fingerprint, Inspection, Lock, PiraInstall,
};
use keel::session::{SessionId, THREAD_ID_ENV};
use keel::shell::ShellTool;
use keel::tool::ToolRegistry;
use keel::workspace::Workspace;

const MAX_TURNS: usize = 8;

/// Write one line to stdout. A closed pipe (`keel pira check | head -1`) is
/// the reader's choice, not a fault: stop quietly instead of panicking.
fn say(line: impl std::fmt::Display) {
    let mut stdout = io::stdout().lock();
    if let Err(error) = writeln!(stdout, "{line}") {
        if error.kind() == io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        panic!("cannot write to stdout: {error}");
    }
}

fn usage_error(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!("{USAGE}");
    std::process::exit(2)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// Read one line from stdin; `None` at end of input.
fn read_line() -> Option<String> {
    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(error) => {
            eprintln!("error: cannot read stdin: {error}");
            std::process::exit(1);
        }
    }
}

/// Asks the person at the REPL. Anything but `y` or `yes` declines, and end
/// of input declines too, so a piped session can never approve by accident.
struct StdinApprover;

impl Approver for StdinApprover {
    fn approve(&mut self, summary: &str) -> bool {
        eprintln!("approve? {summary}");
        eprint!("[y/N] ");
        let answer = read_line().unwrap_or_default();
        matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
    }
}

/// One line per block, prefixed with the role, for `--trace`.
fn describe(message: &Message) -> String {
    let role = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };
    let lines: Vec<String> = message
        .blocks
        .iter()
        .map(|block| match block {
            Block::Text(text) => format!("  text: {text}"),
            Block::ToolCall { id, name, input } => format!("  tool_call {id}: {name}({input})"),
            Block::ToolResult {
                call_id,
                output,
                is_error,
                provenance,
            } => {
                let origin = match provenance {
                    Provenance::Observation => String::new(),
                    Provenance::PiraPolicy { source } => format!(", policy={source}"),
                };
                format!("  tool_result {call_id} (error={is_error}{origin}): {output}")
            }
        })
        .collect();
    format!("[{role}]\n{}", lines.join("\n"))
}

/// Print the INCOMPATIBLE state with its reasons and return the exit code.
fn incompatible(reasons: &[String]) -> i32 {
    say("state: INCOMPATIBLE");
    for reason in reasons {
        say(format!("  {reason}"));
    }
    eprintln!("error: Keel cannot rely on this PIRA installation; fix the reasons above (this state is never locked)");
    1
}

/// Read the installation and its lock, or explain why not.
fn inspect_default() -> Result<(PiraInstall, Inspection, std::path::PathBuf), String> {
    let install = PiraInstall::default_location().map_err(|error| error.to_string())?;
    let lock_path = default_lock_path().map_err(|error| error.to_string())?;
    let inspection = inspect(&install, &lock_path).map_err(|error| error.to_string())?;
    Ok((install, inspection, lock_path))
}

/// `keel pira check [--lock]`. Exit code: 0 for VERIFIED and
/// UNVERIFIED-COMPATIBLE (the latter with a warning), 1 for INCOMPATIBLE.
fn pira_check(record_lock: bool) -> i32 {
    let (install, inspection, lock_path) = match inspect_default() {
        Ok(found) => found,
        Err(reason) => return incompatible(&[reason]),
    };
    say(format!("PIRA installation: {}", install.root().display()));
    say("verification token: present");
    say(format!(
        "policy sources: {}",
        inspection.policy.sources.len()
    ));
    for source in &inspection.policy.sources {
        say(format!("  {} -> {}", source.name, source.relative_path));
    }
    for (tool, version) in &inspection.fingerprint.tools {
        say(format!(
            "tool {tool}: {}",
            version.as_deref().unwrap_or("not found")
        ));
    }
    say(format!(
        "source commit: {}",
        inspection
            .fingerprint
            .source_commit
            .as_deref()
            .unwrap_or("unknown (not a git checkout or git unavailable)")
    ));

    match &inspection.compatibility {
        Compatibility::Verified => {
            say(format!("state: VERIFIED (matches {})", lock_path.display()));
            if record_lock {
                say("lock unchanged");
            }
            0
        }
        Compatibility::UnverifiedCompatible { drift } => {
            say("state: UNVERIFIED-COMPATIBLE");
            for item in drift {
                say(format!("  drift: {item}"));
            }
            if record_lock {
                record(&inspection.fingerprint, &lock_path)
            } else {
                eprintln!("warning: PIRA differs from the last verified state; review the drift above, then run `keel pira check --lock` to record it");
                0
            }
        }
        Compatibility::Incompatible { failures } => incompatible(failures),
    }
}

fn record(fingerprint: &Fingerprint, lock_path: &Path) -> i32 {
    match Lock::from_fingerprint(fingerprint, unix_now()).write(lock_path) {
        Ok(()) => {
            say(format!("recorded: {}", lock_path.display()));
            0
        }
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    }
}

/// The REPL. PIRA must be readable and compatible before the model is asked
/// anything; drift is a warning, INCOMPATIBLE refuses to start (PLAN.md §4.2).
fn repl(model_name: String, trace: bool, mode: ApprovalMode) {
    // Cheap configuration mistakes first, then the PIRA gate.
    let mut model = match OpenAiChatModel::from_env(model_name) {
        Ok(model) => model,
        Err(message) => usage_error(&message),
    };
    let (install, inspection, _) = match inspect_default() {
        Ok(found) => found,
        Err(reason) => {
            eprintln!("error: {reason}");
            eprintln!("run `keel pira check` for details");
            std::process::exit(1);
        }
    };
    match &inspection.compatibility {
        Compatibility::Verified => {}
        Compatibility::UnverifiedCompatible { drift } => {
            for item in drift {
                eprintln!("warning: PIRA drift: {item}");
            }
            eprintln!("warning: run `keel pira check --lock` after reviewing the drift");
        }
        Compatibility::Incompatible { failures } => {
            for failure in failures {
                eprintln!("error: {failure}");
            }
            eprintln!("error: PIRA installation is INCOMPATIBLE; run `keel pira check`");
            std::process::exit(1);
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|error| {
        eprintln!("error: cannot determine the working directory: {error}");
        std::process::exit(1);
    });
    let workspace = Workspace::detect(&cwd);
    let session = SessionId::new(workspace.root());
    // Every subprocess Keel starts inherits this, so pira_ctx groups the
    // session's commands as one thread (PLAN.md §4.3-3, invariant 4).
    std::env::set_var(THREAD_ID_ENV, session.as_str());

    let context = ContextManager::new(
        inspection.policy.agents_md.clone(),
        &HostInfo {
            cwd,
            workspace_root: workspace.root().to_path_buf(),
            approval_mode: mode.describe().to_string(),
            unix_time: unix_now(),
        },
    );

    let mut tools = ToolRegistry::new();
    tools
        .register(Box::new(PolicyLoader::new(install, inspection.policy)))
        .expect("tool names are unique");
    tools
        .register(Box::new(ShellTool::new(
            workspace.root().to_path_buf(),
            session.as_str().to_string(),
        )))
        .expect("tool names are unique");
    let mut gate = PermissionEngine::new(
        mode,
        Workspace::at(workspace.root()),
        vec![loader::TOOL_NAME.to_string()],
        Box::new(StdinApprover),
    );
    let agent = AgentLoop {
        system: context.system_instruction(),
        max_turns: MAX_TURNS,
    };
    if trace {
        eprintln!("[session] {}", session.as_str());
        eprintln!("{}", context.host_block());
    }

    // The REPL owns the transcript; the loop appends to it (PLAN.md §5.3).
    let mut transcript: Vec<Message> = Vec::new();
    loop {
        eprint!("> ");
        let Some(line) = read_line() else {
            break;
        };
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/quit" {
            break;
        }

        let before = transcript.len();
        let outcome = agent.run(&mut model, &mut tools, &mut gate, &mut transcript, input);
        if trace {
            for message in &transcript[before..] {
                eprintln!("{}", describe(message));
            }
        }
        match outcome {
            Ok(outcome) => say(&outcome.final_text),
            Err(error) => eprintln!("error: {error}"),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args, std::env::var("OPENAI_MODEL").ok()) {
        Ok(Cli::Run { model, trace, full }) => {
            let mode = if full {
                ApprovalMode::Full
            } else {
                ApprovalMode::Ask
            };
            repl(model, trace, mode)
        }
        Ok(Cli::PiraCheck { lock }) => std::process::exit(pira_check(lock)),
        Ok(Cli::Help) => say(USAGE),
        Ok(Cli::Version) => say(format!("keel {}", env!("CARGO_PKG_VERSION"))),
        Err(message) => usage_error(&message),
    }
}
