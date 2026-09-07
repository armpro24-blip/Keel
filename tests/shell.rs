//! Tests for the shell tool (PLAN.md §5.4; invariants 3–4).
//!
//! `wrap_command` is tested as a pure function. `run_process` is tested with
//! a plain program so no PIRA installation is needed; the composition of the
//! two is exercised on machines with PIRA by the smoke protocol.

use std::time::Duration;

use keel::session::THREAD_ID_ENV;
use keel::shell::{
    is_pira_internal_tool, run_process, wrap_command, ShellRequest, MAX_INTENT_BYTES,
};
use serde_json::json;

fn request(argv: &[&str]) -> ShellRequest {
    ShellRequest {
        argv: argv.iter().map(|part| part.to_string()).collect(),
        intent: "Inspect repository status".to_string(),
        mode: None,
        interest: None,
        workdir: None,
        timeout_seconds: None,
    }
}

/// A command that prints its arguments' shell-expanded text on this platform.
fn echo_argv(script: &str) -> Vec<String> {
    if cfg!(windows) {
        vec!["cmd".to_string(), "/C".to_string(), script.to_string()]
    } else {
        vec!["sh".to_string(), "-c".to_string(), script.to_string()]
    }
}

#[test]
fn internal_tools_are_recognized_by_basename_on_any_path_or_extension() {
    for program in [
        "pira_ctx",
        "pira_nav.exe",
        "PIRA_DEC.EXE",
        "C:\\Users\\x\\AppData\\Local\\PIRA\\bin\\pira_ctx.exe",
        "/usr/local/bin/pira_svg_check",
        "./pira_nav",
    ] {
        assert!(is_pira_internal_tool(program), "{program}");
    }
    for program in [
        "pira_ctxx",
        "not_pira_ctx",
        "pira_ctx.sh",
        "bash",
        "",
        "pira",
    ] {
        assert!(!is_pira_internal_tool(program), "{program}");
    }
}

#[test]
fn ordinary_commands_are_wrapped_in_pira_ctx_with_the_model_choices() {
    assert_eq!(
        wrap_command(&request(&["git", "status", "--short"])),
        vec![
            "pira_ctx",
            "--intent",
            "Inspect repository status",
            "--",
            "git",
            "status",
            "--short"
        ]
    );

    let mut with_choices = request(&["cargo", "test"]);
    with_choices.mode = Some("check".to_string());
    with_choices.interest = Some("(?i)error".to_string());
    assert_eq!(
        wrap_command(&with_choices),
        vec![
            "pira_ctx",
            "check",
            "--intent",
            "Inspect repository status",
            "--interest",
            "(?i)error",
            "--",
            "cargo",
            "test"
        ]
    );
}

#[test]
fn internal_tools_run_directly_and_are_never_double_wrapped() {
    let direct = request(&["pira_nav", "search", "-e", "Parser", "src"]);
    assert_eq!(wrap_command(&direct), direct.argv);

    let already_wrapped = request(&["pira_ctx", "--intent", "x", "--", "git", "status"]);
    assert_eq!(wrap_command(&already_wrapped), already_wrapped.argv);

    let with_extension = request(&["C:\\PIRA\\bin\\pira_dec.exe", "list"]);
    assert_eq!(wrap_command(&with_extension), with_extension.argv);
}

#[test]
fn parse_validates_argv_intent_mode_and_timeout() {
    let good = ShellRequest::parse(&json!({
        "argv": ["git", "status"],
        "intent": "Inspect repository status",
        "mode": "check",
        "workdir": "src",
        "timeout_seconds": 30
    }))
    .unwrap();
    assert_eq!(good.argv, vec!["git", "status"]);
    assert_eq!(good.mode.as_deref(), Some("check"));
    assert_eq!(good.workdir.as_deref(), Some("src"));
    assert_eq!(good.timeout_seconds, Some(30));

    let long_intent = "x".repeat(MAX_INTENT_BYTES + 1);
    for (input, expected) in [
        (json!({ "intent": "i" }), "'argv'"),
        (json!({ "argv": [], "intent": "i" }), "first element"),
        (json!({ "argv": ["git", 1], "intent": "i" }), "string"),
        (json!({ "argv": ["git"] }), "'intent'"),
        (json!({ "argv": ["git"], "intent": "  " }), "empty"),
        (
            json!({ "argv": ["git"], "intent": "two\nlines" }),
            "single line",
        ),
        (json!({ "argv": ["git"], "intent": long_intent }), "limit"),
        (
            json!({ "argv": ["git"], "intent": "i", "mode": "loud" }),
            "'mode'",
        ),
        (
            json!({ "argv": ["git"], "intent": "i", "timeout_seconds": 0 }),
            "positive",
        ),
    ] {
        let error = ShellRequest::parse(&input).unwrap_err();
        assert!(error.contains(expected), "{input}: {error}");
    }
}

#[test]
fn run_process_passes_the_thread_id_and_workdir_and_reports_the_exit_code() {
    let temp = std::env::temp_dir();
    let command = if cfg!(windows) {
        echo_argv("echo %PIRA_CTX_THREAD_ID% & cd")
    } else {
        echo_argv("echo $PIRA_CTX_THREAD_ID; pwd")
    };

    let result = run_process(&command, &temp, &[(THREAD_ID_ENV, "thread-42")], None);

    assert!(!result.is_error, "{}", result.output);
    assert!(result.output.starts_with("thread-42"), "{}", result.output);
    assert!(result.output.ends_with("[exit 0]"), "{}", result.output);
    let reported_dir = result.output.lines().nth(1).unwrap_or_default();
    assert!(
        std::path::Path::new(reported_dir.trim()).exists(),
        "workdir line: {reported_dir}"
    );
}

#[test]
fn run_process_reports_stderr_and_non_zero_exit_as_an_error() {
    let command = if cfg!(windows) {
        echo_argv("echo oops 1>&2 & exit 3")
    } else {
        echo_argv("echo oops >&2; exit 3")
    };

    let result = run_process(&command, &std::env::temp_dir(), &[], None);

    assert!(result.is_error);
    assert!(
        result.output.contains("[stderr]\noops"),
        "{}",
        result.output
    );
    assert!(result.output.ends_with("[exit 3]"), "{}", result.output);
}

#[test]
fn run_process_kills_a_command_at_the_deadline() {
    let command = if cfg!(windows) {
        echo_argv("ping -n 30 127.0.0.1 > nul")
    } else {
        echo_argv("sleep 30")
    };

    let result = run_process(
        &command,
        &std::env::temp_dir(),
        &[],
        Some(Duration::from_millis(300)),
    );

    assert!(result.is_error);
    assert!(result.output.contains("[killed after"), "{}", result.output);
}

#[test]
fn run_process_reports_a_missing_program_as_an_error() {
    let result = run_process(
        &["definitely-not-a-program-keel".to_string()],
        &std::env::temp_dir(),
        &[],
        None,
    );

    assert!(result.is_error);
    assert!(
        result.output.starts_with("cannot start"),
        "{}",
        result.output
    );
}
