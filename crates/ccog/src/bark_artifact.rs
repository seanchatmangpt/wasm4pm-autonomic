//! Const-generated bark artifact (Phase 4 Stage 4).
//!
//! The four built-in hooks are baked into a `'static` const slice of
//! `(name, require_mask, act_fn, emit_receipt)` tuples. Dispatch becomes a
//! linear scan over a fixed-size table — no `Vec` allocation, no
//! registration step, no plan walk. The mask comparison stays at u64-AND
//! cost; `act_fn` is a const `fn(&CompiledFieldSnapshot) -> Result<Construct8>`
//! pointer.
//!
//! Use [`bark`] for the cheapest possible dispatch — it skips the plan walk
//! entirely. Use [`bark_kernel::BarkKernel`] when you need plan-ordered
//! dispatch with custom hooks.

use crate::compiled::CompiledFieldSnapshot;
use crate::compiled_hook::{compute_present_mask, Predicate};
use crate::construct8::Construct8;
use crate::hooks::HookOutcome;
use crate::receipt::Receipt;
use anyhow::Result;
use chrono::Utc;
use oxigraph::model::{Literal, NamedNode, Term, Triple};

type ActFn = fn(&CompiledFieldSnapshot) -> Result<Construct8>;

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
    },
    BarkSlot {
        name: "phrase_binding",
        require_mask: pref_label_mask(),
        act: act_phrase_binding,
        emit_receipt: true,
    },
    BarkSlot {
        name: "transition_admissibility",
        require_mask: rdf_type_mask(),
        act: act_transition_admissibility,
        emit_receipt: true,
    },
    BarkSlot {
        name: "receipt",
        require_mask: 0,
        act: act_receipt,
        emit_receipt: true,
    },
];

fn act_missing_evidence(snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    let dd = NamedNode::new("https://schema.org/DigitalDocument")
        .expect("Invalid schema:DigitalDocument IRI");
    let pv = NamedNode::new("http://www.w3.org/ns/prov#value")
        .expect("Invalid prov:value IRI");
    let mut delta = Construct8::empty();
    for d in snap.instances_of(&dd) {
        if delta.is_full() {
            break;
        }
        if !snap.has_value_for(d, &pv) {
            let _ = delta.push(Triple::new(
                d.clone(),
                pv.clone(),
                Term::Literal(Literal::new_simple_literal("placeholder")),
            ));
        }
    }
    Ok(delta)
}

fn act_phrase_binding(snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    let pl = NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")
        .expect("Invalid skos:prefLabel IRI");
    let def = NamedNode::new("http://www.w3.org/2004/02/skos/core#definition")
        .expect("Invalid skos:definition IRI");
    let mut delta = Construct8::empty();
    for (concept, _label) in snap.pairs_with_predicate(&pl) {
        if delta.is_full() {
            break;
        }
        let _ = delta.push(Triple::new(
            concept.clone(),
            def.clone(),
            Term::Literal(Literal::new_simple_literal("derived from prefLabel")),
        ));
    }
    Ok(delta)
}

fn act_transition_admissibility(snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    let rt = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        .expect("Invalid rdf:type IRI");
    let tc = NamedNode::new("http://www.w3.org/ns/shacl#targetClass")
        .expect("Invalid sh:targetClass IRI");
    let nk = NamedNode::new("http://www.w3.org/ns/shacl#nodeKind")
        .expect("Invalid sh:nodeKind IRI");
    let bi = NamedNode::new("http://www.w3.org/ns/shacl#BlankNodeOrIRI")
        .expect("Invalid sh:BlankNodeOrIRI IRI");
    let bi_term: Term = bi.into();

    let mut delta = Construct8::empty();
    let mut emitted: u8 = 0;
    for (subj, type_term) in snap.pairs_with_predicate(&rt) {
        if emitted >= 4 {
            break;
        }
        if let Term::NamedNode(_) = type_term {
            let _ = delta.push(Triple::new(subj.clone(), tc.clone(), type_term.clone()));
            let _ = delta.push(Triple::new(subj.clone(), nk.clone(), bi_term.clone()));
            emitted += 1;
        }
    }
    Ok(delta)
}

fn act_receipt(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    let h = blake3::hash(b"receipt_hook");
    let activity = NamedNode::new(&format!("urn:blake3:{}", h.to_hex()))?;
    let rt = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
    let act = NamedNode::new("http://www.w3.org/ns/prov#Activity")?;
    let assoc = NamedNode::new("http://www.w3.org/ns/prov#wasAssociatedWith")?;
    let agent = NamedNode::new("http://www.w3.org/ns/prov#Agent")?;
    let act_term: Term = act.into();
    let agent_term: Term = agent.into();

    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::new(activity.clone(), rt, act_term));
    let _ = delta.push(Triple::new(activity, assoc, agent_term));
    Ok(delta)
}

/// Single-pass dispatch over the const built-in slot table.
///
/// Computes `present_mask` once, then for each slot fires iff
/// `(slot.require_mask & present_mask) == slot.require_mask`. No allocation
/// for the slot list — it's `&'static [BarkSlot]`. Outcomes are heap-allocated
/// only because `Construct8` stores triples; the dispatch itself is alloc-free.
pub fn bark(snap: &CompiledFieldSnapshot) -> Result<Vec<HookOutcome>> {
    bark_table(snap, BUILTINS)
}

/// Dispatch over an arbitrary `&'static [BarkSlot]` table.
///
/// Same single-pass mask logic as [`bark`] but lets callers swap in a custom
/// const table (e.g., for extension experiments without changing the default).
pub fn bark_table(
    snap: &CompiledFieldSnapshot,
    table: &'static [BarkSlot],
) -> Result<Vec<HookOutcome>> {
    let present = compute_present_mask(snap);
    let mut outcomes = Vec::with_capacity(table.len());
    for slot in table {
        if (slot.require_mask & present) != slot.require_mask {
            continue;
        }
        let delta = (slot.act)(snap)?;
        let receipt = if slot.emit_receipt {
            let activity = crate::graph::GraphIri::from_iri(&format!(
                "http://example.org/hook/{}#{}",
                slot.name,
                Utc::now().timestamp()
            ))?;
            let hash = Receipt::blake3_hex(&delta.receipt_bytes());
            Some(Receipt::new(activity, hash, Utc::now()))
        } else {
            None
        };
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
    use crate::field::FieldContext;

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
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let outcomes = bark(&snap)?;
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
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let outcomes = bark(&snap)?;
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
        }];
        let field = FieldContext::new("test");
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let outcomes = bark_table(&snap, TINY)?;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].hook_name, "receipt_only");
        assert!(outcomes[0].receipt.is_none());
        Ok(())
    }
}
