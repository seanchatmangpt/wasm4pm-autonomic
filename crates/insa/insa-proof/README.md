# `insa-proof`

Cold-path evidence and replay layer for INSA.

This crate provides the structures and serialization boundaries necessary to cryptographically verify, trace, and replay a cognitive execution path. 

## Features
* **`powl64`**: Contains high-capacity trace layout vectors. Core to this is `RouteCell64`, a strictly aligned 64-byte structure capturing an ordinal step, node edge routing, before/after `CompletedMask`, and the triggering `InstinctByte`/`KappaByte` attribution.
* **`receipt`**: Cryptographic admission receipts verifying that an action `A` was lawfully derived from `O*`.
* **`wire`**: Cross-platform binary wire encodings.
