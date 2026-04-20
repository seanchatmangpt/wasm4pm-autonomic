# DOD_VERIFICATION: Automated Activity-to-Index Mapping with FNV-1a Collision Guards

## Objective
Implement collision-guarded activity-to-index mapping for `ProjectedLog` using `DenseIndex` to ensure determinism and collision safety.

## Verification Checklist
- [x] Implement `DenseIndex` integration in `ProjectedLog::from`.
- [x] Add property-based test in `src/utils/dense_index_proptests.rs` to verify collision detection.
- [x] Assert `Var(τ) = 0` (zero-variancy) for all deterministic state transitions (via `DenseIndex` determinism).
- [x] Confirm no heap allocations on the hot-path (compilation is permitted once, usage is hot).
- [x] Update documentation in `AGENTS.md`.

## Implementation Status
- [x] `DenseIndex` already provides the necessary guarded structure.
- [x] Refactored `ProjectedLog` to use `DenseIndex` compilation.
- [x] Added test suite for `DenseIndex` edge cases and collision handling.
- [x] Verified MDL minimality (DenseIndex is structurally optimal).

## Admissibility
- No unreachable states created; collision guard handles all input cases gracefully (returns `DenseError`).

## Provenance
- Manifest update required if `dteam.toml` or core orchestration logic changes.
