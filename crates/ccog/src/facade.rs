//! Field-cognition facade: orchestrates the five MVP cognitive passes.

use crate::construct8::{Construct8, Triple};
use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::operation::Operation;
use crate::receipt::Receipt;
use crate::verdict::{BoundTerms, Verdict};
use anyhow::Result;
use chrono::Utc;

/// Public facade: the five MVP operations that orchestrate the cognitive passes.
pub fn process(phrase: &str, field: &mut FieldContext) -> Result<Verdict> {
    // Phase 1: ELIZA phrase binding
    let bound_terms = crate::breeds::eliza::bind_phrase(phrase, field)?;

    // Phase 2: MYCIN evidence-gap detection
    let evidence_gap = crate::breeds::mycin::find_missing_evidence(&bound_terms, field)?;

    // Phase 3: Determine candidate operation
    let ask_action_iri = GraphIri::from_iri("https://schema.org/AskAction")?;
    let candidate_operation = Operation::new(
        ask_action_iri.clone(),
        Some("Request missing evidence".to_string()),
    );

    // Phase 4: STRIPS transition check
    let transition = crate::breeds::strips::check_transition(&ask_action_iri, field)?;

    // Phase 5: Emit receipt
    let receipt = emit_receipt(&candidate_operation, &bound_terms, field)?;

    // Assemble verdict
    let verdict = Verdict::new(
        bound_terms,
        evidence_gap,
        transition,
        candidate_operation,
        receipt,
    );

    // Materialize PROV-O delta
    if let Err(e) = emit_prov_delta(&verdict, field) {
        eprintln!("[ccog prov] emit_prov_delta failed: {}", e);
    }

    Ok(verdict)
}

/// Materialize PROV-O triples for a verdict into the field store using CONSTRUCT8.
///
/// Emits ≤ 8 triples recording the activity, inputs, output, and operation.
/// All IRIs are fully expanded (no prefix shorthand). Uses `urn:ccog:activity:` and
/// `urn:ccog:output:` URN schemes. Returns the materialized delta or an error.
fn emit_prov_delta(verdict: &Verdict, field: &mut FieldContext) -> Result<Construct8> {
    let mut delta = Construct8::empty();

    // Derive output IRI from receipt hash
    let output_iri = format!("urn:ccog:output:{}", verdict.receipt.hash);

    // Fully expanded IRI constants
    let rt = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let prov_activity = "http://www.w3.org/ns/prov#Activity";
    let prov_entity = "http://www.w3.org/ns/prov#Entity";
    let prov_used = "http://www.w3.org/ns/prov#used";
    let prov_generated = "http://www.w3.org/ns/prov#wasGeneratedBy";
    let prov_associated = "http://www.w3.org/ns/prov#wasAssociatedWith";
    let dcterms_type = "http://purl.org/dc/terms/type";
    let schema_askaction = "https://schema.org/AskAction";

    let activity_iri = verdict.receipt.activity_iri.as_str();
    let output_node = output_iri.as_str();
    let operation_iri = verdict.operation.kind_iri.as_str();

    // Triple 1: activity rdf:type prov:Activity
    delta.push(Triple::from_strings(activity_iri, rt, prov_activity));

    // Triples 2-4: activity prov:used bound_term_N (up to 3 max)
    for bound_term in verdict.bound_terms.terms.iter().take(3) {
        delta.push(Triple::from_strings(
            activity_iri,
            prov_used,
            bound_term.as_str(),
        ));
    }

    // Triple 5: output rdf:type prov:Entity
    delta.push(Triple::from_strings(output_node, rt, prov_entity));

    // Triple 6: output prov:wasGeneratedBy activity
    delta.push(Triple::from_strings(
        output_node,
        prov_generated,
        activity_iri,
    ));

    // Triple 7: activity dcterms:type schema:AskAction
    delta.push(Triple::from_strings(
        activity_iri,
        dcterms_type,
        schema_askaction,
    ));

    // Triple 8: activity prov:wasAssociatedWith operation
    delta.push(Triple::from_strings(
        activity_iri,
        prov_associated,
        operation_iri,
    ));

    // Materialize delta to graph
    delta.materialize(&field.graph)?;

    Ok(delta)
}

/// Helper: Emit a PROV receipt with BLAKE3 hash.
fn emit_receipt(
    operation: &Operation,
    bound_terms: &BoundTerms,
    _field: &mut FieldContext,
) -> Result<Receipt> {
    // Construct the activity IRI (deterministic but unique)
    let activity_iri = GraphIri::from_iri(&format!(
        "urn:ccog:activity:{}",
        blake3::hash(format!("{:?}", operation).as_bytes()).to_hex()
    ))?;

    // Hash all inputs for the receipt
    let mut hash_input = Vec::new();
    hash_input.extend_from_slice(operation.kind_iri.as_str().as_bytes());
    for term in &bound_terms.terms {
        hash_input.extend_from_slice(term.as_str().as_bytes());
    }

    let hash = Receipt::blake3_hex(&hash_input);
    let timestamp = Utc::now();

    let receipt = Receipt::new(activity_iri, hash, timestamp);

    Ok(receipt)
}

/// Process a phrase with registered knowledge hooks.
///
/// Runs the full 5-phase cognitive pass via [`process`], then fires all matching hooks
/// against the settled field state. Hook deltas are materialized to the store.
/// Returns the Verdict and all hook outcomes.
///
/// # Arguments
///
/// * `phrase` - The input phrase to process
/// * `field` - The bounded operational field with RDF graph
/// * `registry` - The hook registry containing matching rules
///
/// # Returns
///
/// Returns a tuple of `(Verdict, Vec<HookOutcome>)` or an error if the cognitive pass
/// or hook firing fails.
pub fn process_with_hooks(
    phrase: &str,
    field: &mut FieldContext,
    registry: &crate::hooks::HookRegistry,
) -> Result<(Verdict, Vec<crate::hooks::HookOutcome>)> {
    // Run the full 5-phase cognitive pass
    let verdict = process(phrase, field)?;

    // Fire all matching hooks against the settled field
    let outcomes = registry.fire_matching(field)?;

    // Materialize hook deltas to the store
    for outcome in &outcomes {
        if !outcome.delta.is_empty() {
            outcome.delta.materialize(&field.graph)?;
        }
    }

    Ok((verdict, outcomes))
}
