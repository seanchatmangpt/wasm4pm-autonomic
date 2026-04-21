# MDL Refinement

## Objective
Upgrade structural scoring in src/models/petri_net.rs to follow Φ(N) exactly.

## Requirements
- Every state motion must emit a UDelta and update the UReceipt.
- Conform to the 200ns T1 admissibility threshold (where applicable).
- Adhere to the UniverseOS Dual-Plane L1 Architecture.
- Zero heap allocations in the hot path.
- Branchless execution logic (CC=1).

## Context
See `src/agentic/ralph/patterns/U64_ARCHITECTURE.md` for the C4 diagrams and substrate laws.
