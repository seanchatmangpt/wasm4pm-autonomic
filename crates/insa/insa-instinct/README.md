# `insa-instinct`

INST8, KAPPA8, and core instinctual resolution logic for INSA.

This crate defines the primary 8-bit vectors that determine semantic behavior and execution mapping without dynamic allocation.

## Core Vectors
* **`InstinctByte`**: The 8-bit truth vector determining the immediate structural outcome.
  * `SETTLE`, `RETRIEVE`, `INSPECT`, `ASK`, `AWAIT`, `REFUSE`, `ESCALATE`, `IGNORE`.
* **`KappaByte`**: The 8-bit attribution vector mapping to specific Classical AI semantic engines.
  * `REFLECT`, `PRECONDITION`, `GROUND`, `PROVE`, `RULE`, `RECONSTRUCT`, `FUSE`, `REDUCE_GAP`.
* **Resolution**: Fast LUT-based conflict resolution mapping concurrent instinct triggers into a singular, bounded outcome.
