//! Cold Evidence Arena (Phase 13).
//!
//! Provides offline storage and analysis for `EvidenceLedger` traces and `POWL64` receipts.
//! Designed to handle high-volume 'receipt floods' and 'replay floods' for
//! tournament-style evaluation without impacting hot-path execution.

use crate::powl64::Powl64;
use crate::receipt::Receipt;
use crate::runtime::conformance::{check_conformance, ConformanceReport, EvidenceLedger};
use crate::runtime::CompiledCcogConfig;
use std::time::{Duration, Instant};

/// Offline storage and analysis arena for cognitive evidence.
#[derive(Debug, Default, Clone)]
pub struct ColdEvidenceArena {
    /// The primary collection of POWL64 traces.
    pub ledger: EvidenceLedger,
    /// Cryptographic receipts associated with the traces.
    pub receipts: Vec<Receipt>,
}

impl ColdEvidenceArena {
    /// Create a new empty cold evidence arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest a POWL64 trace and its associated receipt into the arena.
    pub fn ingest(&mut self, trace: Powl64, receipt: Receipt) {
        self.ledger.record(trace);
        self.receipts.push(receipt);
    }

    /// Perform a 'receipt flood' benchmark to measure ingestion throughput.
    ///
    /// This measures how many proofs per second the arena can ingest.
    pub fn benchmark_receipt_flood(&mut self, traces: Vec<(Powl64, Receipt)>) -> (Duration, f64) {
        let count = traces.len();
        let start = Instant::now();
        for (trace, receipt) in traces {
            self.ingest(trace, receipt);
        }
        let elapsed = start.elapsed();
        let throughput = count as f64 / elapsed.as_secs_f64();
        (elapsed, throughput)
    }

    /// Perform a 'replay flood' benchmark to measure reproducibility and verification speed.
    ///
    /// This measures how fast the arena can verify the entire ledger against a configuration.
    pub fn benchmark_replay_flood<const N: usize, const E: usize>(
        &self,
        config: &CompiledCcogConfig<N, E>,
    ) -> (Duration, f64, ConformanceReport) {
        let count = self.ledger.traces.len();
        let start = Instant::now();
        let report = check_conformance(config, &self.ledger);
        let elapsed = start.elapsed();
        let throughput = count as f64 / elapsed.as_secs_f64();
        (elapsed, throughput, report)
    }

    /// Identify and "reject" routes that do not meet the conformance threshold.
    ///
    /// Returns a list of indices in the ledger that are non-conformant.
    pub fn reject_routes<const N: usize, const E: usize>(
        &self,
        config: &CompiledCcogConfig<N, E>,
        fitness_threshold: f64,
    ) -> Vec<usize> {
        let mut rejected = Vec::new();
        for (i, trace) in self.ledger.traces.iter().enumerate() {
            // We use a temporary ledger for individual trace conformance
            let mut temp_ledger = EvidenceLedger::new();
            temp_ledger.record(trace.clone());
            let report = check_conformance(config, &temp_ledger);
            if report.fitness < fitness_threshold {
                rejected.push(i);
            }
        }
        rejected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphIri;
    use crate::ids::CollapseFn;
    use crate::ids::{EdgeId, FieldId, GroupId, NodeId, PackId, RuleId};
    use crate::powl64::Powl64RouteCell;
    use crate::receipt::Receipt;
    use crate::runtime::cog8::*;
    use chrono::Utc;

    fn mock_config() -> CompiledCcogConfig<2, 1> {
        let node0 = Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(1),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::None,
            var_ids: [FieldId::NONE; 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 0,
            response: Instinct::Ignore,
            priority: 0,
        };
        let node1 = Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(2),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::None,
            var_ids: [FieldId::NONE; 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 1,
            response: Instinct::Settle,
            priority: 0,
        };

        let edge = Cog8Edge {
            from: NodeId(0),
            to: NodeId(1),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Choice,
                collapse_fn: CollapseFn::None,
                node_id: NodeId(1),
                edge_id: EdgeId(1),
                guard_mask: 0,
                effect_mask: 1,
            },
        };

        CompiledCcogConfig {
            pack: LoadedFieldPack {
                id: PackId(1),
                name: "test",
                ontology_profile: vec![],
                digest_urn: "urn:test".to_string(),
            },
            nodes: [node0, node1],
            edges: [edge],
            mcp_projections: crate::runtime::mcp::MCPProjectionTable::new(),
        }
    }

    #[test]
    fn test_cold_arena_ingestion() {
        let mut arena = ColdEvidenceArena::new();
        let mut trace = Powl64::new();
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(1),
            ..Default::default()
        });
        let receipt = Receipt::new(
            GraphIri::from_iri("http://example.org/activity").unwrap(),
            "0".repeat(64),
            Utc::now(),
        );

        arena.ingest(trace, receipt);
        assert_eq!(arena.ledger.traces.len(), 1);
        assert_eq!(arena.receipts.len(), 1);
    }

    #[test]
    fn test_receipt_flood_benchmark() {
        let mut arena = ColdEvidenceArena::new();
        let mut traces = Vec::new();
        for i in 0..100 {
            let mut trace = Powl64::new();
            trace.extend(Powl64RouteCell {
                edge_id: EdgeId(i as u16),
                ..Default::default()
            });
            let receipt = Receipt::new(
                GraphIri::from_iri("http://example.org/activity").unwrap(),
                format!("{:064x}", i),
                Utc::now(),
            );
            traces.push((trace, receipt));
        }

        let (elapsed, throughput) = arena.benchmark_receipt_flood(traces);
        assert!(elapsed.as_nanos() > 0);
        assert!(throughput > 0.0);
        assert_eq!(arena.ledger.traces.len(), 100);
    }

    #[test]
    fn test_replay_flood_benchmark() {
        let mut arena = ColdEvidenceArena::new();
        let config = mock_config();

        for i in 0..50 {
            let mut trace = Powl64::new();
            trace.extend(Powl64RouteCell {
                edge_id: EdgeId(1),
                from_node: NodeId(0),
                to_node: NodeId(1),
                ..Default::default()
            });
            let receipt = Receipt::new(
                GraphIri::from_iri("http://example.org/activity").unwrap(),
                format!("{:064x}", i),
                Utc::now(),
            );
            arena.ingest(trace, receipt);
        }

        let (elapsed, throughput, report) = arena.benchmark_replay_flood(&config);
        assert!(elapsed.as_nanos() > 0);
        assert!(throughput > 0.0);
        assert_eq!(report.fitness, 1.0);
    }

    #[test]
    fn test_reject_routes() {
        let mut arena = ColdEvidenceArena::new();
        let config = mock_config();

        // Valid trace
        let mut t1 = Powl64::new();
        t1.extend(Powl64RouteCell {
            edge_id: EdgeId(1),
            from_node: NodeId(0),
            to_node: NodeId(1),
            ..Default::default()
        });

        // Invalid trace
        let mut t2 = Powl64::new();
        t2.extend(Powl64RouteCell {
            edge_id: EdgeId(999),
            from_node: NodeId(0),
            to_node: NodeId(1),
            ..Default::default()
        });

        arena.ingest(
            t1,
            Receipt::new(
                GraphIri::from_iri("urn:t:1").unwrap(),
                "0".repeat(64),
                Utc::now(),
            ),
        );
        arena.ingest(
            t2,
            Receipt::new(
                GraphIri::from_iri("urn:t:2").unwrap(),
                "1".repeat(64),
                Utc::now(),
            ),
        );

        let rejected = arena.reject_routes(&config, 1.0);
        assert_eq!(rejected, vec![1]);
    }
}
