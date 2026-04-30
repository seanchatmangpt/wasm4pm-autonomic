//! POWL8 bark kernel — plan-ordered, single-pass mask dispatch (Phase 4 Stage 3).
//!
//! Combines a [`CompiledHookTable`] with a [`Powl8`] plan that prescribes the
//! partial order of bark dispatch. Each plan node either marks structural
//! progress (`StartNode`, `EndNode`, `Silent`, operators) or invokes a single
//! compiled hook (`Activity(Breed)`).
//!
//! The kernel walks the plan once, maintaining a 64-bit "advanced" mask. A
//! node fires iff:
//!
//! 1. Its predecessors per [`Powl8::predecessor_masks`] are all advanced.
//! 2. Its compiled hook's `require_mask & present_mask == require_mask`.
//!
//! This collapses bark dispatch into a few u64 operations per node.

use crate::compiled::CompiledFieldSnapshot;
use crate::compiled_hook::{compute_present_mask, CompiledHook};
use crate::hooks::HookOutcome;
use crate::powl::{Powl8, Powl8Node, MAX_NODES};
use crate::receipt::Receipt;
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
}

impl BarkKernel {
    /// Build an empty kernel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a linear-sequence kernel from a hook list: `Start → h0 → h1 → … → End`.
    ///
    /// Nodes are inserted as: `[StartNode, Activity(Eliza)*, EndNode]` plus
    /// `OperatorSequence` edges chaining them in registration order. The
    /// concrete [`crate::verdict::Breed`] is `Eliza` purely as a placeholder
    /// for activity nodes whose semantics are carried by the compiled hook.
    pub fn linear(hooks: Vec<CompiledHook>) -> Result<Self> {
        use crate::verdict::Breed;
        let mut plan = Powl8::new();
        let mut slots = Vec::new();

        let start = plan
            .push(Powl8Node::StartNode)
            .map_err(|e| anyhow::anyhow!("plan push failed: {:?}", e))?;
        plan.root = start;
        slots.push(None);

        let mut prev = start;
        for hook in hooks {
            let idx = plan
                .push(Powl8Node::Activity(Breed::Eliza))
                .map_err(|e| anyhow::anyhow!("plan push failed: {:?}", e))?;
            slots.push(Some(hook));
            plan.push(Powl8Node::OperatorSequence { a: prev, b: idx })
                .map_err(|e| anyhow::anyhow!("plan push failed: {:?}", e))?;
            slots.push(None);
            prev = idx;
        }

        let end = plan
            .push(Powl8Node::EndNode)
            .map_err(|e| anyhow::anyhow!("plan push failed: {:?}", e))?;
        slots.push(None);
        plan.push(Powl8Node::OperatorSequence { a: prev, b: end })
            .map_err(|e| anyhow::anyhow!("plan push failed: {:?}", e))?;
        slots.push(None);

        Ok(Self { plan, slots })
    }

    /// Walk the plan in node order, firing slots as their predecessors clear.
    ///
    /// Returns the firing order's outcomes. A slot's hook executes iff:
    /// - All predecessor nodes are advanced.
    /// - Its `require_mask & present_mask == require_mask`.
    ///
    /// Otherwise the node does not fire but is still marked advanced — hooks
    /// downstream of an unsatisfied mask are not gated by the unfiring.
    pub fn fire(&self, snap: &CompiledFieldSnapshot) -> Result<Vec<HookOutcome>> {
        let present = compute_present_mask(snap);
        let preds = self.plan.predecessor_masks();
        let n = self.plan.nodes.len();

        let mut advanced: u64 = 0;
        let mut outcomes = Vec::new();

        for i in 0..n.min(MAX_NODES) {
            let need = preds[i];
            if (need & advanced) != need {
                continue;
            }
            if let Some(Some(hook)) = self.slots.get(i) {
                if (hook.require_mask & present) == hook.require_mask {
                    let delta = (hook.act)(snap)?;
                    let receipt = if hook.emit_receipt {
                        let activity = crate::graph::GraphIri::from_iri(&format!(
                            "http://example.org/hook/{}#{}",
                            hook.name,
                            Utc::now().timestamp()
                        ))?;
                        let hash = Receipt::blake3_hex(&delta.receipt_bytes());
                        Some(Receipt::new(activity, hash, Utc::now()))
                    } else {
                        None
                    };
                    outcomes.push(HookOutcome {
                        hook_name: hook.name,
                        delta,
                        receipt,
                    });
                }
            }
            advanced |= 1u64 << i;
        }

        Ok(outcomes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled_hook::compile_builtin;
    use crate::field::FieldContext;
    use crate::hooks::{
        missing_evidence_hook, phrase_binding_hook, receipt_hook, transition_admissibility_hook,
    };

    #[test]
    fn linear_kernel_fires_in_plan_order() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/c1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .\n\
             <http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"Test\" .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;

        let hooks = vec![
            compile_builtin(&phrase_binding_hook()).expect("compile"),
            compile_builtin(&transition_admissibility_hook()).expect("compile"),
            compile_builtin(&receipt_hook()).expect("compile"),
        ];
        let kernel = BarkKernel::linear(hooks)?;
        let outcomes = kernel.fire(&snap)?;
        let names: Vec<_> = outcomes.iter().map(|o| o.hook_name).collect();
        assert_eq!(names, vec!["phrase_binding", "transition_admissibility", "receipt"]);
        Ok(())
    }

    #[test]
    fn kernel_skips_unsatisfied_masks() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let hooks = vec![
            compile_builtin(&missing_evidence_hook()).expect("compile"),
            compile_builtin(&receipt_hook()).expect("compile"),
        ];
        let kernel = BarkKernel::linear(hooks)?;
        let outcomes = kernel.fire(&snap)?;
        let names: Vec<_> = outcomes.iter().map(|o| o.hook_name).collect();
        assert_eq!(names, vec!["receipt"]);
        Ok(())
    }

    #[test]
    fn empty_kernel_yields_no_outcomes() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let kernel = BarkKernel::linear(Vec::new())?;
        let outcomes = kernel.fire(&snap)?;
        assert!(outcomes.is_empty());
        Ok(())
    }
}
