//! Multi-cycle runtime driver: composes Scheduler + PostureMachine + receipt chain.

use anyhow::Result;
use oxigraph::model::{NamedNode, Term, Triple};
use crate::construct8::Construct8;
use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::hooks::HookRegistry;
use crate::powl64::Powl64;
use crate::runtime::posture::PostureMachine;
use crate::runtime::scheduler::{Scheduler, TickReport};
use crate::verdict::PackPosture;

/// Multi-cycle ccog runtime with chained PROV receipts and posture tracking.
#[derive(Debug)]
pub struct Runtime {
    field: FieldContext,
    scheduler: Scheduler,
    posture: PostureMachine,
    last_receipt_iri: Option<GraphIri>,
    powl64: Powl64,
}

/// Aggregate report from one `Runtime::step()` cycle.
#[derive(Debug)]
pub struct StepReport {
    /// Scheduler tick result: delta + outcomes.
    pub tick: TickReport,
    /// Posture after observing this tick's signal count.
    pub posture: PackPosture,
    /// True iff a `prov:wasInformedBy` chain edge was emitted this step.
    pub chain_extended: bool,
    /// Current head of the BLAKE3 receipt chain after this step, if any.
    pub chain_head: Option<blake3::Hash>,
}

impl Runtime {
    /// Build a runtime from a field and pre-populated hook registry.
    pub fn new(field: FieldContext, registry: HookRegistry) -> Self {
        Self {
            field,
            scheduler: Scheduler::new(registry),
            posture: PostureMachine::new(),
            last_receipt_iri: None,
            powl64: Powl64::new(),
        }
    }

    /// Borrow the field (for inspection in tests/callers).
    pub fn field(&self) -> &FieldContext { &self.field }

    /// Mutable field access for loading state between steps.
    pub fn field_mut(&mut self) -> &mut FieldContext { &mut self.field }

    /// Current posture without mutation.
    pub fn posture(&self) -> PackPosture { self.posture.current() }

    /// Borrow the BLAKE3 receipt chain universe.
    pub fn powl64(&self) -> &Powl64 { &self.powl64 }

    /// Run one cycle: tick → observe posture → chain receipts → return report.
    pub fn step(&mut self) -> Result<StepReport> {
        let tick = self.scheduler.tick(&self.field)?;
        let signal_count = tick.outcomes.iter().filter(|o| o.receipt.is_some()).count();
        let posture = self.posture.observe(signal_count);

        let mut chain_extended = false;
        if signal_count > 0 {
            if let Some(prev_iri) = self.last_receipt_iri.clone() {
                let new_iri_opt = tick.outcomes.iter()
                    .find_map(|o| o.receipt.as_ref().map(|r| r.activity_iri.clone()));
                if let Some(new_iri) = new_iri_opt {
                    let mut chain = Construct8::empty();
                    let was_informed_by = NamedNode::new("http://www.w3.org/ns/prov#wasInformedBy")?;
                    let new_node: NamedNode = new_iri.into();
                    let prev_node: NamedNode = prev_iri.into();
                    chain.push(Triple::new(new_node, was_informed_by, Term::NamedNode(prev_node)));
                    chain.materialize(&self.field.graph)?;
                    chain_extended = true;
                }
            }
        }

        // BLAKE3 receipt chain: thread every receipted outcome through Powl64
        // in scheduler-emission order. Layers underneath the public PROV-O
        // wasInformedBy chain without touching ontology.
        for outcome in tick.outcomes.iter() {
            if let Some(receipt) = outcome.receipt.as_ref() {
                self.powl64.extend(&receipt.activity_iri, 1);
            }
        }

        if let Some(latest) = tick.outcomes.iter().rev()
            .find_map(|o| o.receipt.as_ref().map(|r| r.activity_iri.clone()))
        {
            self.last_receipt_iri = Some(latest);
        }

        Ok(StepReport {
            tick,
            posture,
            chain_extended,
            chain_head: self.powl64.chain_head(),
        })
    }
}
