use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct PortfolioState {
    pub active_projects: usize,
    pub known_artifacts: Vec<PathBuf>,
}

pub struct PortfolioIndexer;

impl Default for PortfolioIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl PortfolioIndexer {
    pub fn new() -> Self {
        Self
    }

    /// Scan `root_dir` for active project indicators and known artifacts.
    ///
    /// Active projects are counted by discovering directories that contain at
    /// least one project manifest (`Cargo.toml`, `pyproject.toml`,
    /// `package.json`, `PROGRAM-CHARTER.md`, or `dteam.toml`).
    ///
    /// Known artifacts are well-known files present under `root_dir` that the
    /// Ralph pipeline cares about: ontology files, charter docs, maturity
    /// matrices, plan markdown files, spec configs, and receipt JSON files.
    pub fn scan(&self, root_dir: &Path) -> Result<PortfolioState> {
        let mut active_projects: usize = 0;
        let mut known_artifacts: Vec<PathBuf> = Vec::new();

        // ── 1. Well-known singleton artifacts ────────────────────────────────
        let singletons = [
            "PUBLIC-ONTOLOGIES.ttl",
            "PUBLIC-ONTOLOGIES.nt",
            "PROGRAM-CHARTER.md",
            "MATURITY-MATRIX.md",
            "dteam.toml",
            "Cargo.toml",
        ];
        for name in &singletons {
            let p = root_dir.join(name);
            if p.exists() {
                known_artifacts.push(p);
            }
        }

        // ── 2. Plans directory: each *.md file is a known artifact ──────────
        let plans_dir = root_dir.join("plans");
        if plans_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&plans_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        known_artifacts.push(path);
                    }
                }
            }
        }

        // ── 3. Ontologies directory: *.ttl / *.nt / *.rq ────────────────────
        let onto_dir = root_dir.join("ontologies");
        if onto_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&onto_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(ext, "ttl" | "nt" | "rq") {
                        known_artifacts.push(path);
                    }
                }
            }
        }

        // ── 4. Receipts: .receipts/**/*.txt ──────────────────────────────────
        let receipts_dir = root_dir.join(".receipts");
        if receipts_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&receipts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "txt").unwrap_or(false) {
                        known_artifacts.push(path);
                    }
                }
            }
        }

        // ── 5. Active projects: immediate subdirs with a project manifest ────
        // Also count root itself if it has a manifest.
        let manifest_names = [
            "Cargo.toml",
            "pyproject.toml",
            "package.json",
            "PROGRAM-CHARTER.md",
            "dteam.toml",
        ];

        let root_has_manifest = manifest_names.iter().any(|m| root_dir.join(m).exists());
        if root_has_manifest {
            active_projects += 1;
        }

        if let Ok(entries) = std::fs::read_dir(root_dir) {
            for entry in entries.flatten() {
                let child = entry.path();
                if !child.is_dir() {
                    continue;
                }
                // Skip hidden directories and well-known non-project dirs.
                let dir_name = child
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if dir_name.starts_with('.') || matches!(dir_name, "target" | "node_modules") {
                    continue;
                }
                let has_manifest = manifest_names.iter().any(|m| child.join(m).exists());
                if has_manifest {
                    active_projects += 1;
                }
            }
        }

        // Deduplicate artifacts (canonical paths, preserving insertion order).
        let mut seen = std::collections::HashSet::new();
        known_artifacts.retain(|p| {
            let key = p.to_string_lossy().to_string();
            seen.insert(key)
        });
        known_artifacts.sort();

        Ok(PortfolioState {
            active_projects,
            known_artifacts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_dir(base: &Path, name: &str) -> PathBuf {
        let p = base.join(name);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn test_scan_empty_dir_yields_zero() {
        let tmp = TempDir::new().unwrap();
        let indexer = PortfolioIndexer::new();
        let state = indexer.scan(tmp.path()).unwrap();
        assert_eq!(state.active_projects, 0, "empty dir has no projects");
        assert!(state.known_artifacts.is_empty(), "empty dir has no artifacts");
    }

    #[test]
    fn test_scan_root_with_cargo_toml_counts_one_project() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let indexer = PortfolioIndexer::new();
        let state = indexer.scan(tmp.path()).unwrap();
        assert_eq!(state.active_projects, 1);
        assert!(state.known_artifacts.iter().any(|p| p.ends_with("Cargo.toml")));
    }

    #[test]
    fn test_scan_subdirs_count_as_projects() {
        let tmp = TempDir::new().unwrap();
        let sub_a = make_dir(tmp.path(), "proj-a");
        let sub_b = make_dir(tmp.path(), "proj-b");
        fs::write(sub_a.join("Cargo.toml"), "[package]").unwrap();
        fs::write(sub_b.join("package.json"), "{}").unwrap();
        let indexer = PortfolioIndexer::new();
        let state = indexer.scan(tmp.path()).unwrap();
        assert_eq!(state.active_projects, 2);
    }

    #[test]
    fn test_scan_hidden_dirs_ignored() {
        let tmp = TempDir::new().unwrap();
        let hidden = make_dir(tmp.path(), ".git");
        fs::write(hidden.join("Cargo.toml"), "[package]").unwrap();
        let indexer = PortfolioIndexer::new();
        let state = indexer.scan(tmp.path()).unwrap();
        assert_eq!(state.active_projects, 0, "hidden dirs must be skipped");
    }

    #[test]
    fn test_scan_plans_dir_yields_artifacts() {
        let tmp = TempDir::new().unwrap();
        let plans = make_dir(tmp.path(), "plans");
        fs::write(plans.join("001-idea.md"), "# Idea").unwrap();
        fs::write(plans.join("002-idea.md"), "# Idea").unwrap();
        let indexer = PortfolioIndexer::new();
        let state = indexer.scan(tmp.path()).unwrap();
        let plan_artifacts: Vec<_> = state
            .known_artifacts
            .iter()
            .filter(|p| p.parent().and_then(|d| d.file_name()).map(|d| d == "plans").unwrap_or(false))
            .collect();
        assert_eq!(plan_artifacts.len(), 2, "both plan files should be discovered");
    }

    #[test]
    fn test_scan_no_duplicate_artifacts() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let indexer = PortfolioIndexer::new();
        let state = indexer.scan(tmp.path()).unwrap();
        let mut seen = std::collections::HashSet::new();
        for p in &state.known_artifacts {
            assert!(
                seen.insert(p.to_string_lossy().to_string()),
                "duplicate artifact: {:?}",
                p
            );
        }
    }
}
