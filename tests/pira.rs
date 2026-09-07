//! Tests for reading and validating a PIRA installation and its lock
//! (PLAN.md §4, §5.5). They use a synthetic PIRA tree, never `~/agent`.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use common::TempDir;

use keel::pira::{
    compare, contract_failures, parse_routing_table, sha256_hex, Compatibility, Fingerprint, Lock,
    PiraInstall, PolicySource, VERIFICATION_TOKEN,
};

struct SyntheticPira {
    _dir: TempDir,
    root: PathBuf,
}

const AGENTS_TEMPLATE: &str = "# PIRA AGENT INSTRUCTIONS

## Verification Token
TOKEN

## Module Loading and Routing
Read on-demand PIRA instruction files exactly.

Load on demand (explicit or inferred):
- `user_profile`: `~/agent/USER.md` when user background matters.
- `coding`: `~/agent/modules/CODING_STYLE.md` for implementation.
- not a source line
- `writing`: `~/agent/modules/SCIENTIFIC_WRITING.md` for prose.

### Constraints
- Edit instruction files only on explicit user request.

## Tool Selection
- `pira_ctx`: `~/agent/not/a/policy/source` this line is outside the section.
";

impl SyntheticPira {
    fn new(label: &str) -> SyntheticPira {
        let dir = TempDir::new(&format!("pira-{label}"));
        let root = dir.path.clone();
        fs::create_dir_all(root.join("modules")).unwrap();
        fs::write(
            root.join("AGENTS.md"),
            AGENTS_TEMPLATE.replace("TOKEN", VERIFICATION_TOKEN),
        )
        .unwrap();
        fs::write(root.join("USER.md"), "# USER\n- fill manually\n").unwrap();
        fs::write(
            root.join("modules").join("CODING_STYLE.md"),
            "# CODING_STYLE\r\nexact bytes with CRLF\r\n",
        )
        .unwrap();
        fs::write(
            root.join("modules").join("SCIENTIFIC_WRITING.md"),
            "# SCIENTIFIC_WRITING\n",
        )
        .unwrap();
        SyntheticPira { _dir: dir, root }
    }

    fn install(&self) -> PiraInstall {
        PiraInstall::at(&self.root)
    }
}

fn tools_all_present() -> BTreeMap<String, Option<String>> {
    [
        ("pira_ctx", Some("1.8.0")),
        ("pira_dec", Some("0.6.0")),
        ("pira_nav", Some("0.17.0")),
        ("pira_svg_check", Some("0.1.1")),
    ]
    .into_iter()
    .map(|(name, version)| (name.to_string(), version.map(str::to_string)))
    .collect()
}

#[test]
fn routing_table_yields_only_declared_sources_inside_the_section() {
    let sources =
        parse_routing_table(&AGENTS_TEMPLATE.replace("TOKEN", VERIFICATION_TOKEN)).unwrap();

    assert_eq!(
        sources,
        vec![
            PolicySource {
                name: "user_profile".to_string(),
                relative_path: "USER.md".to_string(),
            },
            PolicySource {
                name: "coding".to_string(),
                relative_path: "modules/CODING_STYLE.md".to_string(),
            },
            PolicySource {
                name: "writing".to_string(),
                relative_path: "modules/SCIENTIFIC_WRITING.md".to_string(),
            },
        ]
    );
}

#[test]
fn a_declared_path_outside_the_install_is_a_contract_failure() {
    let text = "## Module Loading and Routing\n- `rogue`: `/etc/passwd` never.\n";

    let error = parse_routing_table(text).unwrap_err();

    assert!(error.0.contains("rogue"), "{error}");
}

#[test]
fn a_declared_path_with_parent_or_current_segments_is_rejected() {
    for path in [
        "~/agent/../secrets",
        "~/agent/./USER.md",
        "~/agent/",
        "~/agent//x",
    ] {
        let text = format!(
            "## Module Loading and Routing
- `p`: `{path}` never.
"
        );
        assert!(
            parse_routing_table(&text).is_err(),
            "{path} must be rejected"
        );
    }
}

#[test]
fn lock_without_verified_at_is_rejected() {
    let value = serde_json::json!({
        "schema_version": 1,
        "files": {},
        "tools": {},
    });

    let error = Lock::from_json(&value).unwrap_err();

    assert!(error.0.contains("verified_at_unix"), "{error}");
}

#[test]
fn load_policy_requires_the_verification_token() {
    let pira = SyntheticPira::new("token");
    fs::write(
        pira.root.join("AGENTS.md"),
        "## Module Loading and Routing\n- `coding`: `~/agent/modules/CODING_STYLE.md`\n",
    )
    .unwrap();

    let error = pira.install().load_policy().unwrap_err();

    assert!(error.0.contains("verification token"), "{error}");
}

#[test]
fn read_source_returns_exact_bytes_and_refuses_undeclared_names() {
    let pira = SyntheticPira::new("read");
    let install = pira.install();
    let policy = install.load_policy().unwrap();

    let coding = install.read_source(&policy, "coding").unwrap();
    assert_eq!(coding, "# CODING_STYLE\r\nexact bytes with CRLF\r\n");

    let error = install.read_source(&policy, "AGENTS").unwrap_err();
    assert!(error.0.contains("not a policy source"), "{error}");

    let error = install
        .read_source(&policy, "../../etc/passwd")
        .unwrap_err();
    assert!(error.0.contains("not a policy source"), "{error}");
}

#[test]
fn missing_source_file_is_a_file_failure() {
    let pira = SyntheticPira::new("missing");
    let install = pira.install();
    let policy = install.load_policy().unwrap();
    fs::remove_file(pira.root.join("modules").join("SCIENTIFIC_WRITING.md")).unwrap();

    let failures = install.check_files(&policy);

    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("writing"), "{}", failures[0]);
}

#[test]
fn fingerprint_hashes_agents_md_and_every_source() {
    let pira = SyntheticPira::new("hash");
    let install = pira.install();
    let policy = install.load_policy().unwrap();

    let hashes = install.file_hashes(&policy).unwrap();

    assert_eq!(hashes.len(), 4);
    assert_eq!(
        hashes["modules/SCIENTIFIC_WRITING.md"],
        sha256_hex(b"# SCIENTIFIC_WRITING\n")
    );
    assert_eq!(hashes["AGENTS.md"], sha256_hex(policy.agents_md.as_bytes()));
}

#[test]
fn compare_reports_the_three_states() {
    let mut files = BTreeMap::new();
    files.insert("AGENTS.md".to_string(), "aaa".to_string());
    files.insert("modules/CODING_STYLE.md".to_string(), "bbb".to_string());
    let fingerprint = Fingerprint {
        files: files.clone(),
        tools: tools_all_present(),
        source_commit: Some("abc123".to_string()),
    };

    // No lock yet: compatible but unverified.
    assert_eq!(
        compare(None, &fingerprint, &[]),
        Compatibility::UnverifiedCompatible {
            drift: vec!["no pira.lock recorded yet".to_string()],
        }
    );

    // Lock equals fingerprint: verified.
    let lock = Lock::from_fingerprint(&fingerprint, 1);
    assert_eq!(
        compare(Some(&lock), &fingerprint, &[]),
        Compatibility::Verified
    );

    // A changed file and a tool upgrade: drift, still compatible.
    let mut drifted = fingerprint.clone();
    drifted
        .files
        .insert("modules/CODING_STYLE.md".to_string(), "ccc".to_string());
    drifted
        .tools
        .insert("pira_ctx".to_string(), Some("1.9.0".to_string()));
    match compare(Some(&lock), &drifted, &[]) {
        Compatibility::UnverifiedCompatible { drift } => {
            assert_eq!(
                drift,
                vec![
                    "modules/CODING_STYLE.md changed".to_string(),
                    "pira_ctx 1.8.0 -> 1.9.0".to_string(),
                ]
            );
        }
        other => panic!("expected drift, got {other:?}"),
    }

    // Contract failures dominate everything else.
    let failures = vec!["policy source 'coding' is missing".to_string()];
    assert_eq!(
        compare(Some(&lock), &fingerprint, &failures),
        Compatibility::Incompatible {
            failures: failures.clone()
        }
    );
}

#[test]
fn missing_required_tool_is_a_contract_failure_but_optional_is_not() {
    let mut tools = tools_all_present();
    tools.insert("pira_svg_check".to_string(), None);
    assert!(contract_failures(&[], &tools).is_empty());

    tools.insert("pira_nav".to_string(), None);
    let failures = contract_failures(&[], &tools);
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("pira_nav"), "{}", failures[0]);
}

#[test]
fn lock_round_trips_through_json_on_disk() {
    let pira = SyntheticPira::new("lock");
    let fingerprint = Fingerprint {
        files: BTreeMap::from([("AGENTS.md".to_string(), "aaa".to_string())]),
        tools: tools_all_present(),
        source_commit: None,
    };
    let lock = Lock::from_fingerprint(&fingerprint, 42);
    let path = pira.root.join("state").join("pira.lock");

    lock.write(&path).unwrap();
    let read_back = Lock::read(&path).unwrap().unwrap();

    assert_eq!(read_back, lock);
    assert_eq!(
        read_back.tools.len(),
        4,
        "only tools with a version are locked"
    );
    assert_eq!(Lock::read(&pira.root.join("absent.lock")).unwrap(), None);
}
