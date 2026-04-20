# DOD_VERIFICATION: Zero-Heap Primitives and Collision-Guarded Indices

## Objective
Implement zero-allocation data structures and collision-guarded activity-to-index mapping to ensure nanosecond-scale process intelligence with strict determinism.

## Verification Checklist (DenseIndex)
- [x] Implement `DenseIndex` integration in `ProjectedLog::from`.
- [x] Add property-based test in `src/utils/dense_index_proptests.rs` to verify collision detection.
- [x] Assert `Var(τ) = 0` (zero-variancy) for all deterministic state transitions (via `DenseIndex` determinism).
- [x] Confirm no heap allocations on the hot-path (compilation is permitted once, usage is hot).
- [x] Update documentation in `AGENTS.md`.

## Verification Checklist (StaticPackedKeyTable)
- [x] Implement `StaticPackedKeyTable` in `src/utils/static_pkt.rs`.
- [x] Verify via `proptest` in `src/utils/static_pkt_tests.rs`.
- [x] Confirm no runtime allocations in `StaticPackedKeyTable` (stack-allocated).
- [x] Ensure zero-heap hot path achieved for static capacities.
- [x] Ready for agent-specific integration in hot paths.

## Implementation Status
- [x] `DenseIndex` provides guarded structure for activity indexing.
- [x] `StaticPackedKeyTable` provides stack-allocated, deterministic key table for hot paths.
- [x] Both verified via property-based tests.
- [x] Verified MDL minimality for both structures.

## Provenance
- Manifest update required if `dteam.toml` or core orchestration logic changes.
