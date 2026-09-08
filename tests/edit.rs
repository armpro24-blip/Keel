//! Tests for the `edit_file` tool (PLAN.md §5.10; invariants 18–21).

mod common;

use std::fs;

use common::TempDir;
use keel::edit::{apply, describe_change, EditFileRequest, EditFileTool};
use keel::tool::Tool;
use serde_json::json;

fn request(old: &str, new: &str) -> EditFileRequest {
    EditFileRequest {
        path: "f.txt".to_string(),
        old_text: old.to_string(),
        new_text: new.to_string(),
        safety_review: None,
    }
}

fn write(dir: &TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.path.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn parse_validates_every_field() {
    let good = EditFileRequest::parse(&json!({
        "path": "tally/cli.py",
        "old_text": "a",
        "new_text": "",
        "safety_review": "  Replaces a block.  "
    }))
    .unwrap();
    assert_eq!(good.path, "tally/cli.py");
    assert_eq!(good.new_text, "", "an empty new_text deletes the block");
    assert_eq!(good.review(), Some("Replaces a block."));

    for (input, expected) in [
        (json!({ "old_text": "a", "new_text": "b" }), "'path'"),
        (
            json!({ "path": " ", "old_text": "a", "new_text": "b" }),
            "'path' must not be empty",
        ),
        (json!({ "path": "f", "new_text": "b" }), "'old_text'"),
        (
            json!({ "path": "f", "old_text": "", "new_text": "b" }),
            "'old_text' must not be empty",
        ),
        (json!({ "path": "f", "old_text": "a" }), "'new_text'"),
        (
            json!({ "path": "f", "old_text": "a", "new_text": "a" }),
            "identical",
        ),
        (
            json!({ "path": "f", "old_text": "a", "new_text": "b", "safety_review": 1 }),
            "'safety_review'",
        ),
        (
            json!({ "path": "f", "old_text": ["a"], "new_text": "b" }),
            "'old_text'",
        ),
    ] {
        let error = EditFileRequest::parse(&input).unwrap_err();
        assert!(error.contains(expected), "{input}: {error}");
    }
}

#[test]
fn exactly_one_match_is_replaced_and_every_other_byte_is_kept() {
    let dir = TempDir::new("edit-one");
    let path = write(&dir, "f.txt", b"head\nold block\nline two\ntail\n");

    let result = apply(&path, "f.txt", &request("old block\nline two", "new block"));

    assert!(!result.is_error, "{}", result.output);
    assert_eq!(fs::read(&path).unwrap(), b"head\nnew block\ntail\n");
    assert_eq!(
        result.output,
        "replaced 1 occurrence in f.txt: 18 bytes (2 lines) -> 9 bytes (1 lines); file 29 -> 20 bytes"
    );
}

#[test]
fn crlf_files_keep_their_line_endings_and_non_utf8_bytes_survive() {
    let dir = TempDir::new("edit-crlf");
    let path = write(&dir, "f.txt", b"a\r\n\xff\xfe raw\r\nold\r\nz\r\n");

    let result = apply(&path, "f.txt", &request("old\r\n", "new\r\n"));

    assert!(!result.is_error, "{}", result.output);
    assert_eq!(
        fs::read(&path).unwrap(),
        b"a\r\n\xff\xfe raw\r\nnew\r\nz\r\n"
    );
}

#[test]
fn an_empty_new_text_deletes_the_block() {
    let dir = TempDir::new("edit-delete");
    let path = write(&dir, "f.txt", b"keep\ndrop me\nkeep\n");

    let result = apply(&path, "f.txt", &request("drop me\n", ""));

    assert!(!result.is_error, "{}", result.output);
    assert_eq!(fs::read(&path).unwrap(), b"keep\nkeep\n");
}

#[test]
fn zero_matches_fail_without_touching_the_file_and_name_a_line_ending_mismatch() {
    let dir = TempDir::new("edit-zero");
    let path = write(&dir, "f.txt", b"one\r\ntwo\r\n");

    let plain = apply(&path, "f.txt", &request("missing", "x"));
    assert!(plain.is_error);
    assert_eq!(
        plain.output,
        "old_text not found in f.txt (the file uses CRLF line endings and old_text does not)"
    );

    let lf_file = write(&dir, "g.txt", b"one\ntwo\n");
    let crlf_request = EditFileRequest {
        path: "g.txt".to_string(),
        ..request("one\r\n", "x")
    };
    let mismatch = apply(&lf_file, "g.txt", &crlf_request);
    assert_eq!(
        mismatch.output,
        "old_text not found in g.txt (old_text uses CRLF line endings and the file does not)"
    );

    assert_eq!(fs::read(&path).unwrap(), b"one\r\ntwo\r\n");
    assert_eq!(fs::read(&lf_file).unwrap(), b"one\ntwo\n");
}

#[test]
fn several_matches_fail_with_the_count_and_the_file_is_untouched() {
    let dir = TempDir::new("edit-many");
    let path = write(&dir, "f.txt", b"x = 1\nx = 1\ny\nx = 1\n");

    let result = apply(&path, "f.txt", &request("x = 1", "x = 2"));

    assert!(result.is_error);
    assert_eq!(
        result.output,
        "old_text occurs 3 times in f.txt; include more surrounding text so it occurs once"
    );
    assert_eq!(fs::read(&path).unwrap(), b"x = 1\nx = 1\ny\nx = 1\n");
}

#[test]
fn overlapping_candidates_are_counted_without_overlap() {
    let dir = TempDir::new("edit-overlap");
    let path = write(&dir, "f.txt", b"aaa\n");

    // "aa" occurs once without overlap; the replacement is exact.
    let result = apply(&path, "f.txt", &request("aa", "b"));

    assert!(!result.is_error, "{}", result.output);
    assert_eq!(fs::read(&path).unwrap(), b"ba\n");
}

#[test]
fn missing_files_and_directories_are_errors_and_nothing_is_created() {
    let dir = TempDir::new("edit-missing");

    let missing = apply(
        &dir.path.join("absent.txt"),
        "absent.txt",
        &request("a", "b"),
    );
    assert!(missing.is_error);
    assert!(
        missing.output.starts_with("no such file: absent.txt"),
        "{}",
        missing.output
    );
    assert!(
        !dir.path.join("absent.txt").exists(),
        "edit_file never creates files"
    );

    let directory = apply(&dir.path, "the-dir", &request("a", "b"));
    assert!(directory.is_error);
    assert_eq!(directory.output, "not a regular file: the-dir");
}

#[test]
fn the_tool_resolves_relative_paths_against_the_workspace_root() {
    let dir = TempDir::new("edit-tool");
    fs::create_dir(dir.path.join("sub")).unwrap();
    write(&dir, "sub/f.py", b"def f():\n    return 1\n");
    let mut tool = EditFileTool::new(dir.path.clone());

    let result = tool.execute(&json!({
        "path": "sub/f.py",
        "old_text": "    return 1\n",
        "new_text": "    return 2\n"
    }));

    assert!(!result.is_error, "{}", result.output);
    assert_eq!(
        fs::read(dir.path.join("sub/f.py")).unwrap(),
        b"def f():\n    return 2\n"
    );

    let malformed = tool.execute(&json!({ "path": "sub/f.py", "old_text": "", "new_text": "x" }));
    assert!(malformed.is_error);
    assert!(malformed.output.contains("'old_text' must not be empty"));
}

#[test]
fn the_schema_and_the_change_description_match_the_design() {
    let tool = EditFileTool::new(std::env::temp_dir());
    let spec = tool.spec();
    assert_eq!(spec.name, "edit_file");
    assert_eq!(
        spec.input_schema["required"],
        json!(["path", "old_text", "new_text"])
    );
    assert!(spec
        .description
        .contains("outside-workspace paths require host approval"));
    assert!(
        spec.input_schema["properties"].get("effect").is_none(),
        "effect is fixed by contract"
    );
    assert!(spec.input_schema["properties"]["safety_review"]["type"] == "string");

    assert_eq!(
        describe_change(&request("a\nb\nc", "z")),
        "replaces 5 bytes (3 lines) with 1 bytes (1 lines)"
    );
    assert_eq!(
        describe_change(&request("abc", "")),
        "replaces 3 bytes (1 lines) with 0 bytes (0 lines)"
    );
}
