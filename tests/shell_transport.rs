//! Argument-transport check for the shell tool
//! (docs/dogfood/shell_contract/SHELL_CONTRACT_AB.md, stage 1).
//!
//! Fixed argv payloads with spaces, nested quotes, and backslashes are sent
//! through the real path Keel uses for ordinary commands, `wrap_command` →
//! `pira_ctx … -- python …` → `run_process`, and the child prints what it
//! received. These tests need `pira_ctx` and `python` on PATH, so they are
//! ignored by default and run explicitly on a machine with PIRA:
//!
//!     cargo test --test shell_transport -- --ignored --nocapture
//!
//! They decide one thing only: whether a legal argument arrives unchanged.
//! If any of the first three fails, the transport is at fault and the
//! description experiment does not start.

use std::time::Duration;

use keel::session::THREAD_ID_ENV;
use keel::shell::{run_process, wrap_command, Effect, ShellRequest};

/// The argument list every payload is checked against. Each element exercises
/// one thing: a space, embedded double quotes, a backslash, an apostrophe, an
/// `=` with a space, an empty string, and a trailing backslash (the classic
/// Windows quoting trap).
const PAYLOAD: [&str; 7] = [
    "a b",
    "c\"d\"e",
    "e\\f",
    "it's",
    "--flag=x y",
    "",
    "trailing\\",
];

const ECHO_ARGV: &str = "import sys, json; print(json.dumps(sys.argv[1:]))";

fn request(argv: Vec<&str>, mode: Option<&str>) -> ShellRequest {
    ShellRequest {
        argv: argv.into_iter().map(str::to_string).collect(),
        intent: "Transport check: print the received arguments".to_string(),
        effect: Effect::ReadOnly,
        safety_review: None,
        mode: mode.map(str::to_string),
        interest: None,
        workdir: None,
        timeout_seconds: None,
    }
}

fn run(request: &ShellRequest) -> keel::tool::ToolResult {
    let command = wrap_command(request);
    run_process(
        &command,
        &std::env::temp_dir(),
        &[(THREAD_ID_ENV, "transport-check")],
        Some(Duration::from_secs(60)),
    )
}

fn expected() -> Vec<String> {
    PAYLOAD.iter().map(|part| part.to_string()).collect()
}

/// The argument list the child printed: the first line of the observation
/// that parses as a JSON array of strings. In default mode the synopsis
/// prefixes the line with `L1 stdout:` and may append a space; the JSON
/// text itself is compared as data, not as text, since Python's
/// `json.dumps` and serde differ in spacing.
fn received(observation: &str) -> Option<Vec<String>> {
    observation.lines().find_map(|line| {
        let start = line.find('[')?;
        let end = line.rfind(']')?;
        serde_json::from_str::<Vec<String>>(&line[start..=end]).ok()
    })
}

#[test]
#[ignore = "needs pira_ctx and python on PATH; run on the lab machine"]
fn payload_arrives_unchanged_in_exact_mode() {
    let mut argv = vec!["python", "-c", ECHO_ARGV];
    argv.extend(PAYLOAD);
    let result = run(&request(argv, Some("exact")));
    println!("exact mode observation:\n{}", result.output);
    assert!(!result.is_error, "exact-mode run failed: {}", result.output);
    assert_eq!(
        received(&result.output),
        Some(expected()),
        "argv changed in transit (exact mode)"
    );
}

#[test]
#[ignore = "needs pira_ctx and python on PATH; run on the lab machine"]
fn payload_arrives_unchanged_in_default_mode() {
    let mut argv = vec!["python", "-c", ECHO_ARGV];
    argv.extend(PAYLOAD);
    let result = run(&request(argv, None));
    println!("default mode observation:\n{}", result.output);
    assert!(
        !result.is_error,
        "default-mode run failed: {}",
        result.output
    );
    assert_eq!(
        received(&result.output),
        Some(expected()),
        "argv changed in transit or was not shown by the synopsis (default mode)"
    );
}

#[test]
#[ignore = "needs pira_ctx and python on PATH; run on the lab machine"]
fn program_text_with_quotes_and_backslashes_arrives_unchanged() {
    // The -c program itself carries both quote kinds, a raw backslash, and a
    // newline: the shape the model used at L2-R2 calls 70 and 72.
    let program = "print(\"a b\", 'c\"d', r\"e\\f\")\nprint('second line')";
    let result = run(&request(vec!["python", "-c", program], Some("exact")));
    println!("program observation:\n{}", result.output);
    assert!(!result.is_error, "run failed: {}", result.output);
    let lines: Vec<&str> = result.output.lines().collect();
    assert_eq!(lines.first().copied(), Some("a b c\"d e\\f"));
    assert_eq!(lines.get(1).copied(), Some("second line"));
}

#[test]
#[ignore = "needs pira_ctx and python on PATH; run on the lab machine"]
fn compound_statement_after_semicolon_is_pythons_error_not_transports() {
    // The shape of L2-R2 call 80: a `def` after `;` on one line. Python must
    // reject it itself; the argument reaches Python intact.
    let program = "from datetime import datetime; T=datetime(2026,9,1); def job(n): return n";
    let result = run(&request(vec!["python", "-c", program], Some("exact")));
    println!("compound-statement observation:\n{}", result.output);
    assert!(result.is_error, "Python accepted an invalid one-liner");
    assert!(
        result.output.contains("SyntaxError") && result.output.contains("def job"),
        "the error must be Python's syntax error at the def, showing the argument arrived intact: {}",
        result.output
    );
}

#[cfg(windows)]
#[test]
#[ignore = "needs pira_ctx and python on PATH; run on the lab machine"]
fn cmd_slash_c_reparses_the_element_under_cmd_rules() {
    // The shape of L2-R2 calls 39, 54, 92: an explicit shell, with the whole
    // program as one element that contains double quotes. This is not a
    // transport check; it records what the shell does with the element, so
    // the description can state it. Whatever happens is printed, not asserted.
    let element = "python -c \"import sys; print('through cmd')\"";
    let result = run(&request(vec!["cmd", "/C", element], Some("exact")));
    println!(
        "cmd /C observation (is_error={}):\n{}",
        result.is_error, result.output
    );
}
