# M2 B smoke test: PIRA identity through Keel

Purpose: observe, on a real model, that Keel puts PIRA's identity in front of
the model and that the `read_pira_policy` loader is used the way PIRA's routing
rules expect. Opt-in manual run; not part of `cargo test`. Report every output
unedited.

Expected environment: Windows lab machine, vLLM at `http://192.168.3.103:8000/v1`,
model `mistral-small-4-119b`. Commands are shown for bash; PowerShell
equivalents are noted where they differ.

## 1. Install PIRA `master` at `~/agent` with PIRA's own installer

Keel reads PIRA from `~/agent` and never modifies it. Install or update it
with PIRA's own setup script; skip Codex configuration (Codex is not needed)
and audio. Requires Git, Python 3, and network access.

If `~/agent` does not exist:

```bash
git clone https://github.com/AlgebraLoveme/PIRA.git ~/agent
```

If it already exists, it must be a clean `master` checkout; bring it up to
date without switching branches:

```bash
cd ~/agent && git status --porcelain && git branch --show-current && git pull --ff-only
```

Then, in either case:

```bash
cd ~/agent
git rev-parse HEAD
powershell.exe -ExecutionPolicy Bypass -File assets/scripts/setup_pira.ps1 --skip-codex --user-mode placeholder --audio no --legacy remove --yes
powershell.exe -ExecutionPolicy Bypass -File assets/scripts/setup_pira.ps1 --skip-codex --verify
```

The installer also installs `pira_svg_check`; Keel records it as optional.

Open a new shell afterwards so the PATH change takes effect, then confirm:

```bash
pira_ctx --version
pira_dec --version
pira_nav --version
ls ~/agent/USER.md ~/agent/AGENTS.md
```

If any step fails, stop and report the full output.

## 2. Update and test Keel

```bash
cd <the Keel checkout>
git pull --ff-only
cargo test
```

All suites must pass (expected: 49 tests across 8 suites).

## 3. Validate the PIRA installation and record the lock

```bash
cargo run -q -- pira check
cargo run -q -- pira check --lock
cargo run -q -- pira check
```

Expected: the first run reports `state: UNVERIFIED-COMPATIBLE` with
`drift: no pira.lock recorded yet`; the second records the lock; the third
reports `state: VERIFIED`. Report all three outputs.

## 4. Run the REPL with tracing

bash:

```bash
export OPENAI_BASE_URL=http://192.168.3.103:8000/v1
export OPENAI_API_KEY=dummy
cargo run -q -- --model mistral-small-4-119b --trace
```

PowerShell:

```powershell
$env:OPENAI_BASE_URL = "http://192.168.3.103:8000/v1"
$env:OPENAI_API_KEY = "dummy"
cargo run -q -- --model mistral-small-4-119b --trace
```

With `--trace`, stderr first shows `[session] <id>` and the `<keel_host>` block,
then every message each run appends. Feeding the probes through stdin in one
process is acceptable, as in the M1 run.

## 5. Probes

Enter these one at a time, in this order, in the same session:

1. `What is your name, and what is the verification token in your instructions?`
   Expects the name PIRA and the 32-digit token. This shows `AGENTS.md`
   reached the model.
2. `I am about to ask you to debug a Python function. Before doing anything else, load the PIRA modules your routing rules require for a coding task, then list the modules you loaded and stop.`
   Expects `read_pira_policy` calls for `coding` and `research` (PIRA master
   routes coding tasks to both), each answered by a `tool_result … policy=~/agent/modules/…`.
3. `Load the user_profile source and tell me whether it contains anything beyond placeholders.`
   Expects one `read_pira_policy` call with `user_profile` and an answer
   reflecting the placeholder content. Do not paste the file's content into
   the report if it has been filled in; say so instead.
4. `Use read_pira_policy to load a source named AGENTS.`
   Expects an error observation (`not a policy source declared by AGENTS.md`)
   and the model reporting that it cannot.
5. `Which approval mode is this session running in, and what does PIRA require you to print before a state-changing command in that mode?`
   Expects a reference to the full mode from the host block and to the
   `Safety:` review PIRA requires in that mode.
6. `Which PIRA modules are currently loaded in this conversation? Do not load anything new.`
   Expects the list from probes 2 and 3 with no new tool calls.
7. `/quit`

## 6. Report

Send back, unedited:

- outputs of every command in steps 1 to 3, including `git rev-parse HEAD` of
  `~/agent` and the three tool versions;
- the complete REPL output (stdout and stderr) of steps 4 to 5;
- `cargo --version`, the operating system, and the wall time of the REPL run;
- anything that surprised you, even if it looks harmless.

What the report decides: whether a 119B local model follows a 24 KB policy
text well enough to route modules through the loader, whether it treats
loaded policy as instructions and tool output as data, and how it reacts to
the full approval mode. These observations shape slice C (shell tool and
permissions).
