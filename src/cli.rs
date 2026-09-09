//! Command-line parsing for the `keel` binary.
//!
//! Lives in the library rather than `main.rs` so the parser is a pure,
//! testable function: the environment fallback for the model name is passed
//! in instead of read here.

pub const USAGE: &str =
    "usage: keel --model NAME [--trace] [--full] [--record-wire] [--max-turns N]   (or set OPENAI_MODEL)
       keel pira check [--lock]
       keel log show FILE
       keel --help | --version
env:   OPENAI_API_KEY   required for the REPL
       OPENAI_BASE_URL  optional, default https://api.openai.com/v1
repl:  type a message and press Enter; /quit or EOF exits
trace: --trace prints every message appended by a run to stderr
turns: --max-turns N bounds the model calls spent on one user message (default
       32; 1 to 1000). The value applies to the whole session; the count
       starts again with each message. When it is exhausted the run stops
       with an error, the transcript is kept, and nothing is resent. There
       is no unlimited setting; 1000 is this version's conservative ceiling
full:  --full skips ordinary host approval and provides no sandbox. A command
       the model declares state_changing runs only after the model's own
       safety_review has been shown; Keel checks that the review is present
       and precedes execution, not its content or the model's classification.
       Use the default (ask) when that boundary is not enough. A working
       directory outside the workspace always asks.
pira:  `pira check` validates the PIRA installation at ~/agent against
       ~/.keel/pira.lock; `--lock` records the current state as verified
log:   every session is recorded under ~/.keel/sessions/<workspace>/<session>.jsonl;
       `log show FILE` renders one such file for inspection (nothing is re-run)
wire:  --record-wire is a diagnostic capture for instruction-path audits, not a
       log: it writes the complete model-visible context and every response
       body to <session>.wire.jsonl. Opt-in, local only; inspect and redact
       before sharing, never commit";

/// Model calls per user message when `--max-turns` is absent. Eight sufficed
/// for M0 tests; a real multi-step task with a couple of retries needs more
/// (`docs/evidence/M2C_SMOKE_2026-09-07.md`); L2 showed a two-feature task
/// exceeding it twice (`docs/evidence/L2_2026-09-09.md`). The default stays
/// until more runs say otherwise; the operator can authorize a larger,
/// still finite, budget explicitly.
pub const DEFAULT_MAX_TURNS: usize = 32;

/// Legal range of `--max-turns`. The upper bound is this version's
/// conservative product ceiling, not a mathematical or model-capability
/// limit; there is no unlimited setting, and moving the ceiling needs its
/// own evidence (PLAN.md §8 T22).
pub const MAX_TURNS_RANGE: std::ops::RangeInclusive<usize> = 1..=1000;

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cli {
    Run {
        model: String,
        trace: bool,
        full: bool,
        record_wire: bool,
        /// Model calls per user message; see `DEFAULT_MAX_TURNS`.
        max_turns: usize,
    },
    PiraCheck {
        lock: bool,
    },
    LogShow {
        path: String,
    },
    Help,
    Version,
}

/// Parse `args` (without the program name). `model_from_env` is the value of
/// `OPENAI_MODEL`, used when `--model` is absent.
pub fn parse_args(args: &[String], model_from_env: Option<String>) -> Result<Cli, String> {
    match args.first().map(String::as_str) {
        Some("pira") => return parse_pira_args(&args[1..]),
        Some("log") => return parse_log_args(&args[1..]),
        _ => {}
    }
    let mut model = None;
    let mut trace = false;
    let mut full = false;
    let mut record_wire = false;
    let mut max_turns = DEFAULT_MAX_TURNS;
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
            "--record-wire" => {
                record_wire = true;
                index += 1;
            }
            "--max-turns" => {
                let value = args.get(index + 1).ok_or_else(max_turns_error)?;
                max_turns = value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| MAX_TURNS_RANGE.contains(n))
                    .ok_or_else(max_turns_error)?;
                index += 2;
            }
            "-h" | "--help" => return Ok(Cli::Help),
            "--version" => return Ok(Cli::Version),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let model = model
        .or(model_from_env)
        .ok_or_else(|| "no model given: pass --model NAME or set OPENAI_MODEL".to_string())?;
    Ok(Cli::Run {
        model,
        trace,
        full,
        record_wire,
        max_turns,
    })
}

fn max_turns_error() -> String {
    format!(
        "--max-turns needs an integer from {} to {}",
        MAX_TURNS_RANGE.start(),
        MAX_TURNS_RANGE.end()
    )
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

fn parse_log_args(args: &[String]) -> Result<Cli, String> {
    match args {
        [command, path] if command == "show" => Ok(Cli::LogShow { path: path.clone() }),
        _ => Err("expected: keel log show FILE".to_string()),
    }
}
