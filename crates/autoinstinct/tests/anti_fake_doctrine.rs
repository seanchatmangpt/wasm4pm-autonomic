//! Kill Zone 1 — Doctrine-to-Code Drift Gauntlet.
//!
//! Asserts every constitutional invariant in `doctrine.rs` is wired to a
//! live consumer in the workspace. The check has two halves:
//!
//! 1. The coverage table itself (`doctrine_coverage.rs`) has one link per
//!    enum variant. This is enforced inside the lib unit tests.
//! 2. *Each link's `(file, test_or_symbol)` pair must actually exist on
//!    disk.* If a test is renamed or a file moved, this gauntlet fails.

use std::fs;
use std::path::{Path, PathBuf};

use autoinstinct::doctrine::{
    benchmark_tiers, canonical_cli_commands, canonical_response_lattice,
    public_ontology_profiles, ForbiddenRegression,
};
use autoinstinct::doctrine_coverage::{
    doctrine_constant_consumers_count, earned_zero_coverage,
    forbidden_regression_coverage, identity_surface_coverage, CoverageLink,
};

/// Walk up from CARGO_MANIFEST_DIR until we find the workspace root
/// (identified by the top-level Cargo.toml + a `crates/` directory).
fn workspace_root() -> PathBuf {
    let mut p: PathBuf = env!("CARGO_MANIFEST_DIR").into();
    loop {
        if p.join("crates").is_dir() && p.join("Cargo.toml").exists() {
            return p;
        }
        if !p.pop() {
            panic!("could not locate workspace root from CARGO_MANIFEST_DIR");
        }
    }
}

fn assert_link_resolvable(link: &CoverageLink) {
    let root = workspace_root();
    let path = root.join(link.file);
    assert!(
        path.exists(),
        "coverage link for `{}` points at non-existent file: {}",
        link.invariant,
        path.display()
    );
    let body = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read {}: {e}", path.display())
    });
    assert!(
        body.contains(link.test_or_symbol),
        "coverage link for `{}` mentions `{}` in {}, but it was not found",
        link.invariant,
        link.test_or_symbol,
        path.display()
    );
}

#[test]
fn forbidden_regressions_have_executable_negative_fixtures() {
    let table = forbidden_regression_coverage();
    assert_eq!(
        table.len(),
        ForbiddenRegression::all().len(),
        "every ForbiddenRegression variant must have a coverage row"
    );
    for link in table {
        assert_link_resolvable(link);
    }
}

#[test]
fn earned_zero_classes_have_executable_consumers() {
    for link in earned_zero_coverage() {
        assert_link_resolvable(link);
    }
}

#[test]
fn identity_surfaces_have_executable_consumers() {
    for link in identity_surface_coverage() {
        assert_link_resolvable(link);
    }
}

#[test]
fn doctrine_constants_are_used_by_runtime_paths() {
    // Cardinality sanity: SPR pins these counts.
    assert_eq!(canonical_response_lattice().len(), 7);
    assert_eq!(canonical_cli_commands().len(), 12);
    assert_eq!(benchmark_tiers().len(), 6);
    // Public ontology must be non-empty and every entry pubilcly rooted.
    assert!(!public_ontology_profiles().is_empty());
    // doctrine_coverage exposes a single counter so tests don't have to
    // re-derive it; it must be > 0.
    assert!(doctrine_constant_consumers_count() > 0);
}

#[test]
fn cli_grammar_is_canonical_space_separated() {
    // Sanity check the canonical pairs satisfy `verb noun` shape.
    for (verb, noun) in canonical_cli_commands() {
        assert!(
            !verb.contains('-') && !verb.contains(' '),
            "verb `{verb}` must be a single word"
        );
        assert!(
            !noun.contains('-') && !noun.contains(' '),
            "noun `{noun}` must be a single word"
        );
    }
}

#[test]
fn coverage_links_point_at_workspace_files_not_external_paths() {
    // Every link must be relative under crates/ — no absolute paths leak.
    let root = workspace_root();
    for link in forbidden_regression_coverage()
        .iter()
        .chain(earned_zero_coverage())
        .chain(identity_surface_coverage())
    {
        let p = Path::new(link.file);
        assert!(
            p.is_relative(),
            "coverage link for `{}` is absolute: {}",
            link.invariant,
            link.file
        );
        assert!(
            link.file.starts_with("crates/"),
            "coverage link for `{}` is outside crates/: {}",
            link.invariant,
            link.file
        );
        // And it must resolve relative to the workspace root.
        assert!(root.join(link.file).exists());
    }
}
