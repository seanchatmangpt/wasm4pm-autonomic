# ccog: Cognitive Core Substrate

The `ccog` (Cognitive Core) crate provides the fundamental execution engine and deterministic policy dispatch for the Unibit architecture. It defines the stable lattice of autonomic instincts and the formal verification surface for runtime behavior.

## Core Responsibilities

- **Policy Dispatch**: Executes stable policy decisions via `select_instinct_v0`.
- **Performance Honesty**: Enforces a strict heap-free guarantee for the dispatch hot path.
- **Provenance & Transparency**: Manages the generation of `BarkDecision` packets and provenance-bearing trace receipts.
- **Determinism**: Provides canonical RDF graph operations (`FieldContext`, `CompiledFieldSnapshot`) used throughout the Unibit stack.

## Architecture

- **`instinct.rs`**: Implements the canonical response lattice (Ask, Settle, Inspect, Ignore, Retrieve, Refuse, Escalate).
- **`bark_artifact.rs`**: Core decision logic. Every decision is traceable, deterministic, and physically alloc-free.
- **`compiled.rs`**: Manages snapshots of field state for $O(1)$ dispatch lookups.

## Anti-Fake Gauntlet Compliance

`ccog` is audited via the Kill Zone 6 integrity gate. Every runtime dispatch path is subject to heap-allocation monitoring. Any logic that introduces non-deterministic heap growth is rejected.

## Documentation

- [Anti-Fake Architecture](./docs/anti_fake_architecture.md): Detailed explanation of dispatch honesty and manifest integrity.
