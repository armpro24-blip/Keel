//! Command-line parsing tests.

use keel::cli::{parse_args, Cli, DEFAULT_MAX_TURNS, MAX_TURNS_RANGE};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| arg.to_string()).collect()
}

fn run(model: &str, trace: bool, full: bool) -> Cli {
    Cli::Run {
        model: model.to_string(),
        trace,
        full,
        record_wire: false,
        max_turns: DEFAULT_MAX_TURNS,
    }
}

#[test]
fn model_flag_trace_and_full() {
    assert_eq!(
        parse_args(&args(&["--model", "m", "--trace"]), None),
        Ok(run("m", true, false))
    );
    assert_eq!(
        parse_args(&args(&["--full", "--model", "m"]), None),
        Ok(run("m", false, true))
    );
    assert_eq!(
        parse_args(&args(&["--model", "m", "--record-wire"]), None),
        Ok(Cli::Run {
            model: "m".to_string(),
            trace: false,
            full: false,
            record_wire: true,
            max_turns: DEFAULT_MAX_TURNS,
        })
    );
}

#[test]
fn model_falls_back_to_the_environment_and_flag_wins() {
    assert_eq!(
        parse_args(&args(&[]), Some("env-model".to_string())),
        Ok(run("env-model", false, false))
    );
    assert_eq!(
        parse_args(&args(&["--model", "flag"]), Some("env-model".to_string())),
        Ok(run("flag", false, false))
    );
}

#[test]
fn missing_model_and_bad_arguments_are_errors() {
    assert!(parse_args(&args(&[]), None).is_err());
    assert!(parse_args(&args(&["--model"]), None).is_err());
    assert!(parse_args(&args(&["--bogus"]), None).is_err());
}

#[test]
fn help_and_version_short_circuit_without_a_model() {
    assert_eq!(parse_args(&args(&["--help"]), None), Ok(Cli::Help));
    assert_eq!(parse_args(&args(&["-h"]), None), Ok(Cli::Help));
    assert_eq!(parse_args(&args(&["--version"]), None), Ok(Cli::Version));
}

#[test]
fn log_show_subcommand() {
    assert_eq!(
        parse_args(&args(&["log", "show", "s.jsonl"]), None),
        Ok(Cli::LogShow {
            path: "s.jsonl".to_string()
        })
    );
    assert!(parse_args(&args(&["log"]), None).is_err());
    assert!(parse_args(&args(&["log", "show"]), None).is_err());
    assert!(parse_args(&args(&["log", "tail", "s.jsonl"]), None).is_err());
}

#[test]
fn pira_check_subcommand() {
    assert_eq!(
        parse_args(&args(&["pira", "check"]), None),
        Ok(Cli::PiraCheck { lock: false })
    );
    assert_eq!(
        parse_args(&args(&["pira", "check", "--lock"]), None),
        Ok(Cli::PiraCheck { lock: true })
    );
    assert!(parse_args(&args(&["pira"]), None).is_err());
    assert!(parse_args(&args(&["pira", "sync"]), None).is_err());
    assert!(parse_args(&args(&["pira", "check", "--force"]), None).is_err());
}

#[test]
fn max_turns_defaults_to_32_and_accepts_the_whole_legal_range() {
    assert_eq!(DEFAULT_MAX_TURNS, 32);
    assert_eq!(MAX_TURNS_RANGE, 1..=1000);
    let parsed = |value: &str| parse_args(&args(&["--model", "m", "--max-turns", value]), None);
    for (value, expected) in [("1", 1), ("50", 50), ("1000", 1000)] {
        match parsed(value) {
            Ok(Cli::Run { max_turns, .. }) => assert_eq!(max_turns, expected, "{value}"),
            other => panic!("{value}: {other:?}"),
        }
    }
}

#[test]
fn max_turns_outside_the_range_or_malformed_is_a_usage_error_naming_the_range() {
    for value in ["0", "1001", "-1", "abc", "", "32.0"] {
        let error = parse_args(&args(&["--model", "m", "--max-turns", value]), None).unwrap_err();
        assert_eq!(
            error, "--max-turns needs an integer from 1 to 1000",
            "{value:?}"
        );
    }
    let missing = parse_args(&args(&["--model", "m", "--max-turns"]), None).unwrap_err();
    assert_eq!(missing, "--max-turns needs an integer from 1 to 1000");
}
