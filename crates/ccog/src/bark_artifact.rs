//! Const-generated bark artifact (Phase 4 Stage 4 / Phase 5 Track A).
//!
//! The four built-in hooks are baked into a `'static` const slice of
//! `(name, require_mask, act_fn, emit_receipt)` tuples. Dispatch becomes a
//! linear scan over a fixed-size table — no `Vec` allocation, no
//! registration step, no plan walk. The mask comparison stays at u64-AND
//! cost; `act_fn` is a const `fn(&ClosedFieldContext) -> Result<Construct8>`
//! pointer.
//!
//! Use [`bark`] for the cheapest possible dispatch — it skips the plan walk
//! entirely. Use [`bark_kernel::BarkKernel`] when you need plan-ordered
//! dispatch with custom hooks.
//!
//! # Phase 5 Track A: decide / materialize / seal split
//!
//! [`bark`] / [`bark_table`] remain as convenience wrappers. The dispatch is
//! now factored into three independently-callable stages:
//!
//! - [`decide`] / [`decide_table`] — compute `present_mask` once and report
//!   which slots are eligible to fire as a [`BarkDecision`] bit-set.
//! - [`materialize`] / [`materialize_table`] — for each fired slot run the
//!   per-slot `act_fn` and collect the resulting `Construct8` deltas.
//! - [`seal`] / [`seal_table`] — derive a deterministic `urn:blake3:` activity
//!   IRI for each slot whose `emit_receipt` flag is set and produce a
//!   per-slot `Receipt`. Activity URNs are derived via
//!   [`Receipt::derive_urn`] over [`Receipt::canonical_material`] — no
//!   `http://example.org/...` IRIs and no `Utc::now()` in URN material.
//!
//! # Phase 5 Track D: built-in act semantics
//!
//! The placeholder act bodies (`prov:value "placeholder"`,
//! `skos:definition "derived from prefLabel"`, SHACL shapes on instances)
//! have been replaced with provenance / gap-finding deltas:
//!
//! - `act_missing_evidence` emits `schema:AskAction` activities for each
//!   `schema:DigitalDocument` lacking `prov:value` (gap-finding).
//! - `act_phrase_binding` emits `prov:wasInformedBy` linking the labeled
//!   concept to a literal-derived `urn:blake3:` (provenance link).
//! - `act_transition_admissibility` emits `prov:Activity` + `prov:used`
//!   per typed subject (provenance, not SHACL).
//! - `act_receipt` is unchanged — already deterministic.

use crate::compiled_hook::{compute_present_mask, Predicate};
use crate::construct8::{Construct8, Triple};
use crate::hooks::HookOutcome;
use crate::receipt::Receipt;
use crate::runtime::cog8::{CollapseFn, EdgeId, NodeId};
use crate::runtime::ClosedFieldContext;
use anyhow::Result;
use chrono::Utc;
use oxigraph::model::{NamedNode, Term};

type ActFn = fn(&ClosedFieldContext) -> Result<Construct8>;

/// Static bark slot: name, mask, action, and receipt flag known at compile time.
#[derive(Clone, Copy)]
pub struct BarkSlot {
    /// Hook identifier propagated to outcomes.
    pub name: &'static str,
    /// AND-mask of canonical predicate bits required to fire.
    pub require_mask: u64,
    /// Snapshot-driven action emitted on match.
    pub act: ActFn,
    /// Whether to emit a PROV receipt with the outcome.
    pub emit_receipt: bool,
    /// Plan-node bitmask of predecessor slots that must have advanced
    /// before this slot fires. Default `0` = no predecessor constraint
    /// (preserves legacy semantics). Used by Phase 7 `decide_with_trace`
    /// to record `BarkSkipReason::PredecessorNotAdvanced`; the
    /// alloc-free `decide_table` ignores this field.
    pub predecessor_mask: u64,
}

const fn dd_present_mask() -> u64 {
    (1u64 << Predicate::DD_PRESENT) | (1u64 << Predicate::DD_MISSING_PROV_VALUE)
}

const fn pref_label_mask() -> u64 {
    1u64 << Predicate::HAS_PREF_LABEL
}

const fn rdf_type_mask() -> u64 {
    1u64 << Predicate::HAS_RDF_TYPE
}

/// Const table of the four built-in hooks. Dispatch reads this in registration order.
pub const BUILTINS: &[BarkSlot] = &[
    BarkSlot {
        name: "missing_evidence",
        require_mask: dd_present_mask(),
        act: act_missing_evidence,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "phrase_binding",
        require_mask: pref_label_mask(),
        act: act_phrase_binding,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "transition_admissibility",
        require_mask: rdf_type_mask(),
        act: act_transition_admissibility,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "receipt",
        require_mask: 0,
        act: act_receipt,
        emit_receipt: true,
        predecessor_mask: 0,
    },
];

/// Side-table of real hook check fns aligned 1:1 to `BUILTINS` by slot
/// index. Cited from `hooks.rs:654/679/702/718`. Used only by the
/// diagnostic trace path (`decide_with_trace_table`); the alloc-free
/// `decide_table` does not invoke these. The fourth slot ("receipt") is
/// `Always`-trigger / always-true check, so we pin it to `|_| true`.
type HookFn = fn(&ClosedFieldContext) -> bool;
/// Built-in hook functions for diagnostic traces.
pub const BUILTIN_HOOKS: &[(&str, HookFn)] = &[
    (
        "missing_evidence",
        crate::hooks::check_any_doc_missing_value_snap,
    ),
    (
        "phrase_binding",
        crate::hooks::check_concept_with_label_snap,
    ),
    (
        "transition_admissibility",
        crate::hooks::check_any_typed_subject_snap,
    ),
    ("receipt", |_context| true),
];

/// Decision packet emitted by [`decide`] / [`decide_table`].
///
/// `fired` is a bit-set indexed by slot position in the dispatch table —
/// `bit i` is set iff slot `i` matched its `require_mask` against the
/// snapshot's `present_mask`. `present_mask` is captured for downstream
/// consumers that want to inspect the canonical predicate bits without
/// recomputing them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BarkDecision {
    /// Bit i set iff slot i fired. Indexed by table position.
    pub fired: u64,
    /// The canonical predicate `present_mask` computed from the snapshot.
    pub present_mask: u64,
    /// Collapse function attributed to this decision.
    pub collapse_fn: Option<CollapseFn>,
    /// Selected node ID.
    pub selected_node: Option<NodeId>,
    /// Selected edge ID.
    pub selected_edge: Option<EdgeId>,
}

/// Semantic intent: emit `schema:AskAction` activities pointing at every
/// `schema:DigitalDocument` whose `prov:value` is missing. The act does not
/// fabricate evidence — it records a gap as a request for evidence.
///
/// Per-gap shape (2 triples):
///
/// ```text
/// <urn:blake3:{hash(doc_iri)}> rdf:type schema:AskAction .
/// <urn:blake3:{hash(doc_iri)}> schema:object <doc_iri> .
/// ```
///
/// At most 4 gaps are emitted to stay within the ≤8-triple `Construct8`
/// budget.
fn act_missing_evidence(context: &ClosedFieldContext) -> Result<Construct8> {
    let snap = &context.snapshot;
    let dd = NamedNode::new("https://schema.org/DigitalDocument")
        .expect("Invalid schema:DigitalDocument IRI");
    let pv = NamedNode::new("http://www.w3.org/ns/prov#value").expect("Invalid prov:value IRI");
    let rt = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let ask_action = "https://schema.org/AskAction";
    let schema_object = "https://schema.org/object";

    let mut delta = Construct8::empty();
    let mut gaps_emitted: u8 = 0;
    for d in snap.instances_of(&dd) {
        if gaps_emitted >= 4 || delta.is_full() {
            break;
        }
        if !snap.has_value_for(d, &pv) {
            let activity = format!(
                "urn:blake3:{}",
                blake3::hash(d.as_str().as_bytes()).to_hex()
            );

            let _ = delta.push(Triple::from_strings(&activity, rt, ask_action));
            let _ = delta.push(Triple::from_strings(&activity, schema_object, d.as_str()));
            gaps_emitted += 1;
        }
    }
    Ok(delta)
}

/// Semantic intent: each `(concept, prefLabel literal)` pair becomes a
/// PROV provenance link from the concept to a deterministic `urn:blake3:`
/// derived from the literal's lexical text. The act asserts where the
/// label *came from*, not what the concept means.
///
/// Per-pair shape (1 triple):
///
/// ```text
/// <concept> prov:wasInformedBy <urn:blake3:{hash(label_text)}> .
/// ```
///
/// At most 8 pairs are emitted to stay within the `Construct8` budget.
fn act_phrase_binding(context: &ClosedFieldContext) -> Result<Construct8> {
    let snap = &context.snapshot;
    let pl = NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")
        .expect("Invalid skos:prefLabel IRI");
    let was_informed_by = "http://www.w3.org/ns/prov#wasInformedBy";

    let mut delta = Construct8::empty();
    for (concept, label) in snap.pairs_with_predicate(&pl) {
        if delta.is_full() {
            break;
        }
        let label_text = match label {
            Term::Literal(lit) => lit.value().to_string(),
            Term::NamedNode(n) => n.as_str().to_string(),
            _ => continue,
        };
        let label_urn = format!(
            "urn:blake3:{}",
            blake3::hash(label_text.as_bytes()).to_hex()
        );
        let _ = delta.push(Triple::from_strings(
            concept.as_str(),
            was_informed_by,
            &label_urn,
        ));
    }
    Ok(delta)
}

/// Semantic intent: each typed subject is paired with a deterministic
/// `prov:Activity` whose `urn:blake3:` is derived from the subject IRI.
/// The activity declares it `prov:used` the subject — provenance, not
/// SHACL validation. Emitting `sh:targetClass` on instances (the previous
/// placeholder) was a category error.
///
/// Per-subject shape (2 triples):
///
/// ```text
/// <urn:blake3:{hash(subject_iri)}> rdf:type prov:Activity .
/// <urn:blake3:{hash(subject_iri)}> prov:used <subject> .
/// ```
///
/// At most 4 typed subjects are emitted to stay within the ≤8-triple
/// `Construct8` budget.
fn act_transition_admissibility(context: &ClosedFieldContext) -> Result<Construct8> {
    let snap = &context.snapshot;
    let rt = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        .expect("Invalid rdf:type IRI");
    let rt_iri = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let prov_activity = "http://www.w3.org/ns/prov#Activity";
    let prov_used = "http://www.w3.org/ns/prov#used";

    let mut delta = Construct8::empty();
    let mut emitted: u8 = 0;
    for (subj, type_term) in snap.pairs_with_predicate(&rt) {
        if emitted >= 4 {
            break;
        }
        if let Term::NamedNode(_) = type_term {
            let activity = format!(
                "urn:blake3:{}",
                blake3::hash(subj.as_str().as_bytes()).to_hex()
            );
            let _ = delta.push(Triple::from_strings(&activity, rt_iri, prov_activity));
            let _ = delta.push(Triple::from_strings(&activity, prov_used, subj.as_str()));
            emitted += 1;
        }
    }
    Ok(delta)
}

/// Semantic intent: emit a deterministic `prov:Activity` triple pair from a
/// BLAKE3-derived URN. Already correct in the previous revision; preserved
/// verbatim because the activity URN is content-addressed via BLAKE3 and
/// carries no example.org IRIs.
fn act_receipt(_context: &ClosedFieldContext) -> Result<Construct8> {
    let h = blake3::hash(b"receipt_hook");
    let activity = format!("urn:blake3:{}", h.to_hex());
    let rt = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let act = "http://www.w3.org/ns/prov#Activity";
    let assoc = "http://www.w3.org/ns/prov#wasAssociatedWith";
    let agent = "http://www.w3.org/ns/prov#Agent";

    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::from_strings(&activity, rt, act));
    let _ = delta.push(Triple::from_strings(&activity, assoc, agent));
    Ok(delta)
}

// ---------------------------------------------------------------------------
// Phase 5 Track A: decide / materialize / seal split.
// ---------------------------------------------------------------------------

/// Decide which built-in slots are eligible to fire against `context`.
///
/// Computes the canonical `present_mask` once via [`compute_present_mask`]
/// and returns a [`BarkDecision`] whose `fired` bit `i` is set iff
/// `(BUILTINS[i].require_mask & present_mask) == BUILTINS[i].require_mask`.
pub fn decide(context: &ClosedFieldContext) -> BarkDecision {
    decide_table(context, BUILTINS)
}

/// Decide eligibility for an arbitrary const slot table.
///
/// Identical semantics to [`decide`] but parameterized over `table`.
/// Tables longer than 64 slots have their tail silently truncated — the
/// decision word is a `u64` bit-set keyed on slot index.
pub fn decide_table(context: &ClosedFieldContext, table: &'static [BarkSlot]) -> BarkDecision {
    let present = compute_present_mask(&context.snapshot);
    let mut fired: u64 = 0;
    let max = table.len().min(64);
    for (i, slot) in table.iter().take(max).enumerate() {
        if (slot.require_mask & present) == slot.require_mask {
            fired |= 1u64 << (i as u64);
        }
    }
    BarkDecision {
        fired,
        present_mask: present,
        collapse_fn: None,
        selected_node: None,
        selected_edge: None,
    }
}

/// Materialize per-slot deltas for a [`BarkDecision`] over `BUILTINS`.
///
/// Returns a `Vec<Option<Construct8>>` indexed by slot position: `Some(delta)`
/// if `decision.fired` had bit `i` set, `None` otherwise. Length matches
/// `BUILTINS.len()`.
pub fn materialize(
    decision: &BarkDecision,
    context: &ClosedFieldContext,
) -> Result<Vec<Option<Construct8>>> {
    materialize_table(decision, context, BUILTINS)
}

/// Materialize per-slot deltas for an arbitrary const slot table.
///
/// Identical semantics to [`materialize`] but parameterized over `table`.
pub fn materialize_table(
    decision: &BarkDecision,
    context: &ClosedFieldContext,
    table: &'static [BarkSlot],
) -> Result<Vec<Option<Construct8>>> {
    let max = table.len().min(64);
    let mut out: Vec<Option<Construct8>> = Vec::with_capacity(table.len());
    for (i, slot) in table.iter().enumerate() {
        if i >= max {
            out.push(None);
            continue;
        }
        if decision.fired & (1u64 << (i as u64)) == 0 {
            out.push(None);
            continue;
        }
        let delta = (slot.act)(context)?;
        out.push(Some(delta));
    }
    Ok(out)
}

/// Seal per-slot receipts for `BUILTINS` over the materialized deltas.
///
/// For each slot index `i` where `decision.fired` is set AND `deltas[i]` is
/// `Some(_)` AND `BUILTINS[i].emit_receipt` is true, derives a
/// `urn:blake3:{hex}` activity IRI via
/// [`Receipt::derive_urn`] over [`Receipt::canonical_material`] and produces
/// a [`Receipt`]. All other slots produce `None`.
///
/// Plan-node is the slot index in the `BUILTINS` slice (`u16`). Polarity is
/// `1u8`. `Utc::now()` is used only as the receipt's metadata timestamp —
/// never as URN material.
pub fn seal(
    decision: &BarkDecision,
    deltas: &[Option<Construct8>],
    field_id: &str,
    prior_chain: Option<blake3::Hash>,
) -> Vec<Option<Receipt>> {
    seal_table(decision, deltas, BUILTINS, field_id, prior_chain)
}

/// Seal per-slot receipts for an arbitrary const slot table.
///
/// Identical semantics to [`seal`] but parameterized over `table`.
pub fn seal_table(
    decision: &BarkDecision,
    deltas: &[Option<Construct8>],
    table: &'static [BarkSlot],
    field_id: &str,
    prior_chain: Option<blake3::Hash>,
) -> Vec<Option<Receipt>> {
    let mut out: Vec<Option<Receipt>> = Vec::with_capacity(table.len());
    let max = table.len().min(64);
    let now = Utc::now();
    for (i, slot) in table.iter().enumerate() {
        if i >= max {
            out.push(None);
            continue;
        }
        let fired = decision.fired & (1u64 << (i as u64)) != 0;
        let delta_ref = deltas.get(i).and_then(Option::as_ref);
        match (fired, delta_ref, slot.emit_receipt) {
            (true, Some(delta), true) => {
                let delta_bytes = delta.receipt_bytes();
                let material = Receipt::canonical_material(
                    slot.name,
                    i as u16,
                    &delta_bytes,
                    field_id,
                    prior_chain,
                    1u8,
                );
                let activity_urn = Receipt::derive_urn(&material);
                let activity_iri = crate::graph::GraphIri::from_iri(&activity_urn)
                    .expect("derive_urn must produce a valid urn:blake3 IRI");
                let hash = Receipt::blake3_hex(&delta_bytes);
                out.push(Some(Receipt::new(activity_iri, hash, now)));
            }
            _ => out.push(None),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Convenience wrappers — single-pass dispatch identical to pre-Phase-5 bark().
// ---------------------------------------------------------------------------

/// Single-pass dispatch over the const built-in slot table.
///
/// Computes `present_mask` once, then for each slot fires iff
/// `(slot.require_mask & present_mask) == slot.require_mask`. No allocation
/// for the slot list — it's `&'static [BarkSlot]`. Outcomes are heap-allocated
/// only because `Construct8` stores triples; the dispatch itself is alloc-free.
pub fn bark(context: &ClosedFieldContext) -> Result<Vec<HookOutcome>> {
    bark_table(context, BUILTINS)
}

/// Dispatch over an arbitrary `&'static [BarkSlot]` table.
///
/// Same single-pass mask logic as [`bark`] but lets callers swap in a custom
/// const table (e.g., for extension experiments without changing the default).
///
/// Internally composes [`decide_table`] + [`materialize_table`] + [`seal_table`]
/// to keep the convenience wrapper byte-for-byte equivalent to calling the
/// stages explicitly. The `field_id` passed into receipt material is empty —
/// callers that need a specific field IRI should call the staged API.
pub fn bark_table(
    context: &ClosedFieldContext,
    table: &'static [BarkSlot],
) -> Result<Vec<HookOutcome>> {
    let decision = decide_table(context, table);
    let deltas = materialize_table(&decision, context, table)?;
    let receipts = seal_table(&decision, &deltas, table, "", None);

    let mut outcomes = Vec::with_capacity(table.len());
    for (i, slot) in table.iter().enumerate() {
        let fired = decision.fired & (1u64 << (i as u64)) != 0;
        if !fired {
            continue;
        }
        // SAFETY: fired ⇒ deltas[i] is Some by materialize_table contract.
        let delta = match deltas.get(i).and_then(|d| d.clone()) {
            Some(d) => d,
            None => continue,
        };
        let receipt = receipts.get(i).and_then(|r| r.clone());
        outcomes.push(HookOutcome {
            hook_name: slot.name,
            delta,
            receipt,
        });
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompiledFieldSnapshot;
    use crate::field::FieldContext;
    use crate::multimodal::{ContextBundle, PostureBundle};
    use crate::packs::TierMasks;
    use std::sync::Arc;

    fn empty_context(snap: Arc<CompiledFieldSnapshot>) -> ClosedFieldContext {
        ClosedFieldContext {
            snapshot: snap,
            posture: PostureBundle::default(),
            context: ContextBundle::default(),
            tiers: TierMasks::ZERO,
            human_burden: 0,
        }
    }

    #[test]
    fn const_table_has_four_builtins() {
        assert_eq!(BUILTINS.len(), 4);
        let names: Vec<_> = BUILTINS.iter().map(|s| s.name).collect();
        assert!(names.contains(&"missing_evidence"));
        assert!(names.contains(&"phrase_binding"));
        assert!(names.contains(&"transition_admissibility"));
        assert!(names.contains(&"receipt"));
    }

    #[test]
    fn bark_fires_receipt_on_empty_field() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let outcomes = bark(&context)?;
        let names: Vec<_> = outcomes.iter().map(|o| o.hook_name).collect();
        assert_eq!(names, vec!["receipt"]);
        Ok(())
    }

    #[test]
    fn bark_fires_full_set_on_loaded_field() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let outcomes = bark(&context)?;
        let names: Vec<_> = outcomes.iter().map(|o| o.hook_name).collect();
        assert!(names.contains(&"missing_evidence"));
        assert!(names.contains(&"phrase_binding"));
        assert!(names.contains(&"transition_admissibility"));
        assert!(names.contains(&"receipt"));
        Ok(())
    }

    #[test]
    fn bark_table_dispatches_custom_const() -> Result<()> {
        const TINY: &[BarkSlot] = &[BarkSlot {
            name: "receipt_only",
            require_mask: 0,
            act: act_receipt,
            emit_receipt: false,
            predecessor_mask: 0,
        }];
        let field = FieldContext::new("test");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let outcomes = bark_table(&context, TINY)?;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].hook_name, "receipt_only");
        assert!(outcomes[0].receipt.is_none());
        Ok(())
    }

    /// decide → materialize → seal must produce the same outcome shape
    /// (same fired hook names, same delta sizes) as the convenience [`bark`].
    #[test]
    fn decide_then_materialize_matches_bark() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);

        let decision = decide(&context);
        let deltas = materialize(&decision, &context)?;
        let receipts = seal(&decision, &deltas, "", None);

        let outcomes = bark(&context)?;
        let mut staged_names = Vec::new();
        let mut staged_lens = Vec::new();
        for (i, slot) in BUILTINS.iter().enumerate() {
            if decision.fired & (1u64 << (i as u64)) == 0 {
                continue;
            }
            staged_names.push(slot.name);
            staged_lens.push(deltas[i].as_ref().expect("fired => Some").len());
            // emit_receipt=true for all four builtins => seal must produce a Receipt
            assert!(
                receipts[i].is_some(),
                "seal must emit a Receipt for fired slot {} (emit_receipt=true)",
                slot.name
            );
        }

        let bark_names: Vec<_> = outcomes.iter().map(|o| o.hook_name).collect();
        let bark_lens: Vec<_> = outcomes.iter().map(|o| o.delta.len()).collect();
        assert_eq!(
            staged_names, bark_names,
            "names must match between staged and convenience APIs"
        );
        assert_eq!(
            staged_lens, bark_lens,
            "delta sizes must match between staged and convenience APIs"
        );
        Ok(())
    }

    /// Receipts produced by `seal` must carry an activity IRI of shape
    /// `urn:blake3:{64-hex}` — never `http://example.org/...`.
    #[test]
    fn seal_uses_blake3_urn() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let decision = decide(&context);
        let deltas = materialize(&decision, &context)?;
        let receipts = seal(&decision, &deltas, "field-x", None);

        let mut saw_at_least_one = false;
        for r in receipts.into_iter().flatten() {
            saw_at_least_one = true;
            let iri = r.activity_iri.as_str();
            assert!(
                iri.starts_with("urn:blake3:"),
                "activity IRI must start with 'urn:blake3:', got '{}'",
                iri
            );
            let suffix = &iri["urn:blake3:".len()..];
            assert_eq!(
                suffix.len(),
                64,
                "activity IRI must have a 64-hex suffix, got '{}' ({} chars)",
                suffix,
                suffix.len()
            );
            assert!(
                !iri.contains("example.org"),
                "activity IRI must not contain 'example.org': {}",
                iri
            );
        }
        assert!(saw_at_least_one, "expected at least one sealed receipt");
        Ok(())
    }

    /// Track D regression: missing-evidence delta must be a gap-finding
    /// `schema:AskAction`, never a fabricated `prov:value "placeholder"`.
    #[test]
    fn missing_evidence_emits_ask_action_not_placeholder() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let outcomes = bark(&context)?;
        let me = outcomes
            .iter()
            .find(|o| o.hook_name == "missing_evidence")
            .expect("missing_evidence should fire");

        // Must NOT contain the old placeholder literal.
        let nt = me.delta.to_ntriples();
        assert!(
            !nt.contains("\"placeholder\""),
            "delta must not contain fabricated 'placeholder' literal:\n{}",
            nt
        );
        assert!(
            !nt.contains("<http://www.w3.org/ns/prov#value>"),
            "delta must not assert prov:value (no fabricated evidence):\n{}",
            nt
        );

        // Must contain a schema:AskAction activity and a schema:object link.
        let h_ask = format!(
            "{:08x}",
            crate::utils::dense::fnv1a_64("https://schema.org/AskAction".as_bytes()) as u32
        );
        let h_object = format!(
            "{:04x}",
            crate::utils::dense::fnv1a_64("https://schema.org/object".as_bytes()) as u16
        );
        assert!(
            nt.contains(&h_ask),
            "delta must reference schema:AskAction:\n{}",
            nt
        );
        assert!(
            nt.contains(&h_object),
            "delta must reference schema:object:\n{}",
            nt
        );
        Ok(())
    }

    /// Track D regression: phrase-binding delta must emit a
    /// `prov:wasInformedBy` provenance link, never the old
    /// `skos:definition "derived from prefLabel"` placeholder.
    #[test]
    fn phrase_binding_emits_was_informed_by() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"hello\" .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let outcomes = bark(&context)?;
        let pb = outcomes
            .iter()
            .find(|o| o.hook_name == "phrase_binding")
            .expect("phrase_binding should fire");

        let nt = pb.delta.to_ntriples();
        assert!(
            !nt.contains("derived from prefLabel"),
            "delta must not contain the old skos:definition placeholder:\n{}",
            nt
        );
        let h_informed = format!(
            "{:04x}",
            crate::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#wasInformedBy".as_bytes())
                as u16
        );
        assert!(
            nt.contains(&h_informed),
            "delta must use prov:wasInformedBy:\n{}",
            nt
        );
        Ok(())
    }

    /// Track D regression: transition-admissibility must emit `prov:Activity`
    /// + `prov:used`, never SHACL `sh:targetClass` / `sh:nodeKind` shapes
    ///   asserted on instances.
    #[test]
    fn transition_emits_prov_activity_not_shacl_shape() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/c1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let outcomes = bark(&context)?;
        let ta = outcomes
            .iter()
            .find(|o| o.hook_name == "transition_admissibility")
            .expect("transition_admissibility should fire");

        let nt = ta.delta.to_ntriples();
        let h_activity = format!(
            "{:08x}",
            crate::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#Activity".as_bytes()) as u32
        );
        let h_used = format!(
            "{:04x}",
            crate::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#used".as_bytes()) as u16
        );

        assert!(
            nt.contains(&h_activity),
            "delta must reference prov:Activity:\n{}",
            nt
        );
        assert!(
            nt.contains(&h_used),
            "delta must reference prov:used:\n{}",
            nt
        );
        Ok(())
    }
}
