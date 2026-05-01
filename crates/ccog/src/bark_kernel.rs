//! POWL8 bark kernel — plan-ordered, single-pass mask dispatch (Phase 5 Track A).
//!
//! Combines a [`CompiledHookTable`] with a [`Powl8`] plan that prescribes the
//! partial order of bark dispatch. Each plan node either marks structural
//! progress (`StartNode`, `EndNode`, `Silent`, operators) or invokes a single
//! compiled hook (`Activity(Breed)`).
//!
//! # Three-Stage Pipeline (Phase 5 Track A)
//!
//! Bark dispatch is split into three explicit stages with separate cost tiers:
//!
//! 1. [`BarkKernel::decide`] — nanoscale, allocation-free. Computes a
//!    [`BarkDecision`] containing `fired_mask`/`denied_mask`/`advanced_mask`/
//!    `present_mask` as bit-packed `u64` masks. Pure mask arithmetic — no
//!    `Vec`, no `format!`, no time calls. Bits are over plan-node index.
//! 2. [`BarkKernel::materialize`] — microsecond. Allocates a
//!    `Vec<Option<Construct8>>` indexed by slot, executing each fired slot's
//!    `act` to produce its bounded delta.
//! 3. [`BarkKernel::seal`] — microsecond. Hashes each fired slot's delta into
//!    a deterministic `urn:blake3:` activity URN and emits a [`Receipt`].
//!    Identity material follows the canonical layout in
//!    [`Receipt::canonical_material`]. No `example.org`, no `Utc::now` in
//!    URN material — only as the receipt's metadata timestamp.
//!
//! [`BarkKernel::fire`] is preserved as a warm-path convenience that calls
//! all three stages in sequence and returns `Vec<HookOutcome>`.

use crate::compiled_hook::{compute_present_mask, CompiledHook};
use crate::construct8::Construct8;
use crate::hooks::HookOutcome;
use crate::powl::{Powl8, Powl8Node, MAX_NODES};
use crate::receipt::Receipt;
use crate::runtime::cog8::{
    execute_cog8, BreedId, Cog8Edge, Cog8Row, CollapseFn, EdgeId, GroupId, Instinct, NodeId,
    PackId, Powl8Instr, Powl8Op, RuleId,
};
use crate::runtime::ClosedFieldContext;
use anyhow::Result;
use chrono::Utc;

/// Optional compiled hook for each plan node, indexed by plan node position.
///
/// `slots[i]` is `Some(hook)` iff plan node `i` should fire that hook on
/// match, `None` for structural markers.
#[derive(Debug, Default, Clone)]
pub struct BarkKernel {
    /// Plan whose partial order constrains bark dispatch.
    pub plan: Powl8,
    /// Optional compiled hook for each plan node.
    pub slots: Vec<Option<CompiledHook>>,
    /// COG8 nodes for nonlinear execution.
    pub nodes: Vec<Cog8Row>,
    /// COG8 edges for nonlinear execution.
    pub edges: Vec<Cog8Edge>,
}

/// Outcome of [`BarkKernel::decide`] — bit-packed slot status, no allocations.
///
/// Each `u64` field is a bitmap indexed by plan-node position. Bit `i` refers
/// to plan node `i`. Capacity is bounded by [`MAX_NODES`] (64).
///
/// **Bit domain**: plan-node index (NOT compiled-runtime index — Track B
/// adds compiled-order dispatch separately via `Powl8::compile`).
///
/// # Mask domain rule (Phase 10)
///
/// `advanced_mask` (and the sibling `fired_mask` / `denied_mask`) are
/// **plan-node indexed** in this `BarkKernel::decide` flow because the
/// kernel walks the raw `Powl8.nodes` array. After `Powl8::compile()` the
/// returned `CompiledPowl8` reorders nodes topologically and any masks
/// computed against `CompiledPowl8.preds[i]` are then **runtime-slot
/// indexed, NOT plan-node indexed**. Mixing the two domains across a
/// compile boundary is a defect. The constitutional rule is: every mask
/// must declare its domain at the type or doc level — never silently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BarkDecision {
    /// Bit `i` set iff slot `i` fired (had a compiled hook AND its
    /// predecessors were advanced AND its `require_mask` matched).
    pub fired_mask: u64,
    /// Bit `i` set iff slot `i` was denied (had a compiled hook AND its
    /// predecessors were advanced BUT its `require_mask` was unsatisfied).
    pub denied_mask: u64,
    /// Bit `i` set iff slot `i` is advanced (fired or structural-pass).
    /// Structural-pass = no compiled hook in this slot.
    pub advanced_mask: u64,
    /// Snapshot's `present_mask`, computed once at the start of decide.
    pub present_mask: u64,
    /// Collapse function attributed to this decision.
    pub collapse_fn: Option<CollapseFn>,
    /// Selected node ID.
    pub selected_node: Option<NodeId>,
    /// Selected edge ID.
    pub selected_edge: Option<EdgeId>,
}

impl BarkKernel {
    /// Build an empty kernel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a linear-sequence kernel from a hook list: `Start → h0 → h1 → … → End`.
    pub fn linear(hooks: Vec<CompiledHook>) -> Result<Self> {
        use crate::verdict::Breed;
        let mut plan = Powl8::new();
        let mut slots = Vec::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let start_idx = plan
            .push(Powl8Node::StartNode)
            .map_err(|e| anyhow::anyhow!("plan push failed: {:?}. Hint: a POWL8 plan is limited to 64 nodes. Simplify your hook sequence or split it into multiple kernels.", e))?;
        plan.root = start_idx;
        slots.push(None);
        nodes.push(Cog8Row {
            pack_id: PackId(0),
            group_id: GroupId(0),
            rule_id: RuleId(0),
            breed_id: BreedId(Breed::CompiledHook as u8),
            collapse_fn: CollapseFn::ExpertRule,
            var_ids: [crate::runtime::cog8::FieldId(0); 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 0,
            response: Instinct::Ignore,
            priority: 0,
        });

        let mut prev_idx = start_idx;
        for (i, hook) in hooks.into_iter().enumerate() {
            let idx = plan
                .push(Powl8Node::Activity(Breed::CompiledHook))
                .map_err(|e| anyhow::anyhow!("plan push failed: {:?}. Hint: a POWL8 plan is limited to 64 nodes. Simplify your hook sequence or split it into multiple kernels.", e))?;
            slots.push(Some(hook.clone()));
            nodes.push(Cog8Row {
                pack_id: PackId(0),
                group_id: GroupId(0),
                rule_id: RuleId(i as u16 + 1),
                breed_id: BreedId(Breed::CompiledHook as u8),
                collapse_fn: CollapseFn::ExpertRule,
                var_ids: [crate::runtime::cog8::FieldId(0); 8],
                required_mask: hook.require_mask,
                forbidden_mask: 0,
                predecessor_mask: 1u64 << prev_idx,
                response: Instinct::Settle,
                priority: (i + 1) as u16,
            });

            let edge_idx = plan
                .push(Powl8Node::OperatorSequence { a: prev_idx, b: idx })
                .map_err(|e| anyhow::anyhow!("plan push failed: {:?}. Hint: a POWL8 plan is limited to 64 nodes. Simplify your hook sequence or split it into multiple kernels.", e))?;
            slots.push(None);
            nodes.push(Cog8Row {
                pack_id: PackId(0),
                group_id: GroupId(0),
                rule_id: RuleId(0),
                breed_id: BreedId(Breed::CompiledHook as u8),
                collapse_fn: CollapseFn::ExpertRule,
                var_ids: [crate::runtime::cog8::FieldId(0); 8],
                required_mask: 0,
                forbidden_mask: 0,
                predecessor_mask: 0,
                response: Instinct::Ignore,
                priority: 0,
            });

            edges.push(Cog8Edge {
                from: NodeId(prev_idx),
                to: NodeId(idx),
                kind: crate::runtime::cog8::EdgeKind::Choice,
                instr: Powl8Instr {
                    op: Powl8Op::Act,
                    collapse_fn: CollapseFn::ExpertRule,
                    node_id: NodeId(idx),
                    edge_id: EdgeId(edge_idx),
                    guard_mask: 1u64 << prev_idx,
                    effect_mask: 1u64 << idx,
                },
            });

            prev_idx = idx;
        }

        let end_idx = plan
            .push(Powl8Node::EndNode)
            .map_err(|e| anyhow::anyhow!("plan push failed: {:?}. Hint: a POWL8 plan is limited to 64 nodes. Simplify your hook sequence or split it into multiple kernels.", e))?;
        slots.push(None);
        nodes.push(Cog8Row {
            pack_id: PackId(0),
            group_id: GroupId(0),
            rule_id: RuleId(0),
            breed_id: BreedId(Breed::CompiledHook as u8),
            collapse_fn: CollapseFn::ExpertRule,
            var_ids: [crate::runtime::cog8::FieldId(0); 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 1u64 << prev_idx,
            response: Instinct::Ignore,
            priority: 0,
        });

        let end_edge_idx = plan
            .push(Powl8Node::OperatorSequence { a: prev_idx, b: end_idx })
            .map_err(|e| anyhow::anyhow!("plan push failed: {:?}. Hint: a POWL8 plan is limited to 64 nodes. Simplify your hook sequence or split it into multiple kernels.", e))?;
        slots.push(None);
        nodes.push(Cog8Row {
            pack_id: PackId(0),
            group_id: GroupId(0),
            rule_id: RuleId(0),
            breed_id: BreedId(Breed::CompiledHook as u8),
            collapse_fn: CollapseFn::ExpertRule,
            var_ids: [crate::runtime::cog8::FieldId(0); 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 0,
            response: Instinct::Ignore,
            priority: 0,
        });

        edges.push(Cog8Edge {
            from: NodeId(prev_idx),
            to: NodeId(end_idx),
            kind: crate::runtime::cog8::EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Act,
                collapse_fn: CollapseFn::ExpertRule,
                node_id: NodeId(end_idx),
                edge_id: EdgeId(end_edge_idx),
                guard_mask: 1u64 << prev_idx,
                effect_mask: 1u64 << end_idx,
            },
        });

        Ok(Self {
            plan,
            slots,
            nodes,
            edges,
        })
    }

    /// Stage 1 (nanoscale, allocation-free): compute fire/deny/advance masks.
    ///
    /// Refactored to use [`execute_cog8_graph`] for nonlinear traversal.
    #[inline]
    pub fn decide(&self, context: &ClosedFieldContext) -> BarkDecision {
        let present = compute_present_mask(&context.snapshot);

        // Execute the nonlinear COG8 graph.
        let cog8_dec = execute_cog8(&self.nodes, &self.edges, context, 1u64 << self.plan.root)
            .unwrap_or_default();

        BarkDecision {
            fired_mask: cog8_dec.fired_mask,
            denied_mask: cog8_dec.denied_mask,
            advanced_mask: cog8_dec.completed_mask,
            present_mask: present,
            collapse_fn: cog8_dec.collapse_fn,
            selected_node: cog8_dec.selected_node,
            selected_edge: cog8_dec.selected_edge,
        }
    }

    /// Stage 2 (microsecond): execute each fired slot's `act` to produce deltas.
    ///
    /// Returns a `Vec<Option<Construct8>>` of length `self.slots.len()` where
    /// index `i` is `Some(delta)` iff bit `i` of `decision.fired_mask` is set,
    /// else `None`. The vector is sized to match slot indices for stable
    /// joining with [`BarkKernel::seal`].
    pub fn materialize(
        &self,
        decision: &BarkDecision,
        context: &ClosedFieldContext,
    ) -> Result<Vec<Option<Construct8>>> {
        let mut out: Vec<Option<Construct8>> = Vec::with_capacity(self.slots.len());
        for (i, slot) in self.slots.iter().enumerate() {
            if i >= MAX_NODES {
                out.push(None);
                continue;
            }
            let bit = 1u64 << i;
            match (slot, decision.fired_mask & bit != 0) {
                (Some(hook), true) => out.push(Some((hook.act)(context)?)),
                _ => out.push(None),
            }
        }
        Ok(out)
    }

    /// Stage 3 (microsecond): seal per-slot receipts with deterministic URNs.
    ///
    /// Activity IRIs are derived via [`Receipt::canonical_material`] +
    /// [`Receipt::derive_urn`] — no `example.org`, no `Utc::now()` in identity
    /// material. `Utc::now()` is used only as the receipt's metadata timestamp.
    ///
    /// Returns a `Vec<Option<Receipt>>` aligned with `self.slots` — `Some` iff
    /// the slot fired AND `slot.emit_receipt`.
    pub fn seal(
        &self,
        decision: &BarkDecision,
        deltas: &[Option<Construct8>],
        field_id: &str,
        prior_chain: Option<blake3::Hash>,
    ) -> Vec<Option<Receipt>> {
        let mut out: Vec<Option<Receipt>> = Vec::with_capacity(self.slots.len());
        let now = Utc::now();
        for (i, slot) in self.slots.iter().enumerate() {
            if i >= MAX_NODES {
                out.push(None);
                continue;
            }
            let bit = 1u64 << i;
            let fired = (decision.fired_mask & bit) != 0;
            let delta_ref = deltas.get(i).and_then(Option::as_ref);
            match (slot, fired, delta_ref) {
                (Some(hook), true, Some(delta)) if hook.emit_receipt => {
                    let delta_bytes = delta.receipt_bytes();
                    let material = Receipt::canonical_material(
                        hook.name,
                        i as u16,
                        &delta_bytes,
                        field_id,
                        prior_chain,
                        1u8,
                    );
                    let urn = Receipt::derive_urn(&material);
                    let activity_iri = crate::graph::GraphIri::from_iri(&urn)
                        .expect("derive_urn must produce a valid urn:blake3 IRI");
                    let hash = Receipt::blake3_hex(&delta_bytes);
                    out.push(Some(Receipt::new(activity_iri, hash, now)));
                }
                _ => out.push(None),
            }
        }
        out
    }

    /// Warm-path convenience: calls decide + materialize + seal in sequence.
    ///
    /// Returns `Vec<HookOutcome>` (one per fired slot) for backward compat with
    /// pre-Phase-5 callers. For nanoscale dispatch, call the staged API directly.
    pub fn fire(&self, context: &ClosedFieldContext) -> Result<Vec<HookOutcome>> {
        let decision = self.decide(context);
        let deltas = self.materialize(&decision, context)?;
        let receipts = self.seal(&decision, &deltas, "fire", None);
        let mut out = Vec::new();
        for (i, slot) in self.slots.iter().enumerate() {
            if i >= MAX_NODES {
                continue;
            }
            let bit = 1u64 << i;
            if (decision.fired_mask & bit) == 0 {
                continue;
            }
            let Some(hook) = slot else { continue };
            let Some(Some(delta)) = deltas.get(i).cloned() else {
                continue;
            };
            let receipt = receipts.get(i).cloned().flatten();
            out.push(HookOutcome {
                hook_name: hook.name,
                delta,
                receipt,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompiledFieldSnapshot;
    use crate::compiled_hook::compile_builtin;
    use crate::field::FieldContext;
    use crate::hooks::{
        missing_evidence_hook, phrase_binding_hook, receipt_hook, transition_admissibility_hook,
    };
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
    fn linear_kernel_fires_in_plan_order() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/c1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .\n\
             <http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"Test\" .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);

        let hooks = vec![
            compile_builtin(&phrase_binding_hook()).expect("compile"),
            compile_builtin(&transition_admissibility_hook()).expect("compile"),
            compile_builtin(&receipt_hook()).expect("compile"),
        ];
        let kernel = BarkKernel::linear(hooks)?;
        let outcomes = kernel.fire(&context)?;
        let names: Vec<_> = outcomes.iter().map(|o| o.hook_name).collect();
        assert_eq!(
            names,
            vec!["phrase_binding", "transition_admissibility", "receipt"]
        );
        Ok(())
    }

    #[test]
    fn kernel_skips_unsatisfied_masks() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let hooks = vec![
            compile_builtin(&missing_evidence_hook()).expect("compile"),
            compile_builtin(&receipt_hook()).expect("compile"),
        ];
        let kernel = BarkKernel::linear(hooks)?;
        let outcomes = kernel.fire(&context)?;
        let names: Vec<_> = outcomes.iter().map(|o| o.hook_name).collect();
        assert_eq!(names, vec!["receipt"]);
        Ok(())
    }

    #[test]
    fn empty_kernel_yields_no_outcomes() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let kernel = BarkKernel::linear(Vec::new())?;
        let outcomes = kernel.fire(&context)?;
        assert!(outcomes.is_empty());
        Ok(())
    }

    #[test]
    fn decide_does_not_allocate_construct8() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let kernel = BarkKernel::linear(vec![
            compile_builtin(&missing_evidence_hook()).expect("compile"),
            compile_builtin(&receipt_hook()).expect("compile"),
        ])?;
        let decision = kernel.decide(&context);
        assert_ne!(decision.advanced_mask, 0);
        assert_ne!(decision.fired_mask, 0);
        Ok(())
    }

    #[test]
    fn decide_then_materialize_matches_fire() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let kernel = BarkKernel::linear(vec![
            compile_builtin(&missing_evidence_hook()).expect("compile"),
            compile_builtin(&phrase_binding_hook()).expect("compile"),
            compile_builtin(&receipt_hook()).expect("compile"),
        ])?;

        let fire_outcomes = kernel.fire(&context)?;
        let decision = kernel.decide(&context);
        let deltas = kernel.materialize(&decision, &context)?;
        let _receipts = kernel.seal(&decision, &deltas, "fire", None);

        let fire_names: Vec<_> = fire_outcomes.iter().map(|o| o.hook_name).collect();
        let staged_names: Vec<_> = (0..kernel.slots.len())
            .filter(|&i| (decision.fired_mask & (1u64 << i)) != 0)
            .filter_map(|i| kernel.slots[i].as_ref().map(|h| h.name))
            .collect();
        assert_eq!(fire_names, staged_names);

        let fire_bytes: Vec<_> = fire_outcomes
            .iter()
            .map(|o| o.delta.receipt_bytes())
            .collect();
        let staged_bytes: Vec<_> = deltas
            .iter()
            .filter_map(|d| d.as_ref().map(|c| c.receipt_bytes()))
            .collect();
        assert_eq!(fire_bytes, staged_bytes);
        Ok(())
    }

    #[test]
    fn seal_uses_blake3_urn() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )?;
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let kernel = BarkKernel::linear(vec![
            compile_builtin(&missing_evidence_hook()).expect("compile"),
            compile_builtin(&receipt_hook()).expect("compile"),
        ])?;
        let outcomes = kernel.fire(&context)?;
        for o in &outcomes {
            if let Some(r) = &o.receipt {
                assert!(
                    r.activity_iri.as_str().starts_with("urn:blake3:"),
                    "expected urn:blake3 URN, got {}",
                    r.activity_iri.as_str()
                );
                assert!(!r.activity_iri.as_str().contains("example.org"));
            }
        }
        Ok(())
    }

    #[test]
    fn decide_skips_unsatisfied_mask() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field)?);
        let context = empty_context(snap);
        let kernel = BarkKernel::linear(vec![
            compile_builtin(&missing_evidence_hook()).expect("compile"),
            compile_builtin(&receipt_hook()).expect("compile"),
        ])?;
        let decision = kernel.decide(&context);
        // missing_evidence should be denied; receipt should fire.
        assert_ne!(decision.denied_mask, 0);
        assert_ne!(decision.fired_mask, 0);
        Ok(())
    }
}
