//! The shell tool: PIRA master's `pira_ctx` rule as a runtime invariant
//! (PLAN.md §5.4).
//!
//! PIRA master: "Wrap every shell/exec invocation in `pira_ctx`, except PIRA
//! internal-tool invocations and commands that only load PIRA modules."
//! Module loading has its own tool (`loader`), so here the rule is exact:
//! every command runs through `pira_ctx` unless its program is a PIRA
//! internal tool. The model keeps the choices PIRA gives it (`mode`,
//! `interest`, `intent`); Keel only guarantees the wrapping happens.
//!
//! Commands are `argv` arrays. Keel invents no shell semantics: a model that
//! wants a shell asks for one explicitly (`["bash", "-lc", "..."]`).
//!
//! Output is not bounded here. `pira_ctx` bounds and retains what it wraps;
//! the internal tools bound their own output. Keel adds nothing on top.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::message::ToolSpec;
use crate::session::THREAD_ID_ENV;
use crate::tool::{Tool, ToolResult};

pub const TOOL_NAME: &str = "shell";

/// Programs that run directly, never inside `pira_ctx`.
pub const PIRA_INTERNAL_TOOLS: [&str; 4] = ["pira_ctx", "pira_dec", "pira_nav", "pira_svg_check"];

/// PIRA's bound for `--intent`: one line, at most this many UTF-8 bytes.
pub const MAX_INTENT_BYTES: usize = 256;

const MODES: [&str; 4] = ["auto", "check", "capture", "exact"];

/// Shell operators that are never meant as a program's own argument. Keel
/// interprets no shell syntax; it refuses these as standalone `argv` elements
/// so a misfire (`echo hello > file` printing `hello > file`, exit 0) becomes
/// a first-turn correction instead of a silent no-op. Evidence:
/// `docs/evidence/M2C_SMOKE_2026-09-07.md`.
const SHELL_OPERATORS: [&str; 11] = [
    "|", "||", "&&", ";", ">", ">>", "<", "<<", "2>", "2>>", "&>",
];

/// How to get a shell on this platform, for the tool description and for
/// the operator error message. Built-ins of `cmd` (`dir`, `type`, `copy`) are
/// not programs and need `cmd /C`.
fn shell_hint() -> &'static str {
    if cfg!(windows) {
        "request one explicitly: [\"powershell\",\"-Command\",\"...\"] or [\"cmd\",\"/C\",\"...\"]; \
         cmd built-ins such as dir, type, copy are not programs"
    } else {
        "request one explicitly: [\"sh\",\"-c\",\"...\"] or [\"bash\",\"-lc\",\"...\"]"
    }
}

/// A validated request from the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRequest {
    pub argv: Vec<String>,
    pub intent: String,
    /// The model's judgment of this command as issued (PLAN.md §3): Keel
    /// never infers it from `argv`.
    pub effect: Effect,
    /// The model's pre-execution review, PIRA's Full-Permission Behavior
    /// applied by the model. Keel validates presence and order only.
    pub safety_review: Option<String>,
    pub mode: Option<String>,
    pub interest: Option<String>,
    pub workdir: Option<String>,
    pub timeout_seconds: Option<u64>,
}

/// Whether a command, as issued, changes file, repository, tool, user, or
/// system state. Declared by the model; the declaration is logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    ReadOnly,
    StateChanging,
}

impl Effect {
    pub const NAMES: [&'static str; 2] = ["read_only", "state_changing"];

    pub fn parse(name: &str) -> Option<Effect> {
        match name {
            "read_only" => Some(Effect::ReadOnly),
            "state_changing" => Some(Effect::StateChanging),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Effect::ReadOnly => "read_only",
            Effect::StateChanging => "state_changing",
        }
    }
}

impl ShellRequest {
    /// The model's review with surrounding whitespace removed; `None` when
    /// absent or blank. Presence is all Keel checks.
    pub fn review(&self) -> Option<&str> {
        self.safety_review
            .as_deref()
            .map(str::trim)
            .filter(|review| !review.is_empty())
    }

    /// Validate the tool input. Every failure is a message the model can act on.
    pub fn parse(input: &Value) -> Result<ShellRequest, String> {
        let argv = input
            .get("argv")
            .and_then(Value::as_array)
            .ok_or("input needs an array field 'argv'")?;
        let argv: Vec<String> = argv
            .iter()
            .map(|item| item.as_str().map(str::to_string))
            .collect::<Option<_>>()
            .ok_or("every element of 'argv' must be a string")?;
        if argv.is_empty() {
            return Err("'argv' must contain the program as its first element".to_string());
        }
        if let Some(operator) = argv[1..]
            .iter()
            .find(|part| SHELL_OPERATORS.contains(&part.as_str()))
        {
            return Err(format!(
                "'argv' contains the shell operator '{operator}' as a separate argument, but argv \
                 runs without a shell, so it would be passed literally to {}; to use redirection \
                 or pipes, {}",
                argv[0],
                shell_hint()
            ));
        }

        let intent = input
            .get("intent")
            .and_then(Value::as_str)
            .ok_or("input needs a string field 'intent'")?
            .to_string();
        if intent.trim().is_empty() {
            return Err("'intent' must not be empty".to_string());
        }
        if intent.contains('\n') || intent.contains('\r') {
            return Err("'intent' must be a single line".to_string());
        }
        if intent.len() > MAX_INTENT_BYTES {
            return Err(format!(
                "'intent' is {} bytes; the limit is {MAX_INTENT_BYTES}",
                intent.len()
            ));
        }

        let optional_string = |key: &str| -> Result<Option<String>, String> {
            match input.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::String(text)) => Ok(Some(text.clone())),
                Some(_) => Err(format!("'{key}' must be a string")),
            }
        };
        let mode = optional_string("mode")?;
        if let Some(mode) = &mode {
            if !MODES.contains(&mode.as_str()) {
                return Err(format!("'mode' must be one of {}", MODES.join(", ")));
            }
        }
        let interest = optional_string("interest")?;
        let workdir = optional_string("workdir")?;
        let timeout_seconds = match input.get("timeout_seconds") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                value
                    .as_u64()
                    .filter(|seconds| *seconds > 0)
                    .ok_or("'timeout_seconds' must be a positive integer")?,
            ),
        };
        let effect = input
            .get("effect")
            .and_then(Value::as_str)
            .and_then(Effect::parse)
            .ok_or_else(|| {
                format!(
                    "input needs 'effect', one of {}: your judgment of whether this command changes state",
                    Effect::NAMES.join(", ")
                )
            })?;
        let safety_review = optional_string("safety_review")?;

        Ok(ShellRequest {
            argv,
            intent,
            effect,
            safety_review,
            mode,
            interest,
            workdir,
            timeout_seconds,
        })
    }
}

/// Whether `program` names a PIRA internal tool, by its basename. Directory
/// prefixes and a `.exe` suffix are ignored so that `pira_ctx`,
/// `C:\...\pira_ctx.exe`, and `./pira_nav` are all recognized and never
/// wrapped in a second `pira_ctx`. Both `/` and `\` count as separators on
/// every platform: the model may write Windows paths, and no PIRA tool is
/// ever named with a backslash.
pub fn is_pira_internal_tool(program: &str) -> bool {
    let basename = program.rsplit(['/', '\\']).next().unwrap_or(program);
    let stem = basename
        .strip_suffix(".exe")
        .or_else(|| basename.strip_suffix(".EXE"))
        .unwrap_or(basename);
    PIRA_INTERNAL_TOOLS
        .iter()
        .any(|tool| stem.eq_ignore_ascii_case(tool))
}

/// The argv Keel actually runs: the request unchanged for a PIRA internal
/// tool, otherwise `pira_ctx [MODE] --intent INTENT [--interest RE] -- argv`.
pub fn wrap_command(request: &ShellRequest) -> Vec<String> {
    if is_pira_internal_tool(&request.argv[0]) {
        return request.argv.clone();
    }
    let mut command = vec!["pira_ctx".to_string()];
    if let Some(mode) = &request.mode {
        command.push(mode.clone());
    }
    command.push("--intent".to_string());
    command.push(request.intent.clone());
    if let Some(interest) = &request.interest {
        command.push("--interest".to_string());
        command.push(interest.clone());
    }
    command.push("--".to_string());
    command.extend(request.argv.iter().cloned());
    command
}

/// Run `command` and turn what happened into one observation.
///
/// The output is stdout, then stderr under a `[stderr]` marker when
/// non-empty, then `[exit N]`. A non-zero exit is an error observation.
/// With a timeout the child is killed at the deadline; the grandchild that
/// `pira_ctx` started is not tracked (PIRA: known ceiling; a process group
/// would be the upgrade path).
pub fn run_process(
    command: &[String],
    workdir: &Path,
    env: &[(&str, &str)],
    timeout: Option<Duration>,
) -> ToolResult {
    let mut child = match Command::new(&command[0])
        .args(&command[1..])
        .current_dir(workdir)
        .envs(env.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return ToolResult::error(format!("cannot start {}: {error}", command[0]));
        }
    };

    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    // Readers report through channels so that after a kill Keel can stop
    // waiting: a grandchild that inherited the pipes may keep them open.
    let (stdout_tx, stdout_rx) = mpsc::channel();
    let (stderr_tx, stderr_rx) = mpsc::channel();
    thread::spawn(move || stdout_tx.send(read_all(stdout)));
    thread::spawn(move || stderr_tx.send(read_all(stderr)));

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(error) => {
                return ToolResult::error(format!("cannot wait for {}: {error}", command[0]))
            }
        }
        if timeout.is_some_and(|limit| started.elapsed() >= limit) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(20));
    };
    // After a normal exit the readers finish when the pipes close; a process
    // the command left behind can delay that, exactly as it would for any host.
    let (stdout, stderr) = if status.is_some() {
        (
            stdout_rx.recv().unwrap_or_default(),
            stderr_rx.recv().unwrap_or_default(),
        )
    } else {
        let grace = Duration::from_millis(500);
        let partial = |receiver: mpsc::Receiver<String>| {
            receiver.recv_timeout(grace).unwrap_or_else(|_| {
                "[output unavailable: a process started by the command still holds the pipe]\n"
                    .to_string()
            })
        };
        (partial(stdout_rx), partial(stderr_rx))
    };

    let mut output = stdout;
    if !stderr.is_empty() {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str("[stderr]\n");
        output.push_str(&stderr);
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    match status {
        Some(status) => {
            let code = status.code();
            output.push_str(&format!(
                "[exit {}]",
                code.map_or("signal".to_string(), |code| code.to_string())
            ));
            if code == Some(0) {
                ToolResult::ok(output)
            } else {
                ToolResult::error(output)
            }
        }
        None => {
            output.push_str(&format!(
                "[killed after {} s timeout]",
                timeout.map_or(0, |limit| limit.as_secs())
            ));
            ToolResult::error(output)
        }
    }
}

fn read_all(mut source: impl Read) -> String {
    let mut bytes = Vec::new();
    let read_error = source.read_to_end(&mut bytes).err();
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if let Some(error) = read_error {
        text.push_str(&format!("\n[read error: {error}]\n"));
    }
    text
}

/// The tool the model sees. It never decides whether a command may run; the
/// PermissionEngine does that before `execute` is reached.
pub struct ShellTool {
    workspace_root: PathBuf,
    session_id: String,
}

impl ShellTool {
    pub fn new(workspace_root: PathBuf, session_id: String) -> ShellTool {
        ShellTool {
            workspace_root,
            session_id,
        }
    }
}

/// The directory a request runs in: `workdir` relative to the workspace root
/// (an absolute `workdir` stands on its own), or the root itself.
pub fn resolve_workdir(workspace_root: &Path, request: &ShellRequest) -> PathBuf {
    match &request.workdir {
        Some(workdir) => workspace_root.join(workdir),
        None => workspace_root.to_path_buf(),
    }
}

/// One line a person can read before approving: arguments containing
/// whitespace or quotes are double-quoted so word boundaries are unambiguous.
pub fn display_command(command: &[String]) -> String {
    command
        .iter()
        .map(|part| {
            if part.is_empty() || part.chars().any(|c| c.is_whitespace() || c == '"') {
                format!("\"{}\"", part.replace('"', "\\\""))
            } else {
                part.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl Tool for ShellTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: TOOL_NAME.to_string(),
            description: format!(
                "Run a program with arguments. argv is executed directly, with no shell: \
                 argv[0] is the program and every other element is one argument, delivered \
                 to the program exactly as written. Do not add quotes that only a shell would \
                 remove (write [\"python\", \"-c\", \"print('hi')\"], not \
                 [\"python\", \"-c\", \"\\\"print('hi')\\\"\"]); quotes, spaces, backslashes, and \
                 newlines that belong to the argument's content stay in it. Redirection, pipes, \
                 and && are shell features: as standalone argv elements they are rejected; they \
                 work only inside the command element of an explicitly invoked shell. If you \
                 need a shell, {}; the element after its command flag is then parsed by that \
                 shell under its own quoting rules, not by Keel. workdir sets the child \
                 process's current working directory. Relative paths and module imports are \
                 then resolved according to that program's own rules. Provide the actual \
                 command in argv and its purpose in the top-level intent field. For ordinary \
                 commands, Keel automatically runs argv through pira_ctx; do not wrap ordinary \
                 commands in pira_ctx yourself. Invoke a PIRA internal tool directly only when \
                 that tool itself is the intended command. Returns stdout, stderr, and the exit \
                 code. In full-permission/no-approval mode a state_changing command needs a \
                 safety_review before it runs.",
                shell_hint()
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "argv": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1,
                        "description": "Program followed by its arguments."
                    },
                    "intent": {
                        "type": "string",
                        "description": "One line, at most 256 UTF-8 bytes: prospective action + target + purpose. Keel passes it to pira_ctx --intent; do not put it in argv."
                    },
                    "mode": {
                        "type": "string",
                        "enum": MODES,
                        "description": "pira_ctx mode; omit for auto."
                    },
                    "interest": {
                        "type": "string",
                        "description": "Regex whose matching output lines must dominate the pira_ctx synopsis."
                    },
                    "workdir": {
                        "type": "string",
                        "description": "Working directory relative to the workspace root; default is the root. Directories outside the workspace need the user's confirmation."
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Kill the command after this many seconds. Omit to wait."
                    },
                    "effect": {
                        "type": "string",
                        "enum": Effect::NAMES,
                        "description": "Your judgment of this command as issued: read_only if it changes no file, repository, tool, user, or system state; otherwise state_changing."
                    },
                    "safety_review": {
                        "type": "string",
                        "description": "Required when effect is state_changing in full-permission/no-approval mode: the review PIRA's Full-Permission Behavior requires before this command. Keel shows it as 'Safety: ...' before executing."
                    }
                },
                "required": ["argv", "intent", "effect"]
            }),
        }
    }

    fn execute(&mut self, input: &Value) -> ToolResult {
        let request = match ShellRequest::parse(input) {
            Ok(request) => request,
            Err(message) => return ToolResult::error(message),
        };
        let workdir = resolve_workdir(&self.workspace_root, &request);
        if !workdir.is_dir() {
            return ToolResult::error(format!(
                "workdir does not exist or is not a directory: {}",
                workdir.display()
            ));
        }
        let command = wrap_command(&request);
        let env = [(THREAD_ID_ENV, self.session_id.as_str())];
        run_process(
            &command,
            &workdir,
            &env,
            request.timeout_seconds.map(Duration::from_secs),
        )
    }
}
