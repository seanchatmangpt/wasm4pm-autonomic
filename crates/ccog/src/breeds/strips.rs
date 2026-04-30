//! STRIPS breed: transition admissibility via direct triple-pattern checks.

use anyhow::Result;
use oxigraph::model::NamedNode;
use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::powl::{Powl8, Powl8Node, MAX_NODES};
use crate::verdict::{Breed, PlanAdmission, PlanVerdict, TransitionVerdict};

/// STRIPS: Transition admissibility checking.
/// Evaluates whether a candidate operation can lawfully move the field from one graph state to another.
/// Preconditions: each `schema:DigitalDocument` must have a `prov:value`.

/// Check whether the candidate operation's transition is admissible.
/// Direct triple-pattern walk — no SPARQL parsing.
pub fn check_transition(
    candidate_iri: &GraphIri,
    field: &FieldContext,
) -> Result<TransitionVerdict> {
    let digital_document = NamedNode::new("https://schema.org/DigitalDocument")?;
    let prov_value = NamedNode::new("http://www.w3.org/ns/prov#value")?;

    let docs = field.graph.instances_of(&digital_document)?;
    for doc in &docs {
        if !field.graph.has_value_for(doc, &prov_value)? {
            return Ok(TransitionVerdict {
                admissible: false,
                blocked_by: vec![candidate_iri.clone()],
            });
        }
    }
    Ok(TransitionVerdict {
        admissible: true,
        blocked_by: Vec::new(),
    })
}

/// Probe whether a given breed is admissible right now against `field`.
///
/// Each breed has a precondition predicate that must be satisfied by the
/// graph for the breed to advance:
///
/// | Breed         | Precondition predicate           | Reason                                  |
/// |---------------|----------------------------------|-----------------------------------------|
/// | Eliza         | `skos:prefLabel`                 | Phrase binding needs labeled subjects   |
/// | Mycin         | `prov:value` OR no missing DDs   | Evidence available                      |
/// | Strips        | All DDs have `prov:value`        | Lawful transition                       |
/// | Shrdlu        | `rdf:type`                       | Affordance probing needs typed subjects |
/// | Prolog        | `skos:broader`                   | Transitive relation chains              |
/// | Hearsay       | `prov:wasInformedBy`             | Fusion needs attestations               |
/// | Dendral       | `prov:wasGeneratedBy`            | Chain reconstruction needs activities   |
/// | CompiledHook  | always advanced                  | Mask known by construction              |
pub fn admit_breed(breed: Breed, field: &FieldContext) -> Result<bool> {
    use oxigraph::model::NamedNode as N;
    let admissible = match breed {
        Breed::Eliza => field.graph.pattern_exists(
            None,
            Some(&N::new("http://www.w3.org/2004/02/skos/core#prefLabel")?),
            None,
        )?,
        Breed::Mycin => {
            // Advanced if any prov:value is present or all DDs have one.
            let has_value = field.graph.pattern_exists(
                None,
                Some(&N::new("http://www.w3.org/ns/prov#value")?),
                None,
            )?;
            if has_value {
                true
            } else {
                let dd = N::new("https://schema.org/DigitalDocument")?;
                let pv = N::new("http://www.w3.org/ns/prov#value")?;
                let docs = field.graph.instances_of(&dd)?;
                docs.iter().all(|d| {
                    field
                        .graph
                        .has_value_for(d, &pv)
                        .unwrap_or(false)
                })
            }
        }
        Breed::Strips => {
            let probe = GraphIri::from_iri("urn:ccog:powl8:strips-probe")?;
            check_transition(&probe, field)?.admissible
        }
        Breed::Shrdlu => field.graph.pattern_exists(
            None,
            Some(&N::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?),
            None,
        )?,
        Breed::Prolog => field.graph.pattern_exists(
            None,
            Some(&N::new("http://www.w3.org/2004/02/skos/core#broader")?),
            None,
        )?,
        Breed::Hearsay => field.graph.pattern_exists(
            None,
            Some(&N::new("http://www.w3.org/ns/prov#wasInformedBy")?),
            None,
        )?,
        Breed::Dendral => field.graph.pattern_exists(
            None,
            Some(&N::new("http://www.w3.org/ns/prov#wasGeneratedBy")?),
            None,
        )?,
        Breed::CompiledHook => true,
        // Phase-9 breeds: each delegates to its own pure graph-probe admit.
        // SOAR is hard-gated on Phase 7 trace-history persistence and will
        // routinely return false until that lands.
        Breed::Gps => crate::breeds::gps::admit(field)?,
        Breed::Soar => crate::breeds::soar::admit(field)?,
        Breed::Prs => crate::breeds::prs::admit(field)?,
        Breed::Cbr => crate::breeds::cbr::admit(field)?,
    };
    Ok(admissible)
}

/// Admit a POWL8 plan against the field state.
///
/// Performs `shape_match` first. If the plan is not Sound (Cyclic or
/// Malformed), returns a [`PlanVerdict`] with empty `ready`/`blocked` and the
/// soundness reason. Otherwise computes per-node "advanced" status from the
/// field and classifies each node as ready or blocked based on its
/// predecessors.
///
/// Advanced semantics:
/// - `StartNode`, `Silent`: advanced = `true`.
/// - `EndNode`: advanced = `false` (entered only after all predecessors).
/// - `Activity(breed)`: advanced iff [`admit_breed`] returns `true` for the
///   breed against the current field. Each breed has its own precondition
///   predicate (see [`admit_breed`]).
/// - `OperatorSequence`, `OperatorParallel`, `PartialOrder`: not directly
///   advanced themselves; their children's advancement determines downstream
///   readiness.
pub fn admit_powl8(plan: &Powl8, field: &FieldContext) -> Result<PlanVerdict> {
    if let Err(admission) = plan.shape_match() {
        return Ok(PlanVerdict {
            ready: Vec::new(),
            blocked: Vec::new(),
            admissible: false,
            admission,
        });
    }

    let mut advanced = [false; MAX_NODES];
    for (idx, node) in plan.nodes.iter().enumerate() {
        if idx >= MAX_NODES {
            break;
        }
        advanced[idx] = match *node {
            Powl8Node::StartNode | Powl8Node::Silent => true,
            Powl8Node::EndNode => false,
            Powl8Node::Activity(breed) => admit_breed(breed, field)?,
            Powl8Node::OperatorSequence { .. }
            | Powl8Node::OperatorParallel { .. }
            | Powl8Node::PartialOrder { .. } => false,
            // Choice and Loop are structural — their own activity status is
            // derived from descendants. They are not directly advanced.
            Powl8Node::Choice { .. } | Powl8Node::Loop { .. } => false,
        };
    }

    admit_powl8_with_advanced(plan, &advanced)
}

/// Test-friendly variant of [`admit_powl8`] that accepts an explicit per-node
/// advanced bitmask instead of probing the field.
///
/// Classifies each node:
/// - **Ready** iff the node is not yet advanced *and* every direct
///   predecessor (per [`Powl8::predecessor_masks`]) is advanced.
/// - **Blocked** iff the node is not yet advanced *and* at least one direct
///   predecessor is not advanced.
/// - Already-advanced nodes appear in neither list.
///
/// `admissible` is `true` iff the plan is Sound and `ready` is non-empty
/// (or every node is already advanced — i.e., nothing remains to do).
pub fn admit_powl8_with_advanced(
    plan: &Powl8,
    advanced: &[bool; MAX_NODES],
) -> Result<PlanVerdict> {
    if let Err(admission) = plan.shape_match() {
        return Ok(PlanVerdict {
            ready: Vec::new(),
            blocked: Vec::new(),
            admissible: false,
            admission,
        });
    }

    let preds = plan.predecessor_masks();
    let n = plan.nodes.len();
    let mut ready: Vec<usize> = Vec::new();
    let mut blocked: Vec<usize> = Vec::new();

    for i in 0..n {
        if advanced[i] {
            continue;
        }
        // Skip pure structural operators/partial-order containers — they are
        // not themselves "ready/blocked" runtime activities.
        if matches!(
            plan.nodes[i],
            Powl8Node::OperatorSequence { .. }
                | Powl8Node::OperatorParallel { .. }
                | Powl8Node::PartialOrder { .. }
                | Powl8Node::Choice { .. }
                | Powl8Node::Loop { .. }
        ) {
            continue;
        }

        let p = preds[i];
        let mut all_preds_advanced = true;
        let mut bits = p;
        while bits != 0 {
            let j = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            if j >= MAX_NODES || !advanced[j] {
                all_preds_advanced = false;
                break;
            }
        }
        if all_preds_advanced {
            ready.push(i);
        } else {
            blocked.push(i);
        }
    }

    let admissible = !ready.is_empty()
        || (0..n).all(|i| {
            advanced[i]
                || matches!(
                    plan.nodes[i],
                    Powl8Node::OperatorSequence { .. }
                        | Powl8Node::OperatorParallel { .. }
                        | Powl8Node::PartialOrder { .. }
                        | Powl8Node::Choice { .. }
                        | Powl8Node::Loop { .. }
                )
        });

    Ok(PlanVerdict {
        ready,
        blocked,
        admissible,
        admission: PlanAdmission::Sound,
    })
}

#[cfg(test)]
mod breed_probe_tests {
    use super::*;

    fn field_with(triples: &str) -> FieldContext {
        let mut f = FieldContext::new("t");
        f.load_field_state(triples).expect("load");
        f
    }

    #[test]
    fn eliza_admitted_when_pref_label_present() -> Result<()> {
        let f = field_with(
            "<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
        );
        assert!(admit_breed(Breed::Eliza, &f)?);
        Ok(())
    }

    #[test]
    fn eliza_denied_on_empty_field() -> Result<()> {
        let f = FieldContext::new("t");
        assert!(!admit_breed(Breed::Eliza, &f)?);
        Ok(())
    }

    #[test]
    fn mycin_admitted_when_prov_value_present() -> Result<()> {
        let f = field_with(
            "<http://example.org/d1> <http://www.w3.org/ns/prov#value> \"x\" .\n",
        );
        assert!(admit_breed(Breed::Mycin, &f)?);
        Ok(())
    }

    #[test]
    fn mycin_admitted_when_no_dd_present() -> Result<()> {
        // No DDs and no prov:value → trivially "all DDs have prov:value" (vacuous truth).
        let f = FieldContext::new("t");
        assert!(admit_breed(Breed::Mycin, &f)?);
        Ok(())
    }

    #[test]
    fn mycin_denied_when_dd_missing_prov_value() -> Result<()> {
        let f = field_with(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        );
        assert!(!admit_breed(Breed::Mycin, &f)?);
        Ok(())
    }

    #[test]
    fn strips_admitted_when_all_dds_have_value() -> Result<()> {
        let f = field_with(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <http://example.org/d1> <http://www.w3.org/ns/prov#value> \"x\" .\n",
        );
        assert!(admit_breed(Breed::Strips, &f)?);
        Ok(())
    }

    #[test]
    fn strips_denied_when_dd_missing_value() -> Result<()> {
        let f = field_with(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        );
        assert!(!admit_breed(Breed::Strips, &f)?);
        Ok(())
    }

    #[test]
    fn shrdlu_admitted_when_rdf_type_present() -> Result<()> {
        let f = field_with(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/Thing> .\n",
        );
        assert!(admit_breed(Breed::Shrdlu, &f)?);
        Ok(())
    }

    #[test]
    fn prolog_admitted_when_skos_broader_present() -> Result<()> {
        let f = field_with(
            "<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#broader> <http://example.org/c2> .\n",
        );
        assert!(admit_breed(Breed::Prolog, &f)?);
        Ok(())
    }

    #[test]
    fn hearsay_admitted_when_prov_was_informed_by_present() -> Result<()> {
        let f = field_with(
            "<http://example.org/c1> <http://www.w3.org/ns/prov#wasInformedBy> <http://example.org/c2> .\n",
        );
        assert!(admit_breed(Breed::Hearsay, &f)?);
        Ok(())
    }

    #[test]
    fn dendral_admitted_when_prov_was_generated_by_present() -> Result<()> {
        let f = field_with(
            "<http://example.org/e1> <http://www.w3.org/ns/prov#wasGeneratedBy> <http://example.org/a1> .\n",
        );
        assert!(admit_breed(Breed::Dendral, &f)?);
        Ok(())
    }

    #[test]
    fn compiled_hook_always_admitted() -> Result<()> {
        let f = FieldContext::new("t");
        assert!(admit_breed(Breed::CompiledHook, &f)?);
        Ok(())
    }
}
