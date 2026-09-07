//! The `keel` binary: a stdin/stdout REPL over one real model, plus
//! `keel pira check` to validate the installed PIRA (PLAN.md §7, M1–M2).
//!
//! The REPL does not load PIRA yet; the only tool is `echo`, kept so the real
//! model's tool-use behavior can be observed. `--trace` prints every message
//! a run appended so that behavior is visible.

use std::io::{self, BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use keel::agent::AgentLoop;
use keel::message::{Block, Message, Role};
use keel::openai::OpenAiChatModel;
use keel::pira::{
    compare, contract_failures, default_lock_path, probe_tools, Compatibility, Fingerprint, Lock,
    PiraInstall,
};
use keel::tool::{EchoTool, ToolRegistry};

const USAGE: &str = "usage: keel --model NAME [--trace]   (or set OPENAI_MODEL)
       keel pira check [--lock]
       keel --help | --version
env:   OPENAI_API_KEY   required for the REPL
       OPENAI_BASE_URL  optional, default https://api.openai.com/v1
repl:  type a message and press Enter; /quit or EOF exits
trace: --trace prints every message appended by a run to stderr
pira:  `pira check` validates the PIRA installation at ~/agent against
       ~/.keel/pira.lock; `--lock` records the current state as verified";

const SYSTEM: &str = "You are a helpful assistant. Use the echo tool when asked to echo.";
const MAX_TURNS: usize = 8;

/// What the command line asked for.
enum Cli {
    Run { model: String, trace: bool },
    PiraCheck { lock: bool },
    Help,
    Version,
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    if args.first().map(String::as_str) == Some("pira") {
        return parse_pira_args(&args[1..]);
    }
    let mut model = None;
    let mut trace = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--model" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--model needs a value".to_string())?;
                model = Some(value.clone());
                index += 2;
            }
            "--trace" => {
                trace = true;
                index += 1;
            }
            "-h" | "--help" => return Ok(Cli::Help),
            "--version" => return Ok(Cli::Version),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let model = model
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .ok_or_else(|| "no model given: pass --model NAME or set OPENAI_MODEL".to_string())?;
    Ok(Cli::Run { model, trace })
}

fn parse_pira_args(args: &[String]) -> Result<Cli, String> {
    match args {
        [command] if command == "check" => Ok(Cli::PiraCheck { lock: false }),
        [command, flag] if command == "check" && flag == "--lock" => {
            Ok(Cli::PiraCheck { lock: true })
        }
        _ => Err("expected: keel pira check [--lock]".to_string()),
    }
}

fn usage_error(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!("{USAGE}");
    std::process::exit(2)
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
            } => format!("  tool_result {call_id} (error={is_error}): {output}"),
        })
        .collect();
    format!("[{role}]\n{}", lines.join("\n"))
}

/// `keel pira check [--lock]`. Exit code: 0 for VERIFIED and
/// UNVERIFIED-COMPATIBLE (the latter with a warning), 1 for INCOMPATIBLE.
fn pira_check(record_lock: bool) -> i32 {
    let install = match PiraInstall::default_location() {
        Ok(install) => install,
        Err(error) => {
            eprintln!("INCOMPATIBLE: {error}");
            return 1;
        }
    };
    println!("PIRA installation: {}", install.root().display());

    let policy = match install.load_policy() {
        Ok(policy) => policy,
        Err(error) => {
            println!("state: INCOMPATIBLE");
            println!("  {error}");
            return 1;
        }
    };
    println!("verification token: present");
    println!("policy sources: {}", policy.sources.len());
    for source in &policy.sources {
        println!("  {} -> {}", source.name, source.relative_path);
    }

    let file_failures = install.check_files(&policy);
    let tools = probe_tools();
    for (tool, version) in &tools {
        match version {
            Some(version) => println!("tool {tool}: {version}"),
            None => println!("tool {tool}: not found"),
        }
    }
    let source_commit = install.source_commit();
    println!(
        "source commit: {}",
        source_commit
            .as_deref()
            .unwrap_or("unknown (not a git checkout or git unavailable)")
    );

    let failures = contract_failures(&file_failures, &tools);
    // Hashing needs every declared file; without them the contract already failed.
    let files = if file_failures.is_empty() {
        match install.file_hashes(&policy) {
            Ok(files) => files,
            Err(error) => {
                println!("state: INCOMPATIBLE");
                println!("  {error}");
                return 1;
            }
        }
    } else {
        Default::default()
    };
    let fingerprint = Fingerprint {
        files,
        tools,
        source_commit,
    };

    let lock_path = match default_lock_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("INCOMPATIBLE: {error}");
            return 1;
        }
    };
    let lock = match Lock::read(&lock_path) {
        Ok(lock) => lock,
        Err(error) => {
            println!("state: INCOMPATIBLE");
            println!("  {error}");
            return 1;
        }
    };

    match compare(lock.as_ref(), &fingerprint, &failures) {
        Compatibility::Verified => {
            println!("state: VERIFIED (matches {})", lock_path.display());
            0
        }
        Compatibility::UnverifiedCompatible { drift } => {
            println!("state: UNVERIFIED-COMPATIBLE");
            for item in &drift {
                println!("  drift: {item}");
            }
            if record_lock {
                record(&fingerprint, &lock_path)
            } else {
                eprintln!("warning: PIRA differs from the last verified state; review the drift above, then run `keel pira check --lock` to record it");
                0
            }
        }
        Compatibility::Incompatible { failures } => {
            println!("state: INCOMPATIBLE");
            for failure in &failures {
                println!("  {failure}");
            }
            eprintln!("error: Keel cannot rely on this PIRA installation; fix the failures above (this state is never locked)");
            1
        }
    }
}

fn record(fingerprint: &Fingerprint, lock_path: &std::path::Path) -> i32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    match Lock::from_fingerprint(fingerprint, now).write(lock_path) {
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

fn repl(model_name: String, trace: bool) {
    let mut model = match OpenAiChatModel::from_env(model_name) {
        Ok(model) => model,
        Err(message) => usage_error(&message),
    };

    let mut tools = ToolRegistry::new();
    tools
        .register(Box::new(EchoTool))
        .expect("tool names are unique");
    let agent = AgentLoop {
        system: SYSTEM.to_string(),
        max_turns: MAX_TURNS,
    };

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
    match parse_args(&args) {
        Ok(Cli::Run { model, trace }) => repl(model, trace),
        Ok(Cli::PiraCheck { lock }) => std::process::exit(pira_check(lock)),
        Ok(Cli::Help) => println!("{USAGE}"),
        Ok(Cli::Version) => println!("keel {}", env!("CARGO_PKG_VERSION")),
        Err(message) => usage_error(&message),
    }
}
