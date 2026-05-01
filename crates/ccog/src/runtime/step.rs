//! Multi-cycle runtime driver: composes Scheduler + PostureMachine + receipt chain.

use crate::construct8::Construct8;
use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::hooks::HookRegistry;
use crate::powl64::Powl64;
use crate::runtime::error::{Result, RuntimeError};
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
    pub fn field(&self) -> &FieldContext {
        &self.field
    }

    /// Mutable field access for loading state between steps.
    pub fn field_mut(&mut self) -> &mut FieldContext {
        &mut self.field
    }

    /// Current posture without mutation.
    pub fn posture(&self) -> PackPosture {
        self.posture.current()
    }

    /// Borrow the BLAKE3 receipt chain universe.
    pub fn powl64(&self) -> &Powl64 {
        &self.powl64
    }

    /// Run one cycle: tick → observe posture → chain receipts → return report.
    pub fn step(&mut self) -> Result<StepReport> {
        let tick = self.scheduler.tick(&self.field)?;
        let signal_count = tick.outcomes.iter().filter(|o| o.receipt.is_some()).count();
        let posture = self.posture.observe(signal_count);

        let mut chain_extended = false;
        if signal_count > 0 {
            if let Some(prev_iri) = self.last_receipt_iri.clone() {
                let new_iri_opt = tick
                    .outcomes
                    .iter()
                    .find_map(|o| o.receipt.as_ref().map(|r| r.activity_iri.clone()));
                if let Some(new_iri) = new_iri_opt {
                    use crate::construct8::Triple;
                    let mut chain = Construct8::empty();
                    let was_informed_by = "http://www.w3.org/ns/prov#wasInformedBy";
                    let new_node_str = new_iri.as_str();
                    let prev_node_str = prev_iri.as_str();
                    chain.push(Triple::from_strings(
                        new_node_str,
                        was_informed_by,
                        prev_node_str,
                    ));
                    chain
                        .materialize(&self.field.graph)
                        .map_err(|e| RuntimeError::FieldError(e.to_string()))?;
                    chain_extended = true;
                }
            }
        }

        // BLAKE3 receipt chain: thread every receipted outcome through Powl64
        // in scheduler-emission order. Layers underneath the public PROV-O
        // wasInformedBy chain without touching ontology.
        for outcome in tick.outcomes.iter() {
            if let Some(receipt) = outcome.receipt.as_ref() {
                use crate::powl64::{PartnerId, Polarity, Powl64RouteCell, ProjectionTarget};
                use crate::runtime::cog8::CollapseFn;

                let cell = Powl64RouteCell {
                    graph_id: 0, // Placeholder
                    from_node: Default::default(),
                    to_node: Default::default(),
                    edge_id: Default::default(),
                    edge_kind: Default::default(),
                    collapse_fn: CollapseFn::None,
                    polarity: Polarity::Positive,
                    projection_target: ProjectionTarget::NoOp,
                    partner_id: PartnerId::NONE,
                    input_digest: 0,
                    args_digest: 0,
                    result_digest: 0,
                    prior_chain: self.powl64.chain_head().unwrap_or(0),
                    chain_head: u64::from_str_radix(&receipt.hash[..16], 16).unwrap_or(0),
                };
                self.powl64.extend(cell);
            }
        }

        if let Some(latest) = tick
            .outcomes
            .iter()
            .rev()
            .find_map(|o| o.receipt.as_ref().map(|r| r.activity_iri.clone()))
        {
            self.last_receipt_iri = Some(latest);
        }

        let chain_head_u64 = self.powl64.chain_head();
        let chain_head_hash = chain_head_u64.map(|h| {
            let mut bytes = [0u8; 32];
            bytes[..8].copy_from_slice(&h.to_le_bytes());
            blake3::Hash::from_bytes(bytes)
        });

        Ok(StepReport {
            tick,
            posture,
            chain_extended,
            chain_head: chain_head_hash,
        })
    }
}
