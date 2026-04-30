//! Knowledge hook 4-tuple architecture — (trigger, check, act, receipt) autonomic system.
//!
//! Hooks encode the closed-loop cognitive pattern: detect a condition (trigger),
//! validate it (check), execute a transformation (act), and emit provenance (receipt).
//! Hooks are stateless, deterministic, and composable via HookRegistry.
//!
//! # Architecture
//!
//! Each `KnowledgeHook` is a 4-tuple:
//! - **Trigger**: When to fire (RDF change, SPARQL ASK, or manual)
//! - **Check**: Validation predicate (SPARQL or Fn)
//! - **Act**: Transformation that produces bounded delta (SPARQL CONSTRUCT or Fn)
//! - **Receipt**: Optional PROV activity emission
//!
//! # Examples
//!
//! ```ignore
//! let mut registry = HookRegistry::new();
//! registry.register(missing_evidence_hook());
//! let outcomes = registry.fire_matching(&field)?;
//! for outcome in outcomes {
//!     println!("{}: {} triples", outcome.hook_name, outcome.delta.len());
//! }
//! ```
//!
//! # Warm vs hot dispatch path
//!
//! `HookRegistry::fire_matching` is the **warm/reference** dispatch path:
//! every call rebuilds a `CompiledFieldSnapshot` and walks heap-allocated
//! hook lists. It is appropriate for tests, fixtures, and tools — NOT for
//! the hot bark loop. For nanoscale dispatch use:
//! - `crate::bark_kernel::BarkKernel` for plan-ordered POWL8 dispatch
//! - `crate::bark_artifact::bark` for const-table dispatch
//!
//! Both expose `decide()` / `materialize()` / `seal()` stages with explicit
//! cost tiers.

use crate::compiled::CompiledFieldSnapshot;
use crate::construct8::Construct8;
use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::receipt::Receipt;
use anyhow::Result;
use chrono::Utc;

/// Trigger condition for a hook to fire.
///
/// Encodes patterns: reactive (RDF property change), pattern-match,
/// type-presence, interrogative (SPARQL ASK), or manual invocation.
#[derive(Debug, Clone)]
pub enum HookTrigger {
    /// Fire when a triple with the given predicate is asserted in the graph.
    /// Evaluates to ASK `{ ?s <predicate> ?o }`.
    RdfChange {
        /// The predicate IRI to watch for assertions.
        predicate: GraphIri,
    },

    /// Fire when a SPARQL ASK query returns true.
    /// Query is prepended with PREFIXES automatically.
    SparqlAsk {
        /// The SPARQL ASK query string.
        query: String,
    },

    /// Fire when any quad matching `(subject?, predicate?, object?)` exists.
    /// Direct triple-pattern lookup — no SPARQL parsing.
    Pattern {
        /// Subject IRI to match (None = wildcard).
        subject: Option<oxigraph::model::NamedNode>,
        /// Predicate IRI to match (None = wildcard).
        predicate: Option<oxigraph::model::NamedNode>,
        /// Object term to match (None = wildcard).
        object: Option<oxigraph::model::Term>,
    },

    /// Fire when any instance of the given class exists.
    /// Equivalent to `Pattern { predicate: rdf:type, object: class }` but cheaper.
    TypePresent(oxigraph::model::NamedNode),

    /// Always fires during `HookRegistry::fire_matching`. Replaces the old
    /// `Manual` semantics (which always evaluated to true despite its name).
    Always,

    /// Fires only when invoked via `HookRegistry::fire_one(name)`. Skipped
    /// during `fire_matching`. Use for hooks that must be explicitly named
    /// at the call site (e.g. operator-driven receipts).
    ManualOnly,
}

/// Validation predicate for a hook condition.
///
/// Encodes patterns: declarative (SPARQL), imperative (Fn), pattern-match, or
/// denial-polarity admit. All except `Admit` return `Result<bool>`.
#[derive(Clone)]
pub enum HookCheck {
    /// SPARQL ASK query. Query is prepended with PREFIXES automatically.
    Sparql(String),

    /// Rust function pointer. Must be deterministic and stateless.
    Fn(fn(&FieldContext) -> Result<bool>),

    /// Direct triple-pattern existence check. No SPARQL parsing.
    Pattern {
        /// Subject IRI to match (None = wildcard).
        subject: Option<oxigraph::model::NamedNode>,
        /// Predicate IRI to match (None = wildcard).
        predicate: Option<oxigraph::model::NamedNode>,
        /// Object term to match (None = wildcard).
        object: Option<oxigraph::model::Term>,
    },

    /// Denial-polarity admit function: 0 = admitted, nonzero = denied.
    /// Cannot fail; composed via bitwise OR for branchless gating.
    Admit(fn(&FieldContext) -> u64),

    /// Snapshot-driven check: O(1) HashMap lookups, no graph walks.
    SnapshotFn(fn(&CompiledFieldSnapshot) -> bool),

    /// Denial-polarity admit over the compiled snapshot: 0 = admitted,
    /// nonzero = denied. Snapshot-native version of `HookCheck::Admit`.
    SnapshotAdmit(fn(&CompiledFieldSnapshot) -> u64),
}

impl std::fmt::Debug for HookCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sparql(q) => f.debug_tuple("Sparql").field(q).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").field(&"<fn>").finish(),
            Self::Pattern { subject, predicate, object } => f
                .debug_struct("Pattern")
                .field("subject", subject)
                .field("predicate", predicate)
                .field("object", object)
                .finish(),
            Self::Admit(_) => f.debug_tuple("Admit").field(&"<admit>").finish(),
            Self::SnapshotFn(_) => f.debug_tuple("SnapshotFn").field(&"<snap>").finish(),
            Self::SnapshotAdmit(_) => f.debug_tuple("SnapshotAdmit").field(&"<snap-admit>").finish(),
        }
    }
}

/// Transformation action that produces a bounded CONSTRUCT delta.
///
/// Encodes patterns: declarative (SPARQL CONSTRUCT), imperative (Fn), or
/// pre-computed constant triples (no parsing).
#[derive(Clone)]
pub enum HookAct {
    /// SPARQL CONSTRUCT query producing ≤8 triples. Query is prepended with PREFIXES automatically.
    Sparql(String),

    /// Rust function pointer producing bounded delta. Must be deterministic and stateless.
    Fn(fn(&FieldContext) -> Result<Construct8>),

    /// Pre-computed triples returned verbatim (≤8). No parsing or SPARQL.
    ConstantTriples(Vec<oxigraph::model::Triple>),

    /// Snapshot-driven act: O(1) HashMap lookups, no graph walks.
    SnapshotFn(fn(&CompiledFieldSnapshot) -> Result<Construct8>),
}

impl std::fmt::Debug for HookAct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sparql(q) => f.debug_tuple("Sparql").field(q).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").field(&"<fn>").finish(),
            Self::ConstantTriples(t) => f.debug_tuple("ConstantTriples").field(&t.len()).finish(),
            Self::SnapshotFn(_) => f.debug_tuple("SnapshotFn").field(&"<snap>").finish(),
        }
    }
}

/// Knowledge hook: reactive closure (trigger, check, act, receipt).
///
/// Hooks are stateless, deterministic rule activations in the field's
/// RDF graph. All IRIs and SPARQL strings use public ontologies only.
#[derive(Debug, Clone)]
pub struct KnowledgeHook {
    /// Unique hook name for provenance and debugging.
    pub name: &'static str,

    /// Firing condition: when to evaluate this hook.
    pub trigger: HookTrigger,

    /// Validation predicate: must pass before act runs.
    pub check: HookCheck,

    /// Transformation action: produces bounded delta.
    pub act: HookAct,

    /// If true, emit a PROV activity receipt after successful act.
    pub emit_receipt: bool,
}

/// Outcome of a fired and executed hook.
///
/// Captures the delta produced and optional provenance receipt.
#[derive(Debug, Clone)]
pub struct HookOutcome {
    /// Name of the hook that fired.
    pub hook_name: &'static str,

    /// Bounded delta (≤8 triples) produced by act.
    pub delta: Construct8,

    /// Optional PROV activity receipt if emit_receipt was true.
    pub receipt: Option<Receipt>,
}

/// Registry of knowledge hooks with firing orchestration.
///
/// Hooks are evaluated in registration order. A hook fires if:
/// 1. Trigger evaluates to true, AND
/// 2. Check evaluates to true, AND
/// 3. Act produces a non-error delta.
///
/// The registry enforces deterministic sequencing and captures all outcomes.
#[derive(Debug, Default)]
pub struct HookRegistry {
    hooks: Vec<KnowledgeHook>,
}

impl HookRegistry {
    /// Create a new empty hook registry.
    pub fn new() -> Self {
        Self {
            hooks: Vec::new(),
        }
    }

    /// Register a knowledge hook for evaluation.
    ///
    /// Hooks are evaluated in registration order.
    pub fn register(&mut self, hook: KnowledgeHook) {
        self.hooks.push(hook);
    }

    /// Fire all hooks whose triggers and checks match the field context.
    ///
    /// Iterates registered hooks in order. For each hook:
    /// 1. Evaluate trigger — skip if false
    /// 2. Evaluate check — skip if false or error
    /// 3. Execute act — capture delta and optional receipt
    /// 4. Append outcome to results
    ///
    /// Hook execution is deterministic and side-effect free at the registry level.
    /// Individual act functions may have imperative side effects (e.g., logging).
    pub fn fire_matching(&self, field: &FieldContext) -> Result<Vec<HookOutcome>> {
        let snapshot = CompiledFieldSnapshot::from_field(field)?;
        let mut outcomes = Vec::new();

        for hook in &self.hooks {
            // Evaluate trigger
            let trigger_fired = evaluate_trigger(&hook.trigger, field, &snapshot)?;
            if !trigger_fired {
                continue;
            }

            // Evaluate check
            let check_passed = evaluate_check(&hook.check, field, &snapshot)?;
            if !check_passed {
                continue;
            }

            // Execute act
            let delta = evaluate_act(&hook.act, field, &snapshot)?;

            // Generate receipt if requested
            let receipt = if hook.emit_receipt {
                let activity_iri = GraphIri::from_iri(&format!(
                    "http://example.org/hook/{}#{}",
                    hook.name,
                    Utc::now().timestamp()
                ))?;
                let hash = Receipt::blake3_hex(&delta.receipt_bytes());
                Some(Receipt::new(activity_iri, hash, Utc::now()))
            } else {
                None
            };

            outcomes.push(HookOutcome {
                hook_name: hook.name,
                delta,
                receipt,
            });
        }

        Ok(outcomes)
    }

    /// Invoke a single hook by name, bypassing trigger-variant gating.
    ///
    /// Looks up the hook by `name` and runs check/act. Trigger evaluation is
    /// still performed for non-`ManualOnly` variants so that data-dependent
    /// triggers (e.g. `TypePresent`) gate correctly; for `ManualOnly` the
    /// trigger is treated as fired so the hook always runs when explicitly
    /// requested by name.
    ///
    /// Returns:
    /// - `Ok(Some(outcome))` — hook ran and produced a delta
    /// - `Ok(None)`           — no hook with `name` is registered, or its
    ///                          non-`ManualOnly` trigger declined to fire,
    ///                          or its check returned false
    pub fn fire_one(
        &self,
        field: &FieldContext,
        name: &str,
    ) -> Result<Option<HookOutcome>> {
        let hook = match self.hooks.iter().find(|h| h.name == name) {
            Some(h) => h,
            None => return Ok(None),
        };

        let snapshot = CompiledFieldSnapshot::from_field(field)?;

        // Trigger: ManualOnly bypasses to true; everything else evaluates
        // normally so data-dependent triggers still gate. `Always` naturally
        // returns true.
        let trigger_fired = match &hook.trigger {
            HookTrigger::ManualOnly => true,
            other => evaluate_trigger(other, field, &snapshot)?,
        };
        if !trigger_fired {
            return Ok(None);
        }

        let check_passed = evaluate_check(&hook.check, field, &snapshot)?;
        if !check_passed {
            return Ok(None);
        }

        let delta = evaluate_act(&hook.act, field, &snapshot)?;

        let receipt = if hook.emit_receipt {
            let activity_iri = GraphIri::from_iri(&format!(
                "http://example.org/hook/{}#{}",
                hook.name,
                Utc::now().timestamp()
            ))?;
            let hash = Receipt::blake3_hex(&delta.receipt_bytes());
            Some(Receipt::new(activity_iri, hash, Utc::now()))
        } else {
            None
        };

        Ok(Some(HookOutcome {
            hook_name: hook.name,
            delta,
            receipt,
        }))
    }
}

/// Evaluate a hook trigger against the field context.
///
/// Snapshot-aware variants short-circuit through the pre-built indices:
/// - `TypePresent(class)` → snapshot.instances_of(class)
/// - `Pattern { subject: None, predicate: Some(p), object: None }` → snapshot.has_any_with_predicate(p)
/// - `Always` → unconditionally true
/// - `ManualOnly` → unconditionally false (use `HookRegistry::fire_one` to invoke)
///
/// Other patterns fall back to the graph walk.
fn evaluate_trigger(
    trigger: &HookTrigger,
    field: &FieldContext,
    snapshot: &CompiledFieldSnapshot,
) -> Result<bool> {
    match trigger {
        HookTrigger::RdfChange { predicate } => {
            let p = oxigraph::model::NamedNode::new(predicate.as_str())?;
            Ok(snapshot.has_any_with_predicate(&p))
        }
        HookTrigger::SparqlAsk { query } => field.graph.ask(query),
        HookTrigger::Pattern { subject: None, predicate: Some(p), object: None } => {
            Ok(snapshot.has_any_with_predicate(p))
        }
        HookTrigger::Pattern { subject, predicate, object } => {
            field.graph.pattern_exists(subject.as_ref(), predicate.as_ref(), object.as_ref())
        }
        HookTrigger::TypePresent(class) => Ok(!snapshot.instances_of(class).is_empty()),
        HookTrigger::Always => Ok(true),
        HookTrigger::ManualOnly => Ok(false),
    }
}

/// Evaluate a hook check against the field context.
///
/// - `Sparql(query)` → call field.graph.ask() with PREFIXES prepended
/// - `Fn(f)` → call f(field)
/// - `Pattern { ... }` → direct triple-pattern existence
/// - `Admit(f)` → denial-polarity admit over `FieldContext` (0 = admitted)
/// - `SnapshotFn(f)` → call f(snapshot) — O(1) HashMap path
/// - `SnapshotAdmit(f)` → denial-polarity admit over `CompiledFieldSnapshot` (0 = admitted)
fn evaluate_check(
    check: &HookCheck,
    field: &FieldContext,
    snapshot: &CompiledFieldSnapshot,
) -> Result<bool> {
    match check {
        HookCheck::Sparql(query) => field.graph.ask(query),
        HookCheck::Fn(f) => f(field),
        HookCheck::Pattern { subject, predicate, object } => {
            field.graph.pattern_exists(subject.as_ref(), predicate.as_ref(), object.as_ref())
        }
        HookCheck::Admit(f) => Ok(crate::admit::admitted(f(field))),
        HookCheck::SnapshotFn(f) => Ok(f(snapshot)),
        HookCheck::SnapshotAdmit(f) => Ok(crate::admit::admitted(f(snapshot))),
    }
}

/// Evaluate a hook act against the field context.
///
/// - `Sparql(query)` → call field.graph.construct(), wrap in Construct8
/// - `Fn(f)` → call f(field) directly
/// - `SnapshotFn(f)` → call f(snapshot) — O(1) HashMap path
fn evaluate_act(
    act: &HookAct,
    field: &FieldContext,
    snapshot: &CompiledFieldSnapshot,
) -> Result<Construct8> {
    match act {
        HookAct::Sparql(query) => {
            let triples = field.graph.construct(query)?;
            let mut delta = Construct8::empty();
            for triple in triples {
                if !delta.push(triple) {
                    anyhow::bail!("CONSTRUCT query produced more than 8 triples");
                }
            }
            Ok(delta)
        }
        HookAct::Fn(f) => f(field),
        HookAct::ConstantTriples(triples) => {
            let mut delta = Construct8::empty();
            for triple in triples {
                if !delta.push(triple.clone()) {
                    anyhow::bail!("ConstantTriples act exceeded 8-triple budget");
                }
            }
            Ok(delta)
        }
        HookAct::SnapshotFn(f) => f(snapshot),
    }
}

// ---------------------------------------------------------------------------
// Built-in hook helpers — snapshot-driven (Phase 4 Stage 1).
// ---------------------------------------------------------------------------

/// Check: any `schema:DigitalDocument` instance lacking a `prov:value`.
///
/// Iterates the snapshot's `instances_of` and short-circuits on first gap.
fn check_any_doc_missing_value_snap(snap: &CompiledFieldSnapshot) -> bool {
    let dd = oxigraph::model::NamedNode::new("https://schema.org/DigitalDocument")
        .expect("Invalid schema:DigitalDocument IRI");
    let pv = oxigraph::model::NamedNode::new("http://www.w3.org/ns/prov#value")
        .expect("Invalid prov:value IRI");
    for d in snap.instances_of(&dd) {
        if !snap.has_value_for(d, &pv) {
            return true;
        }
    }
    false
}

/// Act: emit a placeholder `prov:value` triple for each documented gap (≤8 triples).
fn emit_missing_evidence_delta_snap(snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    let dd = oxigraph::model::NamedNode::new("https://schema.org/DigitalDocument")
        .expect("Invalid schema:DigitalDocument IRI");
    let pv = oxigraph::model::NamedNode::new("http://www.w3.org/ns/prov#value")
        .expect("Invalid prov:value IRI");
    let mut delta = Construct8::empty();
    for d in snap.instances_of(&dd) {
        if delta.is_full() {
            break;
        }
        if !snap.has_value_for(d, &pv) {
            let triple = oxigraph::model::Triple::new(
                d.clone(),
                pv.clone(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    "placeholder",
                )),
            );
            let _ = delta.push(triple);
        }
    }
    Ok(delta)
}

/// Check: any subject carries a `skos:prefLabel`.
fn check_concept_with_label_snap(snap: &CompiledFieldSnapshot) -> bool {
    let pref_label =
        oxigraph::model::NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")
            .expect("Invalid skos:prefLabel IRI");
    snap.has_any_with_predicate(&pref_label)
}

/// Act: emit a `skos:definition` placeholder for each labeled concept (≤8 triples).
fn emit_phrase_definition_delta_snap(snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    let pref_label =
        oxigraph::model::NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")
            .expect("Invalid skos:prefLabel IRI");
    let definition =
        oxigraph::model::NamedNode::new("http://www.w3.org/2004/02/skos/core#definition")
            .expect("Invalid skos:definition IRI");
    let mut delta = Construct8::empty();
    for (concept, _label) in snap.pairs_with_predicate(&pref_label) {
        if delta.is_full() {
            break;
        }
        let triple = oxigraph::model::Triple::new(
            concept.clone(),
            definition.clone(),
            oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                "derived from prefLabel",
            )),
        );
        let _ = delta.push(triple);
    }
    Ok(delta)
}

/// Check: any subject carries an `rdf:type` assertion.
fn check_any_typed_subject_snap(snap: &CompiledFieldSnapshot) -> bool {
    let rdf_type =
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
            .expect("Invalid rdf:type IRI");
    snap.has_any_with_predicate(&rdf_type)
}

/// Act: emit two SHACL validity triples per typed subject (≤4 subjects, ≤8 triples).
fn emit_validity_delta_snap(snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    let rdf_type =
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
            .expect("Invalid rdf:type IRI");
    let target_class = oxigraph::model::NamedNode::new("http://www.w3.org/ns/shacl#targetClass")
        .expect("Invalid sh:targetClass IRI");
    let node_kind = oxigraph::model::NamedNode::new("http://www.w3.org/ns/shacl#nodeKind")
        .expect("Invalid sh:nodeKind IRI");
    let blank_or_iri =
        oxigraph::model::NamedNode::new("http://www.w3.org/ns/shacl#BlankNodeOrIRI")
            .expect("Invalid sh:BlankNodeOrIRI IRI");
    let blank_or_iri_term: oxigraph::model::Term = blank_or_iri.into();

    let mut delta = Construct8::empty();
    let mut subjects_emitted: u8 = 0;
    for (subj, type_term) in snap.pairs_with_predicate(&rdf_type) {
        if subjects_emitted >= 4 {
            break;
        }
        if let oxigraph::model::Term::NamedNode(_) = type_term {
            let t1 = oxigraph::model::Triple::new(
                subj.clone(),
                target_class.clone(),
                type_term.clone(),
            );
            let t2 = oxigraph::model::Triple::new(
                subj.clone(),
                node_kind.clone(),
                blank_or_iri_term.clone(),
            );
            let _ = delta.push(t1);
            let _ = delta.push(t2);
            subjects_emitted += 1;
        }
    }
    Ok(delta)
}

/// Act: emit a deterministic `prov:Activity` triple pair from a BLAKE3-derived URN.
fn emit_receipt_activity_delta_snap(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    let h = blake3::hash(b"receipt_hook");
    let activity_iri =
        oxigraph::model::NamedNode::new(&format!("urn:blake3:{}", h.to_hex()))?;
    let rdf_type =
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
    let prov_activity = oxigraph::model::NamedNode::new("http://www.w3.org/ns/prov#Activity")?;
    let prov_was_associated_with =
        oxigraph::model::NamedNode::new("http://www.w3.org/ns/prov#wasAssociatedWith")?;
    let prov_agent = oxigraph::model::NamedNode::new("http://www.w3.org/ns/prov#Agent")?;

    let prov_activity_term: oxigraph::model::Term = prov_activity.into();
    let prov_agent_term: oxigraph::model::Term = prov_agent.into();

    let mut delta = Construct8::empty();
    let _ = delta.push(oxigraph::model::Triple::new(
        activity_iri.clone(),
        rdf_type,
        prov_activity_term,
    ));
    let _ = delta.push(oxigraph::model::Triple::new(
        activity_iri,
        prov_was_associated_with,
        prov_agent_term,
    ));
    Ok(delta)
}

/// Missing evidence hook: fires when a DigitalDocument lacks `prov:value`.
///
/// **Trigger:** `TypePresent(schema:DigitalDocument)` — direct triple-pattern lookup
/// **Check:** `Fn(check_any_doc_missing_value)` — short-circuits on first gap
/// **Act:** `Fn(emit_missing_evidence_delta)` — placeholder `prov:value` per gap (≤8)
/// **Receipt:** Emitted on success
pub fn missing_evidence_hook() -> KnowledgeHook {
    KnowledgeHook {
        name: "missing_evidence",
        trigger: HookTrigger::TypePresent(
            oxigraph::model::NamedNode::new("https://schema.org/DigitalDocument")
                .expect("Invalid schema:DigitalDocument IRI"),
        ),
        check: HookCheck::SnapshotFn(check_any_doc_missing_value_snap),
        act: HookAct::SnapshotFn(emit_missing_evidence_delta_snap),
        emit_receipt: true,
    }
}

/// Phrase binding hook: fires when any subject carries a `skos:prefLabel`.
///
/// **Trigger:** `Pattern { predicate: skos:prefLabel }` — direct triple-pattern lookup
/// **Check:** `Fn(check_concept_with_label)` — short-circuits on first match
/// **Act:** `Fn(emit_phrase_definition_delta)` — `skos:definition` placeholder per pair (≤8)
/// **Receipt:** Emitted on success
pub fn phrase_binding_hook() -> KnowledgeHook {
    let pref_label =
        oxigraph::model::NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")
            .expect("Invalid skos:prefLabel IRI");
    KnowledgeHook {
        name: "phrase_binding",
        trigger: HookTrigger::Pattern {
            subject: None,
            predicate: Some(pref_label),
            object: None,
        },
        check: HookCheck::SnapshotFn(check_concept_with_label_snap),
        act: HookAct::SnapshotFn(emit_phrase_definition_delta_snap),
        emit_receipt: true,
    }
}

/// Transition admissibility hook: fires when `rdf:type` assertions are present.
///
/// **Trigger:** `Pattern { predicate: rdf:type }` — direct triple-pattern lookup
/// **Check:** `Fn(check_any_typed_subject)` — short-circuits on first typed subject
/// **Act:** `Fn(emit_validity_delta)` — SHACL validity per typed subject (≤4 subjects, ≤8 triples)
/// **Receipt:** Emitted on success
pub fn transition_admissibility_hook() -> KnowledgeHook {
    let rdf_type =
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
            .expect("Invalid rdf:type IRI");
    KnowledgeHook {
        name: "transition_admissibility",
        trigger: HookTrigger::Pattern {
            subject: None,
            predicate: Some(rdf_type),
            object: None,
        },
        check: HookCheck::SnapshotFn(check_any_typed_subject_snap),
        act: HookAct::SnapshotFn(emit_validity_delta_snap),
        emit_receipt: true,
    }
}

/// Receipt hook: always-fires trigger that emits a deterministic PROV activity receipt.
///
/// **Trigger:** `Always` — fires on every `fire_matching` call (preserves prior `Manual` semantics)
/// **Check:** Always true
/// **Act:** `Fn(emit_receipt_activity_delta)` — `urn:blake3:` activity IRI + `prov:Activity` + `prov:wasAssociatedWith prov:Agent`
/// **Receipt:** Always emitted
pub fn receipt_hook() -> KnowledgeHook {
    KnowledgeHook {
        name: "receipt",
        trigger: HookTrigger::Always,
        check: HookCheck::SnapshotFn(|_snap| true),
        act: HookAct::SnapshotFn(emit_receipt_activity_delta_snap),
        emit_receipt: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that SPARQL ASK trigger fires when query returns true.
    #[test]
    fn sparql_ask_trigger_fires_when_true() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/doc1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n"
        )?;

        let trigger = HookTrigger::SparqlAsk {
            query: "ASK { ?s rdf:type schema:DigitalDocument }".to_string(),
        };

        let snapshot = CompiledFieldSnapshot::from_field(&field)?;
        let result = evaluate_trigger(&trigger, &field, &snapshot)?;
        assert!(result, "SPARQL ASK trigger should fire when query matches");

        Ok(())
    }

    /// Verify that SPARQL ASK trigger does not fire when query returns false.
    #[test]
    fn sparql_ask_trigger_does_not_fire_when_false() -> Result<()> {
        let field = FieldContext::new("test");

        let trigger = HookTrigger::SparqlAsk {
            query: "ASK { ?s rdf:type schema:NonExistent }".to_string(),
        };

        let snapshot = CompiledFieldSnapshot::from_field(&field)?;
        let result = evaluate_trigger(&trigger, &field, &snapshot)?;
        assert!(!result, "SPARQL ASK trigger should not fire when query does not match");

        Ok(())
    }

    /// Verify that missing_evidence_hook fires when a DigitalDocument lacks prov:value.
    #[test]
    fn missing_evidence_hook_fires_on_gap() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/doc1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n"
        )?;

        let hook = missing_evidence_hook();
        let mut registry = HookRegistry::new();
        registry.register(hook);

        let outcomes = registry.fire_matching(&field)?;
        assert!(!outcomes.is_empty(), "Hook should fire on missing prov:value");
        assert_eq!(outcomes[0].hook_name, "missing_evidence");
        assert!(!outcomes[0].delta.is_empty(), "Delta should contain constructed triple");

        Ok(())
    }

    /// Verify that missing_evidence_hook does not fire when document is complete.
    #[test]
    fn missing_evidence_hook_does_not_fire_complete() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/doc1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <http://example.org/doc1> <http://www.w3.org/ns/prov#value> \"complete\" .\n"
        )?;

        let hook = missing_evidence_hook();
        let mut registry = HookRegistry::new();
        registry.register(hook);

        let outcomes = registry.fire_matching(&field)?;
        assert!(
            outcomes.is_empty(),
            "Hook should not fire when prov:value is present"
        );

        Ok(())
    }

    /// Verify that hook registry fires all matching hooks in order.
    #[test]
    fn hook_registry_fires_all_matching() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/concept1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .\n\
             <http://example.org/concept1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"Test\" .\n"
        )?;

        let hook1 = phrase_binding_hook();
        let hook2 = transition_admissibility_hook();

        let mut registry = HookRegistry::new();
        registry.register(hook1);
        registry.register(hook2);

        let outcomes = registry.fire_matching(&field)?;
        assert!(!outcomes.is_empty(), "Registry should fire matching hooks");

        Ok(())
    }

    /// Verify that hook registry returns receipts when emit_receipt is true.
    #[test]
    fn hook_registry_returns_receipts() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/doc1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n"
        )?;

        let hook = missing_evidence_hook();
        assert!(
            hook.emit_receipt,
            "missing_evidence_hook should emit receipts"
        );

        let mut registry = HookRegistry::new();
        registry.register(hook);

        let outcomes = registry.fire_matching(&field)?;
        assert!(!outcomes.is_empty(), "Should have outcomes");
        assert!(
            outcomes[0].receipt.is_some(),
            "Outcome should have receipt when emit_receipt is true"
        );
        let receipt = outcomes[0].receipt.as_ref().unwrap();
        assert_eq!(receipt.hash.len(), 64, "BLAKE3 hash should be 64 hex chars");

        Ok(())
    }

    /// Verify that `HookTrigger::Always` evaluates to true in `fire_matching`.
    #[test]
    fn always_trigger_fires_in_registry() -> Result<()> {
        let field = FieldContext::new("test");

        let hook = KnowledgeHook {
            name: "always_hook",
            trigger: HookTrigger::Always,
            check: HookCheck::SnapshotFn(|_snap| true),
            act: HookAct::ConstantTriples(vec![]),
            emit_receipt: false,
        };

        let mut registry = HookRegistry::new();
        registry.register(hook);

        let outcomes = registry.fire_matching(&field)?;
        assert_eq!(outcomes.len(), 1, "Always trigger should fire in fire_matching");
        assert_eq!(outcomes[0].hook_name, "always_hook");

        Ok(())
    }

    /// Verify that `HookTrigger::ManualOnly` is skipped during `fire_matching`.
    #[test]
    fn manual_only_trigger_skipped_in_registry() -> Result<()> {
        let field = FieldContext::new("test");

        let hook = KnowledgeHook {
            name: "manual_only_hook",
            trigger: HookTrigger::ManualOnly,
            check: HookCheck::SnapshotFn(|_snap| true),
            act: HookAct::ConstantTriples(vec![]),
            emit_receipt: false,
        };

        let mut registry = HookRegistry::new();
        registry.register(hook);

        let outcomes = registry.fire_matching(&field)?;
        assert!(
            outcomes.is_empty(),
            "ManualOnly trigger must not fire via fire_matching; got {} outcomes",
            outcomes.len()
        );

        Ok(())
    }

    /// Verify that `fire_one` invokes a `ManualOnly` hook by name.
    #[test]
    fn fire_one_invokes_manual_only_hook() -> Result<()> {
        let field = FieldContext::new("test");

        let hook = KnowledgeHook {
            name: "manual_only_invoked",
            trigger: HookTrigger::ManualOnly,
            check: HookCheck::SnapshotFn(|_snap| true),
            act: HookAct::ConstantTriples(vec![]),
            emit_receipt: false,
        };

        let mut registry = HookRegistry::new();
        registry.register(hook);

        // fire_matching skips it
        let warm = registry.fire_matching(&field)?;
        assert!(warm.is_empty(), "ManualOnly hook must be skipped in warm path");

        // fire_one runs it
        let outcome = registry.fire_one(&field, "manual_only_invoked")?;
        assert!(outcome.is_some(), "fire_one should invoke ManualOnly hook by name");
        assert_eq!(outcome.unwrap().hook_name, "manual_only_invoked");

        // Unknown name returns None
        let missing = registry.fire_one(&field, "no_such_hook")?;
        assert!(missing.is_none(), "fire_one should return None for unknown hook name");

        Ok(())
    }

    /// Verify that `HookCheck::SnapshotAdmit` dispatches with denial polarity:
    /// 0 = admitted (fires), nonzero = denied (does not fire).
    #[test]
    fn snapshot_admit_check_dispatches() -> Result<()> {
        let field = FieldContext::new("test");

        let admitted_hook = KnowledgeHook {
            name: "admit_pass",
            trigger: HookTrigger::Always,
            check: HookCheck::SnapshotAdmit(|_snap| 0),
            act: HookAct::ConstantTriples(vec![]),
            emit_receipt: false,
        };
        let denied_hook = KnowledgeHook {
            name: "admit_deny",
            trigger: HookTrigger::Always,
            check: HookCheck::SnapshotAdmit(|_snap| 1),
            act: HookAct::ConstantTriples(vec![]),
            emit_receipt: false,
        };

        let mut registry = HookRegistry::new();
        registry.register(admitted_hook);
        registry.register(denied_hook);

        let outcomes = registry.fire_matching(&field)?;
        assert_eq!(
            outcomes.len(),
            1,
            "Only the admitted SnapshotAdmit hook should fire"
        );
        assert_eq!(
            outcomes[0].hook_name, "admit_pass",
            "Admitted hook (verdict=0) must be the one that fired"
        );

        Ok(())
    }
}
