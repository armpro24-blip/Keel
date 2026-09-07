//! Workspace identity and boundary (PLAN.md §5.7).
//!
//! The identity rule is deliberately the same one `pira_ctx` uses so that
//! Keel and PIRA's memory layer agree on what "this workspace" means:
//! the nearest ancestor of the working directory that contains `.git`,
//! otherwise the working directory itself.
//!
//! Boundary checks are lexical: paths are made absolute and `..` segments are
//! resolved without touching the file system, so a path that does not exist
//! yet can still be classified. Symlinks are not followed.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

/// Where a path lies relative to the workspace. PIRA's rule: the workspace
/// is the default allowed scope and the platform temp directory is the only
/// standing exception; everything else needs explicit user confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathScope {
    Inside,
    Temp,
    Outside,
}

impl Workspace {
    /// Detect the workspace for `cwd` with the `pira_ctx` rule.
    pub fn detect(cwd: &Path) -> Workspace {
        let root = nearest_git_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
        Workspace {
            root: normalize(&root),
        }
    }

    /// Use `root` as the workspace without detection. Intended for tests and
    /// for hosts that already know the boundary.
    pub fn at(root: &Path) -> Workspace {
        Workspace {
            root: normalize(root),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `classify`, but for an existing path also compare its resolved form
    /// (symlinks followed) against the resolved root, and report `Outside`
    /// if either view is outside. Paths that do not exist yet are classified
    /// lexically (PLAN.md §8 T11).
    pub fn classify_resolved(&self, path: &Path) -> PathScope {
        let lexical = self.classify(path);
        if lexical == PathScope::Outside {
            return PathScope::Outside;
        }
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        match (
            std::fs::canonicalize(&absolute),
            std::fs::canonicalize(&self.root),
        ) {
            (Ok(resolved), Ok(resolved_root)) => {
                if starts_with(&resolved, &resolved_root)
                    || starts_with(&resolved, &canonical_temp_dir())
                {
                    lexical
                } else {
                    PathScope::Outside
                }
            }
            _ => lexical,
        }
    }

    /// Classify `path`. A relative path is taken relative to the root.
    pub fn classify(&self, path: &Path) -> PathScope {
        let absolute = if path.is_absolute() {
            normalize(path)
        } else {
            normalize(&self.root.join(path))
        };
        if starts_with(&absolute, &self.root) {
            PathScope::Inside
        } else if starts_with(&absolute, &normalize(&std::env::temp_dir())) {
            PathScope::Temp
        } else {
            PathScope::Outside
        }
    }
}

fn canonical_temp_dir() -> PathBuf {
    let temp = std::env::temp_dir();
    std::fs::canonicalize(&temp).unwrap_or(temp)
}

fn nearest_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Absolute path with `.` removed and `..` resolved lexically.
fn normalize(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Component-wise prefix test. Windows file systems are case-insensitive, so
/// components are compared case-insensitively there.
fn starts_with(path: &Path, prefix: &Path) -> bool {
    let mut path_components = path.components();
    for expected in prefix.components() {
        match path_components.next() {
            Some(actual) if same_component(actual, expected) => {}
            _ => return false,
        }
    }
    true
}

fn same_component(a: Component<'_>, b: Component<'_>) -> bool {
    if cfg!(windows) {
        a.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&b.as_os_str().to_string_lossy())
    } else {
        a == b
    }
}
