//! Load and represent an anti-fake evidence bundle.
//!
//! The bundle is the directory written by
//! `ainst run gauntlet --mode anti-fake --evidence`. Every required file
//! must be present; missing files are a load error, not a soft warning.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::scorecard::Scorecard;

/// Files that must exist in every evidence bundle.
pub const REQUIRED_FILES: &[&str] = &[
    "scorecard.json",
    "git.txt",
    "toolchain.txt",
    "anti_fake_doctrine.out",
    "anti_fake_causal.out",
    "anti_fake_ocel.out",
    "anti_fake_perf.out",
    "anti_fake_packs.out",
    "anti_fake_master.out",
    "ccog.out",
    "autoinstinct.out",
];

/// Failures encountered while loading an evidence bundle.
#[derive(Debug, Error)]
pub enum EvidenceLoadError {
    /// A required evidence file is missing from the bundle.
    #[error("missing required evidence file: {0}")]
    Missing(String),
    /// I/O failure reading an evidence file.
    #[error("read {path}: {source}")]
    Io {
        /// Offending path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// `scorecard.json` failed to parse into [`Scorecard`].
    #[error("parse scorecard.json: {0}")]
    Scorecard(String),
}

/// In-memory representation of a loaded evidence bundle.
#[derive(Clone, Debug)]
pub struct EvidenceBundle {
    /// Root directory of the bundle.
    pub root: PathBuf,
    /// Parsed scorecard.
    pub scorecard: Scorecard,
    /// `git.txt` contents.
    pub git_txt: String,
    /// `toolchain.txt` contents.
    pub toolchain_txt: String,
    /// File-name → contents for every loaded file (including the
    /// scorecard JSON source and the .out files). Lookups use the
    /// short file name only — no path traversal.
    pub outputs: BTreeMap<String, String>,
}

impl EvidenceBundle {
    /// Load every required file from `root`.
    ///
    /// # Errors
    ///
    /// Returns `EvidenceLoadError` if a required file is missing,
    /// unreadable, or `scorecard.json` fails to parse.
    pub fn load(root: &Path) -> Result<Self, EvidenceLoadError> {
        let mut outputs = BTreeMap::new();
        for &name in REQUIRED_FILES {
            let path = root.join(name);
            if !path.exists() {
                return Err(EvidenceLoadError::Missing(name.to_string()));
            }
            let contents =
                std::fs::read_to_string(&path).map_err(|e| EvidenceLoadError::Io {
                    path: path.display().to_string(),
                    source: e,
                })?;
            outputs.insert(name.to_string(), contents);
        }

        let scorecard_src = outputs
            .get("scorecard.json")
            .expect("scorecard.json present after loop");
        let scorecard: Scorecard = serde_json::from_str(scorecard_src)
            .map_err(|e| EvidenceLoadError::Scorecard(e.to_string()))?;

        let git_txt = outputs.get("git.txt").cloned().unwrap_or_default();
        let toolchain_txt = outputs
            .get("toolchain.txt")
            .cloned()
            .unwrap_or_default();

        Ok(Self {
            root: root.to_path_buf(),
            scorecard,
            git_txt,
            toolchain_txt,
            outputs,
        })
    }

    /// Look up a file's contents. Returns `None` for unknown names.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.outputs.get(name).map(String::as_str)
    }

    /// Concatenate every loaded file into a single body for stdin
    /// transport to Gemini. Files are tagged with `=== filename ===`
    /// banners so the model can attribute snippets back to a source.
    #[must_use]
    pub fn concat_for_prompt(&self) -> String {
        let mut out = String::new();
        for (name, body) in &self.outputs {
            out.push_str("=== ");
            out.push_str(name);
            out.push_str(" ===\n");
            out.push_str(body);
            if !body.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_minimal_bundle(root: &Path, scorecard: &Scorecard) {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(
            root.join("scorecard.json"),
            scorecard.to_json().unwrap(),
        )
        .unwrap();
        std::fs::write(root.join("git.txt"), "branch: x\ncommit: abc\n").unwrap();
        std::fs::write(root.join("toolchain.txt"), "rustc 1.0.0\n").unwrap();
        for f in [
            "anti_fake_doctrine.out",
            "anti_fake_causal.out",
            "anti_fake_ocel.out",
            "anti_fake_perf.out",
            "anti_fake_packs.out",
            "anti_fake_master.out",
            "ccog.out",
            "autoinstinct.out",
        ] {
            std::fs::write(root.join(f), format!("test result: ok. for {f}\n")).unwrap();
        }
    }

    #[test]
    fn load_succeeds_for_complete_bundle() {
        let dir = std::env::temp_dir().join("ainst-evidence-load-ok");
        let _ = std::fs::remove_dir_all(&dir);
        let card = crate::scorecard::all_true_scorecard();
        write_minimal_bundle(&dir, &card);
        let b = EvidenceBundle::load(&dir).unwrap();
        assert!(b.scorecard.overall_pass);
        assert!(b.get("anti_fake_master.out").unwrap().contains("anti_fake_master.out"));
        assert!(b.concat_for_prompt().contains("=== scorecard.json ==="));
    }

    #[test]
    fn load_rejects_missing_required_file() {
        let dir = std::env::temp_dir().join("ainst-evidence-load-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Only scorecard.json — every other required file is missing.
        std::fs::write(
            dir.join("scorecard.json"),
            crate::scorecard::all_true_scorecard().to_json().unwrap(),
        )
        .unwrap();
        match EvidenceBundle::load(&dir) {
            Err(EvidenceLoadError::Missing(name)) => {
                assert!(REQUIRED_FILES.contains(&name.as_str()));
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn load_rejects_corrupt_scorecard() {
        let dir = std::env::temp_dir().join("ainst-evidence-load-bad-json");
        let _ = std::fs::remove_dir_all(&dir);
        let card = crate::scorecard::all_true_scorecard();
        write_minimal_bundle(&dir, &card);
        std::fs::write(dir.join("scorecard.json"), "{ not json").unwrap();
        match EvidenceBundle::load(&dir) {
            Err(EvidenceLoadError::Scorecard(_)) => {}
            other => panic!("expected Scorecard parse error, got {other:?}"),
        }
    }
}
