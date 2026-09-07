//! M1 binary: a stdin/stdout REPL over one real model (PLAN.md §7, M1).
//!
//! No PIRA, shell, permissions, or persistence yet. The only tool is `echo`,
//! kept so the real model's tool-use behavior can be observed.

use std::io::{self, BufRead, Write};

use keel::agent::AgentLoop;
use keel::message::Message;
use keel::openai::OpenAiChatModel;
use keel::tool::{EchoTool, ToolRegistry};

const USAGE: &str = "usage: keel --model NAME   (or set OPENAI_MODEL)
       keel --help | --version
env:   OPENAI_API_KEY   required
       OPENAI_BASE_URL  optional, default https://api.openai.com/v1
repl:  type a message and press Enter; /quit or EOF exits";

const SYSTEM: &str = "You are a helpful assistant. Use the echo tool when asked to echo.";
const MAX_TURNS: usize = 8;

/// What the command line asked for.
enum Cli {
    Run { model: String },
    Help,
    Version,
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    let mut model = None;
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
            "-h" | "--help" => return Ok(Cli::Help),
            "--version" => return Ok(Cli::Version),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let model = model
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .ok_or_else(|| "no model given: pass --model NAME or set OPENAI_MODEL".to_string())?;
    Ok(Cli::Run { model })
}

fn usage_error(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!("{USAGE}");
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let model_name = match parse_args(&args) {
        Ok(Cli::Run { model }) => model,
        Ok(Cli::Help) => {
            println!("{USAGE}");
            return;
        }
        Ok(Cli::Version) => {
            println!("keel {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Err(message) => usage_error(&message),
    };
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
        match agent.run(&mut model, &mut tools, &mut transcript, input) {
            Ok(outcome) => println!("{}", outcome.final_text),
            Err(error) => eprintln!("error: {error}"),
        }
    }
}
