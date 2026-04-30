//! Field-cognition facade: orchestrates the five MVP cognitive passes.

use anyhow::Result;
use chrono::Utc;
use oxigraph::model::{Triple, NamedNode, Term};

use crate::breeds::{eliza, mycin, strips};
use crate::construct8::Construct8;
use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::operation::Operation;
use crate::receipt::Receipt;
use crate::verdict::{Verdict, BoundTerms};

/// Public facade: the five MVP operations that orchestrate the cognitive passes.
///
/// Process a phrase through all cognitive passes and emit a verdict.
pub fn process(phrase: &str, field: &mut FieldContext) -> Result<Verdict> {
    // Phase 1: ELIZA phrase binding
    let bound_terms = eliza::bind_phrase(phrase, field)?;

    // Phase 2: MYCIN evidence-gap detection
    let evidence_gap = mycin::find_missing_evidence(&bound_terms, field)?;

    // Phase 3: Determine candidate operation
    // For MVP: if missing evidence, suggest requesting it
    let ask_action_iri = GraphIri::from_iri("https://schema.org/AskAction")?;
    let candidate_operation = Operation::new(
        ask_action_iri.clone(),
        Some("Request missing evidence".to_string()),
    );

    // Phase 4: STRIPS transition check
    let transition = strips::check_transition(&ask_action_iri, field)?;

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

    // Materialize PROV-O delta — soft failure (log, don't abort)
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
    let output_iri = GraphIri::from_iri(&format!(
        "urn:ccog:output:{}",
        verdict.receipt.hash
    ))?;

    // Fully expanded IRI constants
    let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
    let prov_activity = NamedNode::new("http://www.w3.org/ns/prov#Activity")?;
    let prov_entity = NamedNode::new("http://www.w3.org/ns/prov#Entity")?;
    let prov_used = NamedNode::new("http://www.w3.org/ns/prov#used")?;
    let prov_generated = NamedNode::new("http://www.w3.org/ns/prov#wasGeneratedBy")?;
    let prov_associated = NamedNode::new("http://www.w3.org/ns/prov#wasAssociatedWith")?;
    let dcterms_type = NamedNode::new("http://purl.org/dc/terms/type")?;
    let schema_askaction = NamedNode::new("https://schema.org/AskAction")?;

    let activity_iri: NamedNode = verdict.receipt.activity_iri.clone().into();
    let output_node: NamedNode = output_iri.clone().into();
    let operation_iri: NamedNode = verdict.operation.kind_iri.clone().into();

    // Triple 1: activity rdf:type prov:Activity
    delta.push(Triple::new(
        activity_iri.clone(),
        rdf_type.clone(),
        Term::NamedNode(prov_activity),
    ));

    // Triples 2-4: activity prov:used bound_term_N (up to 3 max)
    for bound_term in verdict.bound_terms.terms.iter().take(3) {
        let bound_node: NamedNode = bound_term.clone().into();
        delta.push(Triple::new(
            activity_iri.clone(),
            prov_used.clone(),
            Term::NamedNode(bound_node),
        ));
    }

    // Triple 5: output rdf:type prov:Entity
    delta.push(Triple::new(
        output_node.clone(),
        rdf_type.clone(),
        Term::NamedNode(prov_entity),
    ));

    // Triple 6: output prov:wasGeneratedBy activity
    delta.push(Triple::new(
        output_node.clone(),
        prov_generated.clone(),
        Term::NamedNode(activity_iri.clone()),
    ));

    // Triple 7: activity dcterms:type schema:AskAction
    delta.push(Triple::new(
        activity_iri.clone(),
        dcterms_type.clone(),
        Term::NamedNode(schema_askaction),
    ));

    // Triple 8: activity prov:wasAssociatedWith operation
    delta.push(Triple::new(
        activity_iri,
        prov_associated,
        Term::NamedNode(operation_iri),
    ));

    // Materialize delta to graph
    let triples: Vec<Triple> = delta.iter().cloned().collect();
    field.graph.insert_triples(&triples)?;

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
        if outcome.delta.len() > 0 {
            let _ = outcome.delta.materialize(&field.graph)?;
        }
    }

    Ok((verdict, outcomes))
}
