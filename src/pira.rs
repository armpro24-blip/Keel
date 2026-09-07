//! The installed PIRA: read it, validate it, never change it (PLAN.md §4).
//!
//! PIRA `master` lives at `~/agent`, installed and updated by PIRA's own
//! setup flow. Keel reads `AGENTS.md`, learns which files the routing table
//! declares as trusted policy sources, checks that the installation is
//! usable, and records the validated state in a lock file so later drift is
//! reported rather than silently absorbed.
//!
//! Two roles are kept apart on purpose (PLAN.md §4.2): hashes detect that
//! something changed; the contract checks decide whether Keel can still work
//! with it.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// PIRA's own marker that the policy text is in context.
pub const VERIFICATION_TOKEN: &str = "31415926535897932384626433832795";

/// The prefix PIRA `master` uses for every policy path in `AGENTS.md`.
const POLICY_PATH_PREFIX: &str = "~/agent/";

/// Tools that PIRA's own `AGENTS.md` requires. `pira_svg_check` is part of
/// the PIRA tool set but not referenced by the policy text, so it is probed
/// and recorded without being required.
pub const REQUIRED_TOOLS: [&str; 3] = ["pira_ctx", "pira_dec", "pira_nav"];
pub const OPTIONAL_TOOLS: [&str; 1] = ["pira_svg_check"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiraError(pub String);

impl fmt::Display for PiraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PiraError {}

fn home_dir() -> Result<PathBuf, PiraError> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| {
            PiraError(format!(
                "cannot locate the home directory: {variable} is not set"
            ))
        })
}

/// A PIRA checkout on disk. Keel never writes into it.
pub struct PiraInstall {
    root: PathBuf,
}

impl PiraInstall {
    /// The canonical v1 location, `~/agent` (PLAN.md §4.1).
    pub fn default_location() -> Result<PiraInstall, PiraError> {
        Ok(PiraInstall {
            root: home_dir()?.join("agent"),
        })
    }

    /// A PIRA tree at an explicit path. This exists for tests with synthetic
    /// installations; it is not a relocation feature (PLAN.md §4.1).
    pub fn at(root: &Path) -> PiraInstall {
        PiraInstall {
            root: root.to_path_buf(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read `AGENTS.md`, verify the token, and parse the routing table.
    pub fn load_policy(&self) -> Result<Policy, PiraError> {
        let agents_path = self.root.join("AGENTS.md");
        let agents_md = fs::read_to_string(&agents_path).map_err(|error| {
            PiraError(format!("cannot read {}: {error}", agents_path.display()))
        })?;
        if !agents_md.contains(VERIFICATION_TOKEN) {
            return Err(PiraError(format!(
                "{} does not contain the PIRA verification token",
                agents_path.display()
            )));
        }
        let sources = parse_routing_table(&agents_md)?;
        if sources.is_empty() {
            return Err(PiraError(
                "AGENTS.md declares no policy sources under 'Module Loading and Routing'"
                    .to_string(),
            ));
        }
        Ok(Policy { agents_md, sources })
    }

    /// Exact bytes of one declared policy source. Only names the routing
    /// table declares are loadable; this is the loader's trust boundary
    /// (PLAN.md §5.5).
    pub fn read_source(&self, policy: &Policy, name: &str) -> Result<String, PiraError> {
        let source = policy
            .sources
            .iter()
            .find(|source| source.name == name)
            .ok_or_else(|| {
                PiraError(format!(
                    "'{name}' is not a policy source declared by AGENTS.md"
                ))
            })?;
        let path = self.root.join(&source.relative_path);
        fs::read_to_string(&path)
            .map_err(|error| PiraError(format!("cannot read {}: {error}", path.display())))
    }

    /// File-level contract: every declared source exists and is readable.
    pub fn check_files(&self, policy: &Policy) -> Vec<String> {
        let mut failures = Vec::new();
        for source in &policy.sources {
            let path = self.root.join(&source.relative_path);
            if let Err(error) = fs::metadata(&path) {
                failures.push(format!(
                    "policy source '{}' at {} is missing: {error}",
                    source.name,
                    path.display()
                ));
            }
        }
        failures
    }

    /// SHA-256 of `AGENTS.md` and every declared source, keyed by relative path.
    pub fn file_hashes(&self, policy: &Policy) -> Result<BTreeMap<String, String>, PiraError> {
        let mut hashes = BTreeMap::new();
        hashes.insert(
            "AGENTS.md".to_string(),
            sha256_hex(policy.agents_md.as_bytes()),
        );
        for source in &policy.sources {
            let path = self.root.join(&source.relative_path);
            let bytes = fs::read(&path)
                .map_err(|error| PiraError(format!("cannot read {}: {error}", path.display())))?;
            hashes.insert(source.relative_path.clone(), sha256_hex(&bytes));
        }
        Ok(hashes)
    }

    /// `git rev-parse HEAD` of the checkout, when it is one and git is available.
    pub fn source_commit(&self) -> Option<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!commit.is_empty()).then_some(commit)
    }
}

/// One entry of the routing table: a name the model routes to and the file
/// it stands for, relative to the installation root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySource {
    pub name: String,
    pub relative_path: String,
}

/// The parsed policy: the verbatim `AGENTS.md` and its declared sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub agents_md: String,
    pub sources: Vec<PolicySource>,
}

/// Parse lines of the form `` - `name`: `~/agent/relative/path` … `` inside
/// the `## Module Loading and Routing` section. Anything else in the file is
/// ignored; a declared path outside `~/agent/` is a contract failure because
/// Keel has no other place to look.
pub fn parse_routing_table(agents_md: &str) -> Result<Vec<PolicySource>, PiraError> {
    let mut in_section = false;
    let mut sources = Vec::new();
    for line in agents_md.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            in_section = heading.trim() == "Module Loading and Routing";
            continue;
        }
        if !in_section {
            continue;
        }
        let Some(rest) = line.strip_prefix("- `") else {
            continue;
        };
        let Some((name, rest)) = rest.split_once('`') else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(": `") else {
            continue;
        };
        let Some((path, _)) = rest.split_once('`') else {
            continue;
        };
        let relative = path.strip_prefix(POLICY_PATH_PREFIX).ok_or_else(|| {
            PiraError(format!(
                "policy source '{name}' points outside {POLICY_PATH_PREFIX}: {path}"
            ))
        })?;
        sources.push(PolicySource {
            name: name.to_string(),
            relative_path: relative.to_string(),
        });
    }
    Ok(sources)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Run `TOOL --version` for each PIRA tool and return the reported version.
/// `None` means the tool was not found or did not answer.
pub fn probe_tools() -> BTreeMap<String, Option<String>> {
    let mut versions = BTreeMap::new();
    for tool in REQUIRED_TOOLS.iter().chain(OPTIONAL_TOOLS.iter()) {
        versions.insert(tool.to_string(), tool_version(tool));
    }
    versions
}

fn tool_version(tool: &str) -> Option<String> {
    let output = Command::new(tool).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // Tools print "NAME VERSION" on the first line.
    text.lines()
        .next()?
        .split_whitespace()
        .nth(1)
        .map(str::to_string)
}

/// Everything the contract checks look at, so the checks are a pure function
/// of this value and can be tested without a real installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    pub files: BTreeMap<String, String>,
    pub tools: BTreeMap<String, Option<String>>,
    pub source_commit: Option<String>,
}

/// Failures that make the installation unusable for Keel, independent of
/// any lock. An empty list means the contract holds.
pub fn contract_failures(
    file_failures: &[String],
    tools: &BTreeMap<String, Option<String>>,
) -> Vec<String> {
    let mut failures = file_failures.to_vec();
    for tool in REQUIRED_TOOLS {
        if tools.get(tool).and_then(Option::as_deref).is_none() {
            failures.push(format!(
                "required PIRA tool '{tool}' is not on PATH or did not report a version"
            ));
        }
    }
    failures
}

/// The recorded validated state (PLAN.md §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    pub files: BTreeMap<String, String>,
    pub tools: BTreeMap<String, String>,
    pub source_commit: Option<String>,
    pub verified_at_unix: u64,
}

const LOCK_SCHEMA_VERSION: u64 = 1;

impl Lock {
    pub fn from_fingerprint(fingerprint: &Fingerprint, verified_at_unix: u64) -> Lock {
        let tools = fingerprint
            .tools
            .iter()
            .filter_map(|(name, version)| version.clone().map(|version| (name.clone(), version)))
            .collect();
        Lock {
            files: fingerprint.files.clone(),
            tools,
            source_commit: fingerprint.source_commit.clone(),
            verified_at_unix,
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "schema_version": LOCK_SCHEMA_VERSION,
            "source_commit": self.source_commit,
            "verified_at_unix": self.verified_at_unix,
            "files": self.files,
            "tools": self.tools,
        })
    }

    pub fn from_json(value: &Value) -> Result<Lock, PiraError> {
        let schema = value.get("schema_version").and_then(Value::as_u64);
        if schema != Some(LOCK_SCHEMA_VERSION) {
            return Err(PiraError(format!(
                "unsupported pira.lock schema_version {schema:?}; expected {LOCK_SCHEMA_VERSION}"
            )));
        }
        let string_map = |key: &str| -> Result<BTreeMap<String, String>, PiraError> {
            let object = value
                .get(key)
                .and_then(Value::as_object)
                .ok_or_else(|| PiraError(format!("pira.lock is missing object '{key}'")))?;
            object
                .iter()
                .map(|(name, entry)| {
                    entry
                        .as_str()
                        .map(|text| (name.clone(), text.to_string()))
                        .ok_or_else(|| {
                            PiraError(format!("pira.lock '{key}.{name}' is not a string"))
                        })
                })
                .collect()
        };
        Ok(Lock {
            files: string_map("files")?,
            tools: string_map("tools")?,
            source_commit: value
                .get("source_commit")
                .and_then(Value::as_str)
                .map(str::to_string),
            verified_at_unix: value
                .get("verified_at_unix")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        })
    }

    pub fn read(path: &Path) -> Result<Option<Lock>, PiraError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(PiraError(format!(
                    "cannot read {}: {error}",
                    path.display()
                )))
            }
        };
        let value: Value = serde_json::from_str(&text)
            .map_err(|error| PiraError(format!("{} is not valid JSON: {error}", path.display())))?;
        Lock::from_json(&value).map(Some)
    }

    pub fn write(&self, path: &Path) -> Result<(), PiraError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                PiraError(format!("cannot create {}: {error}", parent.display()))
            })?;
        }
        let text = serde_json::to_string_pretty(&self.to_json())
            .map_err(|error| PiraError(format!("cannot serialize pira.lock: {error}")))?;
        fs::write(path, text + "\n")
            .map_err(|error| PiraError(format!("cannot write {}: {error}", path.display())))
    }
}

/// The three states of PLAN.md §4.2. `UnverifiedCompatible` with an empty
/// lock means no validated state has been recorded yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compatibility {
    Verified,
    UnverifiedCompatible { drift: Vec<String> },
    Incompatible { failures: Vec<String> },
}

/// Decide the state. Contract failures dominate; otherwise any difference
/// from the lock is drift.
pub fn compare(
    lock: Option<&Lock>,
    fingerprint: &Fingerprint,
    failures: &[String],
) -> Compatibility {
    if !failures.is_empty() {
        return Compatibility::Incompatible {
            failures: failures.to_vec(),
        };
    }
    let Some(lock) = lock else {
        return Compatibility::UnverifiedCompatible {
            drift: vec!["no pira.lock recorded yet".to_string()],
        };
    };

    let mut drift = Vec::new();
    for (path, hash) in &fingerprint.files {
        match lock.files.get(path) {
            Some(locked) if locked == hash => {}
            Some(_) => drift.push(format!("{path} changed")),
            None => drift.push(format!("{path} is new")),
        }
    }
    for path in lock.files.keys() {
        if !fingerprint.files.contains_key(path) {
            drift.push(format!("{path} is no longer declared"));
        }
    }
    for (tool, version) in &fingerprint.tools {
        match (lock.tools.get(tool), version) {
            (Some(locked), Some(current)) if locked != current => {
                drift.push(format!("{tool} {locked} -> {current}"))
            }
            (None, Some(current)) => drift.push(format!("{tool} {current} is new")),
            (Some(locked), None) => drift.push(format!("{tool} {locked} is no longer available")),
            _ => {}
        }
    }
    if lock.source_commit != fingerprint.source_commit {
        drift.push(format!(
            "source commit {} -> {}",
            lock.source_commit.as_deref().unwrap_or("unknown"),
            fingerprint.source_commit.as_deref().unwrap_or("unknown")
        ));
    }

    if drift.is_empty() {
        Compatibility::Verified
    } else {
        Compatibility::UnverifiedCompatible { drift }
    }
}

/// Where Keel keeps its own state; the lock lives here (PLAN.md §4.2).
pub fn default_lock_path() -> Result<PathBuf, PiraError> {
    Ok(home_dir()?.join(".keel").join("pira.lock"))
}
