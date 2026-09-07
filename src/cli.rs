//! Command-line parsing for the `keel` binary.
//!
//! Lives in the library rather than `main.rs` so the parser is a pure,
//! testable function: the environment fallback for the model name is passed
//! in instead of read here.

pub const USAGE: &str = "usage: keel --model NAME [--trace] [--full]   (or set OPENAI_MODEL)
       keel pira check [--lock]
       keel --help | --version
env:   OPENAI_API_KEY   required for the REPL
       OPENAI_BASE_URL  optional, default https://api.openai.com/v1
repl:  type a message and press Enter; /quit or EOF exits
trace: --trace prints every message appended by a run to stderr
full:  --full runs actions without asking (no sandbox); default asks before
       each action, and a working directory outside the workspace always asks
pira:  `pira check` validates the PIRA installation at ~/agent against
       ~/.keel/pira.lock; `--lock` records the current state as verified";

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cli {
    Run {
        model: String,
        trace: bool,
        full: bool,
    },
    PiraCheck {
        lock: bool,
    },
    Help,
    Version,
}

/// Parse `args` (without the program name). `model_from_env` is the value of
/// `OPENAI_MODEL`, used when `--model` is absent.
pub fn parse_args(args: &[String], model_from_env: Option<String>) -> Result<Cli, String> {
    if args.first().map(String::as_str) == Some("pira") {
        return parse_pira_args(&args[1..]);
    }
    let mut model = None;
    let mut trace = false;
    let mut full = false;
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
            "--full" => {
                full = true;
                index += 1;
            }
            "-h" | "--help" => return Ok(Cli::Help),
            "--version" => return Ok(Cli::Version),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let model = model
        .or(model_from_env)
        .ok_or_else(|| "no model given: pass --model NAME or set OPENAI_MODEL".to_string())?;
    Ok(Cli::Run { model, trace, full })
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
