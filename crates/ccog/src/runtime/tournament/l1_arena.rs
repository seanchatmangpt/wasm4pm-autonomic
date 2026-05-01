//! L1 Reflex ICE Arena (PRD v0.4).
//!
//! High-performance reflexive execution environment for COG8 graphs,
//! designed to fit entirely within a 64KiB L1 TruthBlock.

use crate::runtime::cog8::{execute_cog8, Cog8Decision, Cog8Edge, Cog8Row};

/// L1 Reflex Arena (64KiB).
///
/// Designed to fit entirely within a L1 TruthBlock (L1 cache) to ensure
/// maximum throughput for closure evaluation.
///
/// Contains 512 COG8 nodes and 1024 POWL edges.
#[repr(C, align(64))]
pub struct L1ReflexArena {
    /// Closure nodes.
    pub nodes: [Cog8Row; 512],
    /// Topology edges.
    pub edges: [Cog8Edge; 1024],
}

impl Default for L1ReflexArena {
    fn default() -> Self {
        Self::new()
    }
}

impl L1ReflexArena {
    /// Create a new empty arena.
    pub fn new() -> Self {
        Self {
            nodes: [Cog8Row::default(); 512],
            edges: [Cog8Edge::default(); 1024],
        }
    }

    /// Execute the graph with zero heap allocations.
    ///
    /// This is the hot path for reflexive cognitive response.
    #[inline(always)]
    pub fn execute(
        &self,
        context: &crate::runtime::ClosedFieldContext,
        completed: u64,
    ) -> Cog8Decision {
        execute_cog8(&self.nodes, &self.edges, context, completed).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multimodal::{ContextBundle, PostureBundle};
    use crate::packs::TierMasks;
    use crate::runtime::cog8::*;
    use crate::runtime::ClosedFieldContext;
    use std::sync::Arc;
    use std::time::Instant;

    fn empty_context(snap: Arc<crate::compiled::CompiledFieldSnapshot>) -> ClosedFieldContext {
        ClosedFieldContext {
            snapshot: snap,
            posture: PostureBundle::default(),
            context: ContextBundle::default(),
            tiers: TierMasks::ZERO,
            human_burden: 0,
        }
    }

    #[test]
    fn test_arena_size_constraints() {
        let size = std::mem::size_of::<L1ReflexArena>();
        println!("L1ReflexArena size: {} octets", size);
        assert!(size <= 65536, "Arena exceeds 64KiB TruthBlock limit");
    }

    #[test]
    fn bench_pure_closure_reflex() {
        let mut arena = Box::new(L1ReflexArena::new());
        let field = crate::field::FieldContext::new("bench");
        let snap = Arc::new(crate::compiled::CompiledFieldSnapshot::from_field(&field).unwrap());
        let context = empty_context(snap);

        // Setup a single active node that matches everything
        arena.nodes[0] = Cog8Row {
            priority: 100,
            response: Instinct::Settle,
            ..Default::default()
        };

        // Setup one edge to it
        arena.edges[0] = Cog8Edge {
            to: NodeId(0),
            instr: Powl8Instr {
                guard_mask: 0,
                effect_mask: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let iterations = 200_000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = arena.execute(&context, 0);
        }

        let duration = start.elapsed();
        let rate = iterations as f64 / duration.as_secs_f64();

        println!("Pure closure reflex rate: {:.2} closures/sec", rate);
    }

    #[test]
    fn test_duplicate_storm() {
        let mut arena = Box::new(L1ReflexArena::new());
        let field = crate::field::FieldContext::new("bench");
        let snap = Arc::new(crate::compiled::CompiledFieldSnapshot::from_field(&field).unwrap());
        let context = empty_context(snap);

        // Fill arena with identical nodes and edges
        for i in 0..512 {
            arena.nodes[i] = Cog8Row {
                priority: i as u16,
                response: Instinct::Settle,
                ..Default::default()
            };
        }

        for i in 0..1024 {
            arena.edges[i] = Cog8Edge {
                to: NodeId((i % 512) as u16),
                instr: Powl8Instr {
                    guard_mask: 0,
                    effect_mask: 1 << (i % 64),
                    ..Default::default()
                },
                ..Default::default()
            };
        }

        let start = Instant::now();
        let decision = arena.execute(&context, 0);
        let duration = start.elapsed();

        println!("Duplicate storm execution took: {:?}", duration);
        assert_eq!(decision.response, Instinct::Settle);
    }

    #[test]
    fn test_unsafe_command_storm() {
        let mut arena = Box::new(L1ReflexArena::new());
        let field = crate::field::FieldContext::new("bench");
        let snap = Arc::new(crate::compiled::CompiledFieldSnapshot::from_field(&field).unwrap());
        let context = empty_context(snap);

        for i in 0..512 {
            arena.nodes[i] = Cog8Row {
                required_mask: 0xFFFFFFFFFFFFFFFF, // Hard to match
                response: Instinct::Refuse,
                ..Default::default()
            };
        }

        for i in 0..1024 {
            arena.edges[i] = Cog8Edge {
                to: NodeId((i % 512) as u16),
                kind: EdgeKind::Blocking,
                instr: Powl8Instr {
                    op: Powl8Op::Block,
                    guard_mask: 0,
                    effect_mask: 0xFFFFFFFFFFFFFFFF,
                    ..Default::default()
                },
                ..Default::default()
            };
        }

        let start = Instant::now();
        let _ = arena.execute(&context, 0);
        let duration = start.elapsed();

        println!("Unsafe-command storm execution took: {:?}", duration);
    }
}
