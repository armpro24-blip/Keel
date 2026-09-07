//! Shared helpers for integration tests.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use keel::pira::{PiraInstall, VERIFICATION_TOKEN};

/// A fresh directory under the platform temp dir, removed on drop.
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> TempDir {
        let unique = format!(
            "keel-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after 1970")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("create temp dir");
        TempDir { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// `AGENTS.md` for a synthetic PIRA tree; `TOKEN` is replaced at write time.
pub const AGENTS_TEMPLATE: &str = "# PIRA AGENT INSTRUCTIONS

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

pub const CODING_STYLE_BYTES: &str = "# CODING_STYLE\r\nexact bytes with CRLF\r\n";

/// A synthetic PIRA installation in a temp dir: AGENTS.md with the token,
/// USER.md, and two modules. Never touches `~/agent`.
pub struct SyntheticPira {
    _dir: TempDir,
    pub root: PathBuf,
}

impl SyntheticPira {
    pub fn new(label: &str) -> SyntheticPira {
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
            CODING_STYLE_BYTES,
        )
        .unwrap();
        fs::write(
            root.join("modules").join("SCIENTIFIC_WRITING.md"),
            "# SCIENTIFIC_WRITING\n",
        )
        .unwrap();
        SyntheticPira { _dir: dir, root }
    }

    pub fn install(&self) -> PiraInstall {
        PiraInstall::at(&self.root)
    }
}
