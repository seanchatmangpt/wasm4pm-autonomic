//! Verdict types from cognitive passes: binding, evidence gaps, transitions, final verdicts.

use crate::graph::GraphIri;
use crate::operation::Operation;
use crate::receipt::Receipt;

/// Bound terms from ELIZA phrase binding pass.
#[derive(Clone, Debug)]
pub struct BoundTerms {
    /// List of SKOS concept IRIs that matched the phrase.
    pub terms: Vec<GraphIri>,
}

/// Evidence gap identified by MYCIN pass.
#[derive(Clone, Debug)]
pub struct EvidenceGap {
    /// List of entity IRIs with missing evidence.
    pub missing: Vec<GraphIri>,
}

/// Transition verdict from STRIPS pass.
#[derive(Clone, Debug)]
pub struct TransitionVerdict {
    /// Whether the transition is admissible.
    pub admissible: bool,

    /// List of IRIs blocking the transition (if not admissible).
    pub blocked_by: Vec<GraphIri>,
}

/// Final verdict from the complete ccog process.
#[derive(Clone, Debug)]
pub struct Verdict {
    /// Bound JTBD terms from ELIZA.
    pub bound_terms: BoundTerms,

    /// Missing evidence detected by MYCIN.
    pub evidence_gap: Option<EvidenceGap>,

    /// Transition admissibility from STRIPS.
    pub transition: TransitionVerdict,

    /// Candidate operation produced by the cognitive passes.
    pub operation: Operation,

    /// Whether the operation is admissible.
    pub admissible: bool,

    /// PROV receipt with cryptographic proof.
    pub receipt: Receipt,
}

impl Verdict {
    /// Create a new verdict with all components.
    pub fn new(
        bound_terms: BoundTerms,
        evidence_gap: Option<EvidenceGap>,
        transition: TransitionVerdict,
        operation: Operation,
        receipt: Receipt,
    ) -> Self {
        let admissible = transition.admissible;
        Self {
            bound_terms,
            evidence_gap,
            transition,
            operation,
            admissible,
            receipt,
        }
    }
}

/// Affordance verdict from SHRDLU pass: admissible actions for an object.
#[derive(Clone, Debug)]
pub struct AffordanceVerdict {
    /// The object IRI being assessed.
    pub object: GraphIri,
    /// Set of admissible action IRIs (deduplicated, sorted).
    pub actions: Vec<GraphIri>,
}

/// Transitive relation proof from Prolog-style breed.
#[derive(Clone, Debug)]
pub struct RelationProof {
    /// Subject of the proven relation.
    pub subject: GraphIri,
    /// Predicate (transitive predicate, e.g. skos:broader).
    pub predicate: GraphIri,
    /// Target reached via predicate-chain.
    pub target: GraphIri,
    /// Path of IRIs from subject to target inclusive.
    pub path: Vec<GraphIri>,
}

/// Pack-level operational posture fused from blackboard outcomes by Hearsay-II.
///
/// Variants escalate left-to-right by signal density.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackPosture {
    /// Zero confirmed signals — no fused evidence.
    Calm,
    /// Exactly one confirmed signal — single-source observation.
    Alert,
    /// Two or three confirmed signals — multi-source corroboration.
    Engaged,
    /// Four or more confirmed signals — saturated blackboard.
    Settled,
}

impl std::fmt::Display for PackPosture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PackPosture::Calm => "calm",
            PackPosture::Alert => "alert",
            PackPosture::Engaged => "engaged",
            PackPosture::Settled => "settled",
        })
    }
}

/// Single backward step in a DENDRAL provenance walk.
#[derive(Clone, Debug)]
pub struct ProvenanceStep {
    /// IRI of the prov:Activity at this step.
    pub activity: GraphIri,
    /// IRIs of entities the activity used (prov:used).
    pub inputs: Vec<GraphIri>,
}

/// Reconstructed PROV chain produced by DENDRAL backward walk.
#[derive(Clone, Debug)]
pub struct ProvenanceChain {
    /// IRI of the entity whose lineage was reconstructed.
    pub root_entity: GraphIri,
    /// Ordered steps from the root entity backward through ancestors.
    pub steps: Vec<ProvenanceStep>,
}

/// Cognitive breed identifier — used by POWL8 plan nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Breed {
    /// ELIZA: phrase binding to public ontology.
    Eliza = 0,
    /// MYCIN: evidence-gap detection.
    Mycin = 1,
    /// STRIPS: transition admissibility.
    Strips = 2,
    /// SHRDLU: object affordance.
    Shrdlu = 3,
    /// Prolog: transitive relation proof.
    Prolog = 4,
    /// Hearsay-II: blackboard fusion.
    Hearsay = 5,
    /// DENDRAL: provenance chain reconstruction.
    Dendral = 6,
    /// Compiled-hook activity — runtime kind for nodes whose semantics are
    /// carried by an attached `CompiledHook` slot rather than a fixed breed.
    /// Used by `BarkKernel::linear` and any plan-builder that targets the
    /// compiled-hook slot table.
    CompiledHook = 7,
}

/// Soundness classification for a POWL8 plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanAdmission {
    /// Plan is acyclic and well-formed.
    Sound,
    /// Plan contains at least one cycle in its partial order.
    Cyclic,
    /// Plan has structural defects (out-of-bounds child indices, etc.).
    Malformed,
}

/// Verdict from POWL8 plan admission via STRIPS.
///
/// Ready nodes have all predecessors advanced; blocked nodes wait on at least one predecessor.
/// `admissible` is true iff the plan is Sound and at least one node is ready (or all advanced).
#[derive(Clone, Debug)]
pub struct PlanVerdict {
    /// Indices of nodes whose predecessors are all advanced (≤64).
    pub ready: Vec<usize>,
    /// Indices of nodes waiting on at least one predecessor (≤64).
    pub blocked: Vec<usize>,
    /// True iff the plan is Sound and progress is possible.
    pub admissible: bool,
    /// Soundness classification of the plan.
    pub admission: PlanAdmission,
}
