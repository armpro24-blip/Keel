//! The `keel` binary: a stdin/stdout REPL that runs PIRA over one real model,
//! plus `keel pira check` to validate the installed PIRA (PLAN.md §7, M1–M2).
//!
//! Output convention: findings and the final `state:` line go to stdout;
//! warnings and errors go to stderr.

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use keel::agent::AgentLoop;
use keel::cli::{parse_args, Cli, USAGE};
use keel::context::{ContextManager, HostInfo};
use keel::loader::PolicyLoader;
use keel::message::{Block, Message, Provenance, Role};
use keel::openai::OpenAiChatModel;
use keel::pira::{
    default_lock_path, inspect, Compatibility, Fingerprint, Inspection, Lock, PiraInstall,
};
use keel::session::{SessionId, THREAD_ID_ENV};
use keel::tool::{EchoTool, ToolRegistry};
use keel::workspace::Workspace;

const MAX_TURNS: usize = 8;

/// Until the PermissionEngine arrives (M2 slice C) Keel executes every tool
/// call without asking, and PIRA must be told so it applies its
/// full-permission rules.
const APPROVAL_MODE: &str = "full (Keel asks for no approvals and provides no sandbox)";

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
    println!("state: INCOMPATIBLE");
    for reason in reasons {
        println!("  {reason}");
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
    println!("PIRA installation: {}", install.root().display());
    println!("verification token: present");
    println!("policy sources: {}", inspection.policy.sources.len());
    for source in &inspection.policy.sources {
        println!("  {} -> {}", source.name, source.relative_path);
    }
    for (tool, version) in &inspection.fingerprint.tools {
        println!("tool {tool}: {}", version.as_deref().unwrap_or("not found"));
    }
    println!(
        "source commit: {}",
        inspection
            .fingerprint
            .source_commit
            .as_deref()
            .unwrap_or("unknown (not a git checkout or git unavailable)")
    );

    match &inspection.compatibility {
        Compatibility::Verified => {
            println!("state: VERIFIED (matches {})", lock_path.display());
            if record_lock {
                println!("lock unchanged");
            }
            0
        }
        Compatibility::UnverifiedCompatible { drift } => {
            println!("state: UNVERIFIED-COMPATIBLE");
            for item in drift {
                println!("  drift: {item}");
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
            println!("recorded: {}", lock_path.display());
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
fn repl(model_name: String, trace: bool) {
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

    let mut model = match OpenAiChatModel::from_env(model_name) {
        Ok(model) => model,
        Err(message) => usage_error(&message),
    };

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
            approval_mode: APPROVAL_MODE.to_string(),
            unix_time: unix_now(),
        },
    );

    let mut tools = ToolRegistry::new();
    tools
        .register(Box::new(PolicyLoader::new(install, inspection.policy)))
        .expect("tool names are unique");
    tools
        .register(Box::new(EchoTool))
        .expect("tool names are unique");
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
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("> ");
        stdout.flush().expect("stdout is writable");
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("error: cannot read stdin: {error}");
                std::process::exit(1);
            }
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/quit" {
            break;
        }

        let before = transcript.len();
        let outcome = agent.run(&mut model, &mut tools, &mut transcript, input);
        if trace {
            for message in &transcript[before..] {
                eprintln!("{}", describe(message));
            }
        }
        match outcome {
            Ok(outcome) => println!("{}", outcome.final_text),
            Err(error) => eprintln!("error: {error}"),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args, std::env::var("OPENAI_MODEL").ok()) {
        Ok(Cli::Run { model, trace }) => repl(model, trace),
        Ok(Cli::PiraCheck { lock }) => std::process::exit(pira_check(lock)),
        Ok(Cli::Help) => println!("{USAGE}"),
        Ok(Cli::Version) => println!("keel {}", env!("CARGO_PKG_VERSION")),
        Err(message) => usage_error(&message),
    }
}
