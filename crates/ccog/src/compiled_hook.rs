//! Compiled hook table with bitmask predicate dispatch (Phase 4 Stage 2).
//!
//! Each hook compiles to a `(require_mask, act_fn)` pair. The runtime
//! computes a single `present_mask` from the snapshot once per fire and
//! dispatches every hook with one AND-equality check:
//!
//! ```text
//! if (hook.require_mask & present_mask) == hook.require_mask { act(snapshot) }
//! ```
//!
//! This collapses trigger+check into a u64 AND. Built-in hooks share a
//! canonical predicate vocabulary defined as `Predicate::*` constants below.

use crate::compiled::CompiledFieldSnapshot;
use crate::construct8::Construct8;
use crate::hooks::{HookCheck, HookOutcome, HookTrigger, KnowledgeHook};
use crate::receipt::Receipt;
use anyhow::Result;
use chrono::Utc;
use oxigraph::model::NamedNode;

/// Canonical predicate-bit assignments for compiled hook dispatch.
///
/// Each bit names a single condition over `CompiledFieldSnapshot`. Hooks
/// reference these by `1u64 << Predicate::X` in their `require_mask`.
#[allow(non_snake_case)]
pub mod Predicate {
    /// At least one `schema:DigitalDocument` instance exists.
    pub const DD_PRESENT: u32 = 0;
    /// At least one `schema:DigitalDocument` lacks a `prov:value`.
    pub const DD_MISSING_PROV_VALUE: u32 = 1;
    /// At least one triple uses `skos:prefLabel`.
    pub const HAS_PREF_LABEL: u32 = 2;
    /// At least one triple uses `rdf:type`.
    pub const HAS_RDF_TYPE: u32 = 3;
}

/// Compute the `present_mask` for the canonical predicate set from a snapshot.
///
/// O(N) in the number of `schema:DigitalDocument` instances at worst, but
/// runs once per fire and amortizes across all hooks.
pub fn compute_present_mask(snap: &CompiledFieldSnapshot) -> u64 {
    use std::sync::OnceLock;
    static DD: OnceLock<NamedNode> = OnceLock::new();
    static PV: OnceLock<NamedNode> = OnceLock::new();
    static PL: OnceLock<NamedNode> = OnceLock::new();
    static RT: OnceLock<NamedNode> = OnceLock::new();
    // Each predicate IRI heap-allocs once across the process. After warmup,
    // `compute_present_mask` becomes alloc-free — load-bearing for the
    // KernelFloor budget (gauntlet test
    // `gauntlet_compute_present_mask_zero_alloc`).
    let dd = DD.get_or_init(|| {
        NamedNode::new("https://schema.org/DigitalDocument")
            .expect("Invalid schema:DigitalDocument IRI")
    });
    let pv = PV.get_or_init(|| {
        NamedNode::new("http://www.w3.org/ns/prov#value").expect("Invalid prov:value IRI")
    });
    let pl = PL.get_or_init(|| {
        NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")
            .expect("Invalid skos:prefLabel IRI")
    });
    let rt = RT.get_or_init(|| {
        NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
            .expect("Invalid rdf:type IRI")
    });

    let mut mask = 0u64;

    let dd_instances = snap.instances_of(dd);
    if !dd_instances.is_empty() {
        mask |= 1u64 << Predicate::DD_PRESENT;
        for d in dd_instances {
            if !snap.has_value_for(d, pv) {
                mask |= 1u64 << Predicate::DD_MISSING_PROV_VALUE;
                break;
            }
        }
    }
    if snap.has_any_with_predicate(pl) {
        mask |= 1u64 << Predicate::HAS_PREF_LABEL;
    }
    if snap.has_any_with_predicate(rt) {
        mask |= 1u64 << Predicate::HAS_RDF_TYPE;
    }
    mask
}

/// A hook compiled to a bitmask predicate over the canonical predicate set.
///
/// `require_mask = 0` means "always fires" (e.g., the manual receipt hook).
#[derive(Clone)]
pub struct CompiledHook {
    /// Hook identifier propagated to the outcome.
    pub name: &'static str,
    /// AND-mask of canonical predicate bits this hook requires to fire.
    pub require_mask: u64,
    /// Snapshot-driven action emitted when the mask matches.
    pub act: fn(&CompiledFieldSnapshot) -> Result<Construct8>,
    /// Whether to emit a PROV receipt with the outcome.
    pub emit_receipt: bool,
}

impl std::fmt::Debug for CompiledHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledHook")
            .field("name", &self.name)
            .field("require_mask", &format_args!("{:#066b}", self.require_mask))
            .field("emit_receipt", &self.emit_receipt)
            .finish()
    }
}

/// Compiled hook registry. Single-pass mask dispatch over a single snapshot.
#[derive(Debug, Default, Clone)]
pub struct CompiledHookTable {
    /// Compiled hooks evaluated in registration order.
    pub hooks: Vec<CompiledHook>,
}

impl CompiledHookTable {
    /// Build a new empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a compiled hook.
    pub fn register(&mut self, hook: CompiledHook) {
        self.hooks.push(hook);
    }

    /// Single-pass mask dispatch:
    ///
    /// 1. Compute `present_mask` once from the snapshot.
    /// 2. For each hook, fire iff `(require_mask & present_mask) == require_mask`.
    /// 3. Receipts are emitted from the action's `Construct8` BLAKE3.
    pub fn fire(&self, snap: &CompiledFieldSnapshot) -> Result<Vec<HookOutcome>> {
        let present = compute_present_mask(snap);
        let mut out = Vec::new();
        for hook in &self.hooks {
            if (hook.require_mask & present) != hook.require_mask {
                continue;
            }
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
            out.push(HookOutcome { hook_name: hook.name, delta, receipt });
        }
        Ok(out)
    }
}

/// Compile a built-in `KnowledgeHook` into a `CompiledHook`.
///
/// The four built-in hooks have known canonical mask shapes; this function
/// recognizes them by name. Unrecognized hooks fall back to `require_mask=0`
/// (always fires) — the caller should ensure the act remains snapshot-driven.
pub fn compile_builtin(hook: &KnowledgeHook) -> Option<CompiledHook> {
    let act = match &hook.act {
        crate::hooks::HookAct::SnapshotFn(f) => *f,
        _ => return None,
    };
    let require_mask = match (&hook.trigger, &hook.check, hook.name) {
        (HookTrigger::TypePresent(_), HookCheck::SnapshotFn(_), "missing_evidence") => {
            (1u64 << Predicate::DD_PRESENT) | (1u64 << Predicate::DD_MISSING_PROV_VALUE)
        }
        (HookTrigger::Pattern { .. }, HookCheck::SnapshotFn(_), "phrase_binding") => {
            1u64 << Predicate::HAS_PREF_LABEL
        }
        (HookTrigger::Pattern { .. }, HookCheck::SnapshotFn(_), "transition_admissibility") => {
            1u64 << Predicate::HAS_RDF_TYPE
        }
        (HookTrigger::Always, HookCheck::SnapshotFn(_), "receipt") => 0u64,
        _ => return None,
    };
    Some(CompiledHook {
        name: hook.name,
        require_mask,
        act,
        emit_receipt: hook.emit_receipt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::FieldContext;
    use crate::hooks::{
        missing_evidence_hook, phrase_binding_hook, receipt_hook, transition_admissibility_hook,
    };

    #[test]
    fn present_mask_detects_dd_and_gap() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let mask = compute_present_mask(&snap);
        assert_ne!(mask & (1u64 << Predicate::DD_PRESENT), 0);
        assert_ne!(mask & (1u64 << Predicate::DD_MISSING_PROV_VALUE), 0);
        assert_ne!(mask & (1u64 << Predicate::HAS_RDF_TYPE), 0);
        assert_eq!(mask & (1u64 << Predicate::HAS_PREF_LABEL), 0);
        Ok(())
    }

    #[test]
    fn compiled_table_fires_missing_evidence() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let mut table = CompiledHookTable::new();
        table.register(compile_builtin(&missing_evidence_hook()).expect("compile"));
        let outcomes = table.fire(&snap)?;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].hook_name, "missing_evidence");
        Ok(())
    }

    #[test]
    fn compiled_table_skips_when_mask_unmet() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let mut table = CompiledHookTable::new();
        table.register(compile_builtin(&missing_evidence_hook()).expect("compile"));
        table.register(compile_builtin(&phrase_binding_hook()).expect("compile"));
        table.register(compile_builtin(&transition_admissibility_hook()).expect("compile"));
        let outcomes = table.fire(&snap)?;
        assert!(outcomes.is_empty());
        Ok(())
    }

    #[test]
    fn compiled_table_fires_receipt_unconditionally() -> Result<()> {
        let field = FieldContext::new("test");
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let mut table = CompiledHookTable::new();
        table.register(compile_builtin(&receipt_hook()).expect("compile"));
        let outcomes = table.fire(&snap)?;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].hook_name, "receipt");
        Ok(())
    }

    #[test]
    fn compile_builtin_recognizes_all_four() {
        assert!(compile_builtin(&missing_evidence_hook()).is_some());
        assert!(compile_builtin(&phrase_binding_hook()).is_some());
        assert!(compile_builtin(&transition_admissibility_hook()).is_some());
        assert!(compile_builtin(&receipt_hook()).is_some());
    }
}
