//! L2 Working-Skill Arena (PRD v0.8).
//!
//! Provides a 2MiB TruthBlock for loading full working skill packs and
//! executing them with task-level throughput. Includes stress tests for
//! ecology metrics like human burden and externalization rates.

use crate::ids::HumanRoleId;
use crate::runtime::cog8::{execute_cog8, Cog8Decision, Cog8Edge, Cog8Row, Instinct};
use crate::runtime::hitl::HumanRoleProfile;
use crate::runtime::mcp::MCPProjectionTable;

/// L2 Working-Skill Arena (reduced size for demo).
///
/// Contains 1024 COG8 nodes and 256 POWL edges.
#[repr(C, align(64))]
pub struct L2WorkingSkillArena {
    /// Closure nodes (1k COG8).
    pub nodes: [Cog8Row; 1024],
    /// Topology edges.
    pub edges: [Cog8Edge; 256],
    /// Projection table for MCP integration.
    pub mcp_projections: MCPProjectionTable,
}

impl Default for L2WorkingSkillArena {
    fn default() -> Self {
        Self::new()
    }
}

impl L2WorkingSkillArena {
    /// Create a new empty L2 arena.
    pub fn new() -> Self {
        Self {
            nodes: [Cog8Row::default(); 1024],
            edges: [Cog8Edge::default(); 256],
            mcp_projections: MCPProjectionTable::new(),
        }
    }
    // ... (load_packs unchanged)
    /// Load skill packs into the arena.
    pub fn load_packs(&mut self) {
        let mut node_offset = 0;
        let mut edge_offset = 0;

        // Load Dev Pack
        for node in crate::packs::dev::COG8_NODES {
            if node_offset < self.nodes.len() {
                self.nodes[node_offset] = *node;
                node_offset += 1;
            }
        }
        for edge in crate::packs::dev::COG8_EDGES {
            if edge_offset < self.edges.len() {
                self.edges[edge_offset] = *edge;
                edge_offset += 1;
            }
        }

        // Load Edge Pack
        for node in crate::packs::edge::COG8_NODES {
            if node_offset < self.nodes.len() {
                self.nodes[node_offset] = *node;
                node_offset += 1;
            }
        }
        for edge in crate::packs::edge::COG8_EDGES {
            if edge_offset < self.edges.len() {
                self.edges[edge_offset] = *edge;
                edge_offset += 1;
            }
        }

        // Load Enterprise Pack
        for node in crate::packs::enterprise::COG8_NODES {
            if node_offset < self.nodes.len() {
                self.nodes[node_offset] = *node;
                node_offset += 1;
            }
        }
        for edge in crate::packs::enterprise::COG8_EDGES {
            if edge_offset < self.edges.len() {
                self.edges[edge_offset] = *edge;
                edge_offset += 1;
            }
        }

        // Load Lifestyle Pack
        for node in crate::packs::lifestyle::COG8_NODES {
            if node_offset < self.nodes.len() {
                self.nodes[node_offset] = *node;
                node_offset += 1;
            }
        }
        for edge in crate::packs::lifestyle::COG8_EDGES {
            if edge_offset < self.edges.len() {
                self.edges[edge_offset] = *edge;
                edge_offset += 1;
            }
        }
    }

    /// Execute the graph in the arena.
    #[inline(always)]
    pub fn execute(
        &self,
        context: &crate::runtime::ClosedFieldContext,
        completed: u64,
    ) -> Cog8Decision {
        execute_cog8(&self.nodes, &self.edges, context, completed).unwrap_or_default()
    }

    /// Benchmark task-level throughput.
    /// Returns decisions/sec.
    pub fn run_benchmark(&self, iterations: usize) -> f64 {
        use std::time::Instant;
        let field = crate::field::FieldContext::new("bench");
        let snap = std::sync::Arc::new(
            crate::compiled::CompiledFieldSnapshot::from_field(&field).unwrap(),
        );
        let context = crate::runtime::ClosedFieldContext {
            snapshot: snap,
            posture: crate::multimodal::PostureBundle::default(),
            context: crate::multimodal::ContextBundle::default(),
            tiers: crate::packs::TierMasks::ZERO,
            human_burden: 0,
        };
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = self.execute(&context, 0);
        }
        let elapsed = start.elapsed();
        iterations as f64 / elapsed.as_secs_f64()
    }

    /// Measure projections/sec.
    pub fn measure_projections_sec(&self, iterations: usize) -> f64 {
        use std::time::Instant;
        // Mock context for projection
        let field = crate::field::FieldContext::new("bench");
        let snap = std::sync::Arc::new(
            crate::compiled::CompiledFieldSnapshot::from_field(&field).unwrap(),
        );
        let context = crate::runtime::ClosedFieldContext {
            snapshot: snap,
            posture: crate::multimodal::PostureBundle::default(),
            context: crate::multimodal::ContextBundle::default(),
            tiers: crate::packs::TierMasks::ZERO,
            human_burden: 0,
        };

        let start = Instant::now();
        let mut projections = 0;
        for i in 0..iterations {
            let decision = self.execute(&context, i as u64);
            if self.mcp_projections.project(&decision, &context).is_some() {
                projections += 1;
            }
        }
        let elapsed = start.elapsed();
        projections as f64 / elapsed.as_secs_f64()
    }
}

/// Stress tester for L2 working-skill logic.
pub struct L2StressTester<'a> {
    /// Reference to the arena under test.
    pub arena: &'a L2WorkingSkillArena,
}

impl<'a> L2StressTester<'a> {
    /// 'missing-evidence storm': Simulates a flood of inputs that fail matching
    /// or trigger Instinct::Ask, driving high human burden.
    pub fn storm_missing_evidence(
        &self,
        context: &crate::runtime::ClosedFieldContext,
        iterations: usize,
    ) -> f64 {
        let mut ask_count = 0;
        for i in 0..iterations {
            // Use bit patterns likely to trigger 'Ask' in the loaded packs
            let decision = self.arena.execute(context, i as u64);
            if decision.response == Instinct::Ask {
                ask_count += 1;
            }
        }
        ask_count as f64 / iterations as f64
    }

    /// 'human-overload storm': Simulates HITL burden protection by flooding
    /// a human profile and measuring the rejection rate.
    pub fn storm_human_overload(&self, iterations: usize) -> (u64, f64) {
        let mut profile = HumanRoleProfile::new(HumanRoleId(1), 10, 0.9);
        let mut rejections = 0;
        let burden_limit = 1000;

        for _ in 0..iterations {
            if profile.current_burden > burden_limit {
                rejections += 1;
            } else {
                profile.add_burden(50);
            }
            // Apply slight decay
            profile.decay_burden(1, 1);
        }

        (
            profile.current_burden,
            rejections as f64 / iterations as f64,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_arena_size() {
        let size = std::mem::size_of::<L2WorkingSkillArena>();
        println!("L2WorkingSkillArena size: {} octets", size);
        // 2MiB = 2,097,152 octets
        assert!(
            size <= 2097152 + 64,
            "Arena exceeds 2MiB limit significantly"
        );
    }

    #[test]
    fn test_load_and_bench() {
        // Use a boxed arena to avoid stack overflow in tests
        let mut arena = L2WorkingSkillArena::new();
        arena.load_packs();

        let throughput = arena.run_benchmark(100_000);
        println!("L2 throughput: {:.2} decisions/sec", throughput);
        assert!(throughput > 0.0);
    }

    #[test]
    fn test_stress_storms() {
        let mut arena = L2WorkingSkillArena::new();
        arena.load_packs();

        let field = crate::field::FieldContext::new("bench");
        let snap = std::sync::Arc::new(
            crate::compiled::CompiledFieldSnapshot::from_field(&field).unwrap(),
        );
        let context = crate::runtime::ClosedFieldContext {
            snapshot: snap,
            posture: crate::multimodal::PostureBundle::default(),
            context: crate::multimodal::ContextBundle::default(),
            tiers: crate::packs::TierMasks::ZERO,
            human_burden: 0,
        };

        let tester = L2StressTester { arena: &arena };

        let ask_rate = tester.storm_missing_evidence(&context, 10_000);
        println!(
            "Missing-evidence storm 'Ask' rate: {:.2}%",
            ask_rate * 100.0
        );

        let (final_burden, rejection_rate) = tester.storm_human_overload(10_000);
        println!(
            "Human-overload storm: final burden {}, rejection rate {:.2}%",
            final_burden,
            rejection_rate * 100.0
        );

        assert!(final_burden > 0);
    }
}
