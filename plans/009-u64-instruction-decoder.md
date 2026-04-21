# U64-Instruction-Decoder

## Objective
Implement the UInstruction Decoder for zero-copy mapping of operator IDs to Scratch-resident masks.

## Requirements
- Every state motion must emit a UDelta and update the UReceipt.
- Conform to the 200ns T1 admissibility threshold (where applicable).
- Adhere to the UniverseOS Dual-Plane L1 Architecture.
- Zero heap allocations in the hot path.
- Branchless execution logic (CC=1).

## Context
See `src/agentic/ralph/patterns/U64_ARCHITECTURE.md` for the C4 diagrams and substrate laws.
