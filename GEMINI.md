# dteam — Digital Team Process Intelligence Engine

## Project Overview

**dteam** (Digital Team) is a high-performance, autonomous process discovery engine implemented in Rust. It specializes in nanosecond-scale process intelligence for WASM-compatible environments by utilizing formal bitset algebra and deterministic reinforcement learning. This project is licensed under the **Business Source License 1.1 (BUSL-1.1)** to prevent unauthorized commercialization.

### Core Architecture
- **K-Tier Memory Model**: Bounded, word-aligned bitset representation ($K \in \{64, 128, 256, 512, 1024\}$) for markings and transition masks.
- **Branchless Execution Kernel**: Eliminates data-dependent branching in transition firing using bitwise mask calculus ($M' = (M \ \& \ \neg I) \ | \ O$).
- **Zero-Heap Optimization**: Reinforcement learning states (`RlState`) are stack-allocated `Copy` structs (136 bits) to eliminate heap churn in hot paths.
- **Packed Key Table (PKT)**: Deterministic, cache-friendly alternative to `FxHashMap` using binary search over sorted 64-bit FNV-1a hashes. This delivered an **80% speedup** in RL updates.
- **Autonomic Discovery**: Self-optimizing discovery loops that target 100% accuracy on the PDC-2025 suite via closed-loop RL feedback.

## Licensing

This project is licensed under the **Business Source License 1.1**.
- **Licensor**: dteam
- **Change Date**: April 18, 2029
- **Change License**: Apache License, Version 2.0
- **Restriction**: You may not use this software to start a company or provide a competing service. See `LICENSE` for full terms.

## Building and Running

### Prerequisites
- Rust 1.75+ (Stable)
- `cargo`

### Key Commands

| Task | Command |
|---|---|
| **Build Library** | `cargo build` |
| **Run Unit Tests** | `cargo test` |
| **Run All Benches** | `cargo bench` |
| **Check Constraints** | `cargo check` |
| **Project Doctor** | `make doctor` |
| **Lint & Format** | `make lint && make fmt` |

## Development Conventions

- **Versioning**: Follows the `dteam` CalVer variant: `vYEAR.MONTH.DAY`.
- **Zero-Allocation Policy**: The hot path (replay, RL updates) MUST NOT perform heap allocations. Use `PackedKeyTable` and `KBitSet` instead of `HashMap` and `Vec`.
- **Branchless Logic**: Prefer bitwise mask selection over `if/else` for data-dependent execution to maintain instruction-level stability.
- **K-Tier Alignment**: All bitset operations must align with `KTier` word boundaries (64-bit multiples).
- **Autonomic Lifecycle**: Follow the 7-step loop (`observe` -> `infer` -> `propose` -> `accept` -> `execute` -> `manifest` -> `adapt`) defined in `src/autonomic/`.
- **Serialization**: Use `to_js_str()` in `src/utils/mod.rs` for WASM compatibility. `PackedKeyTable` supports serialization for persisting agent Q-tables.

## Key Modules & Files

- `src/autonomic/`: The core Digital Team lifecycle and action kernel.
- `src/dteam/`: Engine orchestration, K-tier definitions, and branchless kernel definitions.
- `src/reinforcement/`: Model-free RL agents (Q-Learning, SARSA, etc.) optimized for zero-heap execution.
- `src/utils/dense_kernel.rs`: The "Dense Kernel" primitives (`PackedKeyTable`, `DenseIndex`, `KBitSet`).
- `src/models/petri_net.rs`: Petri net structure and structural validity scoring.
- `src/conformance/`: High-performance token-based replay implementations.
- `benches/`: Comprehensive performance verification suite (9+ specialized benches).
- `examples/doctor.rs`: System diagnostic utility.

## Common Gotchas for AI Agents

1. **Hashing**: Use `crate::utils::dense_kernel::fnv1a_64` for high-speed string and collection hashing in the hot path.
2. **State Key Encoding**: RL state keys for serialization are encoded as `i64` in `rl_state_serialization`.
3. **Capacity Limits**: The engine will trigger `EngineResult::PartitionRequired` if the log's activity footprint exceeds the configured `KTier` capacity.
4. **Petri Arcs**: Arcs must follow the bipartite property (Place $\leftrightarrow$ Transition) enforced by `ArcPolicy::BipartitePetri`.
