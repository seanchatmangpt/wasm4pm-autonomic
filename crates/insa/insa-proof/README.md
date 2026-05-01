# `insa-proof`

**Cold-path evidence and replay layer for INSA.**

This crate provides the structures and serialization boundaries necessary to cryptographically verify, trace, and replay a cognitive execution path. While `insa-hotpath` defines how to make a fast decision, `insa-proof` guarantees that the decision leaves an immutable, cryptographically secure receipt (`A = µ(O*)`).

## `powl64` Route Proofs
The `powl64` module provides high-capacity trace layouts.
* **`RouteCell64`**: A 64-byte `repr(C, align(64))` structure designed to fit perfectly into a single cache line. It records every step of an executed route without requiring dynamic memory formatting.
  * It captures the `NodeId` and `EdgeId`.
  * It records the before/after state via `pre_mask` and `post_mask` (`CompletedMask`).
  * It embeds the exact triggering `InstinctByte` and the collapse attribution (`KappaByte`).

## Evidence & Verification
* **`receipt`**: Cryptographic admission receipts confirming execution logic against policy boundaries.
* **`wire`**: Cross-platform binary wire encodings ensuring the exact memory layout of an executed proof can be shipped out of the system without complex serialization overhead.
