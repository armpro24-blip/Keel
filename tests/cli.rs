//! Command-line parsing tests.

use keel::cli::{parse_args, Cli};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| arg.to_string()).collect()
}

#[test]
fn model_flag_and_trace() {
    assert_eq!(
        parse_args(&args(&["--model", "m", "--trace"]), None),
        Ok(Cli::Run {
            model: "m".to_string(),
            trace: true
        })
    );
}

#[test]
fn model_falls_back_to_the_environment_and_flag_wins() {
    assert_eq!(
        parse_args(&args(&[]), Some("env-model".to_string())),
        Ok(Cli::Run {
            model: "env-model".to_string(),
            trace: false
        })
    );
    assert_eq!(
        parse_args(&args(&["--model", "flag"]), Some("env-model".to_string())),
        Ok(Cli::Run {
            model: "flag".to_string(),
            trace: false
        })
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
