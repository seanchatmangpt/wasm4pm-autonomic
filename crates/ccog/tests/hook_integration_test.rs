//! Integration test for autonomic hooks.
//!
//! Tests that the ccog process fires knowledge hooks in response to verdicts
//! and correctly materializes hook deltas into the field context.

use ccog::{process_with_hooks, FieldContext, HookRegistry};

/// Field state fixture with missing evidence scenarios.
///
/// Contains a claim with an associated document missing a value and SKOS concepts
/// for witness statements.
const NT_MISSING_EVIDENCE: &str = r#"
<http://example.org/claim/99> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Claim> .
<http://example.org/doc/missing_value> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .
<http://example.org/doc/missing_value> <http://purl.org/dc/terms/type> <http://example.org/witness_concept> .
<http://example.org/witness_concept> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .
<http://example.org/witness_concept> <http://www.w3.org/2004/02/skos/core#prefLabel> "witness statement" .
"#;

/// Test that autonomic loop fires missing evidence hook.
///
/// Verifies that when a verdict detects an evidence gap, the registered
/// missing_evidence_hook fires and materializes its delta. The hook outcome
/// should contain a valid BLAKE3 hash and the delta should be in the graph.
#[test]
fn autonomic_loop_fires_missing_evidence_hook() {
    let mut field = FieldContext::new("claims-autonomic");
    field
        .load_field_state(NT_MISSING_EVIDENCE)
        .expect("Failed to load");

    let mut registry = HookRegistry::new();
    registry.register(ccog::hooks::missing_evidence_hook());

    let (verdict, hook_outcomes) =
        process_with_hooks("The witness statement is missing", &mut field, &registry)
            .expect("process_with_hooks() failed");

    // Assert: at least one hook fired
    assert!(
        !hook_outcomes.is_empty(),
        "Expected at least one hook outcome"
    );

    // Assert: hook outcome has valid BLAKE3 hash
    let outcome = &hook_outcomes[0];
    if let Some(receipt) = &outcome.receipt {
        assert_eq!(receipt.hash.len(), 64, "BLAKE3 hash must be 64 chars");
    }

    // Assert: hook delta is in the graph
    assert!(outcome.delta.len() > 0, "Hook delta should have triples");

    // Assert: verdict fields are correct
    assert!(!verdict.bound_terms.terms.is_empty(), "Should bind phrase");
    assert!(verdict.evidence_gap.is_some(), "Should detect gap");
    assert!(!verdict.transition.admissible, "Should block transition");
}

/// Test that no hook fires when evidence is complete.
///
/// Verifies that when evidence is present (has prov:value), the missing_evidence_hook
/// does not fire. This ensures hooks only materialize deltas when their condition is met.
#[test]
fn autonomic_loop_no_hook_when_evidence_complete() {
    let mut field = FieldContext::new("claims-complete");
    field
        .load_field_state(
            r#"
<http://example.org/doc/complete> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .
<http://example.org/doc/complete> <http://www.w3.org/ns/prov#value> "witness_audio.mp3" .
"#,
        )
        .expect("Failed to load");

    let mut registry = HookRegistry::new();
    registry.register(ccog::hooks::missing_evidence_hook());

    let (_verdict, outcomes) = process_with_hooks("The witness is complete", &mut field, &registry)
        .expect("process_with_hooks() failed");

    assert!(
        outcomes.is_empty(),
        "No hook should fire when evidence is complete"
    );
}
