//! Workspace identity and boundary tests (PLAN.md §5.7).

mod common;

use std::fs;
use std::path::Path;

use common::TempDir;
use keel::workspace::{PathScope, Workspace};

#[test]
fn nearest_git_ancestor_is_the_root() {
    let tree = TempDir::new("git");
    fs::create_dir_all(tree.path.join(".git")).unwrap();
    let nested = tree.path.join("src").join("deep");
    fs::create_dir_all(&nested).unwrap();

    let workspace = Workspace::detect(&nested);

    assert_eq!(workspace.root(), Workspace::at(&tree.path).root());
}

#[test]
fn without_git_the_cwd_is_the_root() {
    let tree = TempDir::new("nogit");
    let nested = tree.path.join("work");
    fs::create_dir_all(&nested).unwrap();

    let workspace = Workspace::detect(&nested);

    assert_eq!(workspace.root(), Workspace::at(&nested).root());
}

#[test]
fn classification_distinguishes_inside_temp_and_outside() {
    let root = Path::new(if cfg!(windows) {
        r"C:\projects\keel"
    } else {
        "/projects/keel"
    });
    let workspace = Workspace::at(root);

    assert_eq!(
        workspace.classify(Path::new("src/main.rs")),
        PathScope::Inside
    );
    assert_eq!(workspace.classify(&root.join("docs")), PathScope::Inside);
    assert_eq!(
        workspace.classify(&std::env::temp_dir().join("scratch.txt")),
        PathScope::Temp
    );
    let outside = if cfg!(windows) {
        r"C:\projects\other\file"
    } else {
        "/projects/other/file"
    };
    assert_eq!(workspace.classify(Path::new(outside)), PathScope::Outside);
}

#[test]
fn parent_segments_cannot_escape_lexically() {
    let root = Path::new(if cfg!(windows) {
        r"C:\projects\keel"
    } else {
        "/projects/keel"
    });
    let workspace = Workspace::at(root);

    assert_eq!(
        workspace.classify(Path::new("src/../../other")),
        PathScope::Outside
    );
    assert_eq!(
        workspace.classify(Path::new("src/../docs/./x")),
        PathScope::Inside
    );
}

#[test]
fn a_sibling_directory_with_the_root_as_prefix_string_is_outside() {
    let root = Path::new(if cfg!(windows) {
        r"C:\projects\keel"
    } else {
        "/projects/keel"
    });
    let workspace = Workspace::at(root);
    let sibling = if cfg!(windows) {
        r"C:\projects\keel2\file"
    } else {
        "/projects/keel2/file"
    };

    assert_eq!(workspace.classify(Path::new(sibling)), PathScope::Outside);
}

#[test]
fn resolved_classification_agrees_with_lexical_for_real_directories() {
    let tree = TempDir::new("resolved");
    let inside = tree.path.join("inside");
    fs::create_dir_all(&inside).unwrap();
    let workspace = Workspace::at(&tree.path);

    assert_eq!(workspace.classify_resolved(&inside), PathScope::Inside);
    assert_eq!(
        workspace.classify_resolved(Path::new("does-not-exist-yet")),
        PathScope::Inside
    );
    let outside = if cfg!(windows) {
        Path::new(r"C:\Windows")
    } else {
        Path::new("/usr")
    };
    assert_eq!(workspace.classify_resolved(outside), PathScope::Outside);
}
