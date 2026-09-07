//! Tests for the PIRA policy loader tool (PLAN.md §5.5, invariant 6).

mod common;

use common::{SyntheticPira, CODING_STYLE_BYTES};
use keel::loader::{PolicyLoader, TOOL_NAME};
use keel::message::Provenance;
use keel::tool::Tool;
use serde_json::json;

fn loader(pira: &SyntheticPira) -> PolicyLoader {
    let install = pira.install();
    let policy = install.load_policy().unwrap();
    PolicyLoader::new(install, policy)
}

#[test]
fn spec_offers_exactly_the_declared_names() {
    let pira = SyntheticPira::new("loader-spec");
    let spec = loader(&pira).spec();

    assert_eq!(spec.name, TOOL_NAME);
    assert_eq!(
        spec.input_schema["properties"]["name"]["enum"],
        json!(["user_profile", "coding", "writing"])
    );
    assert_eq!(spec.input_schema["required"], json!(["name"]));
}

#[test]
fn a_declared_name_loads_exact_bytes_with_policy_provenance() {
    let pira = SyntheticPira::new("loader-read");

    let result = loader(&pira).execute(&json!({ "name": "coding" }));

    assert!(!result.is_error);
    assert_eq!(result.output, CODING_STYLE_BYTES);
    assert_eq!(
        result.provenance,
        Provenance::PiraPolicy {
            source: "~/agent/modules/CODING_STYLE.md".to_string()
        }
    );
}

#[test]
fn undeclared_names_and_bad_input_are_error_observations() {
    let pira = SyntheticPira::new("loader-bad");
    let mut tool = loader(&pira);

    let undeclared = tool.execute(&json!({ "name": "AGENTS" }));
    assert!(undeclared.is_error);
    assert_eq!(undeclared.provenance, Provenance::Observation);
    assert!(undeclared.output.contains("not a policy source"));

    let traversal = tool.execute(&json!({ "name": "../USER.md" }));
    assert!(traversal.is_error);

    let malformed = tool.execute(&json!({ "module": "coding" }));
    assert!(malformed.is_error);
    assert!(malformed.output.contains("'name'"));
}
