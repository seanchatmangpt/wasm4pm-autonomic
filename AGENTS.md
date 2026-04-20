# AGENTS.md — guidance for automated agents and contributors

This file summarizes how the **dteam** (Deterministic Process Intelligence Engine / Digital Team) Rust workspace is organized, how to validate changes, and what constraints to respect. It is the primary onboarding artifact for coding agents. When this document conflicts with source or `Cargo.toml`, prefer the repository and update this file in the same change.

## 1. Project identity

- **Package**: `dteam` version **1.3.0**, Rust **2021** (`Cargo.toml`). Single crate workspace (`[workspace]` is present but empty).
- **Purpose**: Deterministic process-intelligence kernel: event logs and Petri nets, token-based conformance replay, tabular reinforcement-learning agents for discovery loops, autonomic “digital team” kernels, SIMD/SWAR helpers, POWL and OCEL-oriented structures, and WASM-friendly serialization hooks.
- **License**: **Business Source License 1.1** (`LICENSE`). Commercial and competitive-use restrictions may apply; read the full text before assuming OSS freedoms.
- **Third-party models**: `Event` / `Trace` / `EventLog` attribution in **`ATTRIBUTION.md`** (rust4pm lineage, MIT/Apache-2.0).

## 2. Toolchain and dev profile

- **Runtime deps** (high level): `bcinr`, `rustc-hash`, `serde`/`serde_json`/`toml`, `quick-xml`, `anyhow`, `chrono`, `uuid`, `hashbrown`, `wasm-bindgen`, `serde-wasm-bindgen`, `fastrand`, `itertools`.
- **Dev-dependencies**: `criterion`, `divan`, `iai-callgrind`, `dhat`, `process_mining` (reference/bench comparisons where used).
- **Features** (see §4): default includes **`token-based-replay`**.
- **MSRV**: Not pinned in `Cargo.toml`; use current **stable** Rust compatible with edition 2021 and the dependency versions in the lockfile (`Cargo.lock`).

## 3. Commands (daily use)

| Goal | Command |
|------|---------|
| Fast compile | `cargo check` or `make check` |
| Library unit tests | `cargo test --lib` or `make test` (uses `--nocapture` via Makefile) |
| Release build | `cargo build --release` or `make build` |
| Lint (deny warnings) | `make lint` → `cargo clippy --all-targets -- -D warnings` |
| Format | `make fmt` |
| All-target check (examples, benches) | `cargo check --all-targets` (also part of `make doctor`) |
| Benchmarks | `cargo bench` / `make bench` |
| Diagnostics example | `cargo run --example doctor` |
| Autonomic simulation | `cargo run --example autonomic_runner` |

**`make doc`** runs `pdflatex` twice on [`docs/thesis/main.tex`](docs/thesis/main.tex) and renames the output to **`docs/thesis/dteam-whitepaper.pdf`**.

## 4. Cargo features

- **`token-based-replay`** (default): gates `src/conformance/case_centric/token_based_replay.rs` behind `cfg(feature = "token-based-replay")`. Adversarial tests under `case_centric/` remain available in test builds.

## 5. Configuration (`dteam.toml` and `AutonomicConfig`)

- Path: workspace root **`dteam.toml`**. **`AutonomicConfig::load`** in `src/config.rs` returns **`Default`** if the file is missing (no error).
- Sections map roughly as follows:

| TOML block | Rust type | Notes |
|------------|-----------|--------|
| `[meta]` | `MetaConfig` | Version string, environment label, identity. |
| `[kernel]` | `KernelConfig` | `tier` (e.g. K256), `alignment`, `determinism`, `allocation_policy` (documented intent; not all keys may drive code paths yet). |
| `[autonomic]` | `AutonomicSystemConfig` | `mode`, `sampling_rate`, `integrity_hash`; nested **`guards`** (`risk_threshold`, `min_health_threshold`, `max_cycle_latency_ms`, `repair_authority`) and **`policy`** (`profile`, `mdl_penalty`, `human_weight`). |
| `[rl]` | `RlConfig` | `algorithm` name string, learning/discount/exploration rates, `reward_weights` map. |
| `[discovery]` | `DiscoveryConfig` | Epoch caps, fitness stop threshold, strategy, drift window. |
| `[paths]` | `PathConfig` | Training/test/ground-truth dirs under `data/`, `artifacts_dir`, manifest bus path. |
| `[wasm]` | `WasmConfig` | Batch size and max pages (host/WASM tuning). |

- **`dteam::dteam::orchestration::Engine::run`** loads `dteam.toml` for reward weight keys such as **`fitness`** and **`soundness`** when building training behavior.
- **`DefaultKernel`** and **`AutonomicKernel::run_cycle`** also load `dteam.toml` for guard thresholds and policy (e.g. `strict_conformance`, `min_health_threshold`).

---

## 6. Library surface (`src/lib.rs`)

Notable public items:

- **Process data**: `models` (`Event`, `Trace`, `EventLog`, `petri_net::PetriNet`), re-exported at crate root via `pub use models::*`.
- **Conformance**: `conformance` re-exported at crate root.
- **RL embedding**: **`RlState`**, **`RlAction`**, tied to **`reinforcement::WorkflowState` / `WorkflowAction`**, with a small **`rl_state_serialization`** module for keyed tables.
- **Autonomic**: re-exports **`AutonomicKernel`**, **`DefaultKernel`**, **`AutonomicState`**, **`AutonomicAction`**, **`AutonomicEvent`**, **`AutonomicResult`**, **`ActionType`**, **`ActionRisk`**.
- **Namespaces**: `dteam::dteam` holds **`core::KTier`**, **`orchestration::{Engine, EngineBuilder, ExecutionManifest, EngineResult}`**, thin **`kernel` / `artifacts` / `verification`** modules, and **`verification::run_skeptic_harness`**.
- **Feature modules**: `simd`, `probabilistic`, `powl`, `ml`, `agentic`, `ocpm`, `benchmark`, `config`, `skeptic_harness`, `skeptic_contract`, `ref_models`, `ref_conformance`.
- **Integration tests in library**: `jtbd_tests`, `jtbd_counterfactual_tests`, `reinforcement_tests` are normal modules with `#[cfg(test)]` suites (see §10).

---

## 7. Source layout (by area)

### Core process models — `src/models/`

- **`mod.rs`**: Attributes, `Event` / `Trace` / `EventLog`, **`activity_footprint`**, **`canonical_hash`** (FNV-1a over concept:name activity strings).
- **`petri_net.rs`**: `Place`, `Transition`, `Arc`, **`PetriNet`** with `PackedKeyTable` markings, structural workflow-net checks, soundness / MDL-related helpers used by automation and orchestration.

### Conformance — `src/conformance/`

- **`mod.rs`**: **`ProjectedLog`** (indexed activities, aggregated traces), **`token_replay_projected`**, **`token_replay`**: fast **`u64`** marking masks when place count **≤ 64**; otherwise **`replay_trace_standard`** path. **`ConformanceResult`**, **`TokenReplayDeviation`** types.
- **`case_centric/`**: Feature-gated **`token_based_replay`**, **`adversarial_tests`** (overflow, missing tokens).

### I/O — `src/io/`

- **`xes.rs`**: `XESReader` using `quick-xml` (`read`, `parse_str`). Adds a `source` attribute when reading from path.
- **`xes_tests.rs`**: Regression tests for import.

### Utilities — `src/utils/`

- **`dense_kernel.rs`**: **`fnv1a_64`**, **`PackedKeyTable`**, **`DenseIndex`** (collision-guarded mapping); primary hash spine for nets and logs.
- **`dense_index_proptests.rs`**: Property tests for **`DenseIndex`** collision detection.
- **`bitset.rs`**, **`perturbation.rs`**, **`simd/swar.rs`**: low-level and perturbation helpers.
- **`mod.rs`**: **`to_js_str`** bridges serde types to **`wasm_bindgen::JsValue`** via **`serde_wasm_bindgen`**.

### Reinforcement — `src/reinforcement/`

- Traits: **`WorkflowState`**, **`WorkflowAction`**, **`Agent`**, **`AgentMeta`**.
- Algorithms: **`QLearning`**, **`DoubleQLearning`**, **`SARSAAgent`**, **`ExpectedSARSAAgent`**, **`ReinforceAgent`**.
- Shared Q-table helpers use **`PackedKeyTable`** and **`rustc_hash::FxHasher`** for state keys (see `get_q_values`, `hash_state`).

### Automation — `src/automation.rs`

- **`train_with_provenance`** / **`train_with_provenance_projected`**: RL loop over **`ProjectedLog`**, **`token_replay_projected`** fitness, structural checks, emits **`PetriNet`** plus byte **`trajectory`** of action indices.
- **`automate_discovery`**: scans `data_dir` + config paths for `*.xes` matching `*00.xes` naming (legacy contest layout).

### Autonomic — `src/autonomic/`

- **`types.rs`**: States, actions, events, risk tiers.
- **`kernel.rs`**: **`AutonomicKernel`** trait (**observe → infer → propose → accept → execute → adapt**), **`DefaultKernel`** implementing policy/guard behavior from config.
- **`vision_2030_kernel.rs`**: **`Vision2030Kernel`** used heavily in JTBD tests; manifest strings include **`VISION_2030_MANIFEST`** invariants.
- **`macros.rs`**: supporting macros.

### Extended stacks

- **`src/powl/`**: Partially ordered workflow language structures (**`PowlModel`**, operators XOR/AND/LOOP/etc., choice graphs); soundness validation hooks on **`PowlNode`**.
- **`src/ocpm/`**: **`OcelLog`** — hashed IDs, flat vectors, **`add_event_hashed`** to avoid per-event `String` churn where possible.
- **`src/probabilistic/`**: e.g. count-min style sketch (`count_min.rs`).
- **`src/ml/`**: **`linucb`** and related exports.
- **`src/agentic/`**: **`counterfactual::Simulator`** for scenario-style tests.
- **`src/simd/`**: module wrapper around SWAR helpers.

### Verification / reference

- **`skeptic_harness.rs`**: Enumerates adversarial “attacks” and claim registry; includes tests tying narrative to code.
- **`skeptic_contract.rs`**: Non-implementation **contract** document: constants like **`CHECK_RESET_AXIOM`** encode obligations for trace-isolated evaluation (overfitting, value–structure gap, etc.).
- **`ref_models/`**, **`ref_conformance/`**: reference Petri net, event log, token replay for parity checks.

### Benchmarks entry — `src/benchmark.rs`

- **`run_contest_benchmark`**: calls **`automate_discovery("./data/pdc2025/")`** for pipeline timing (requires data on disk).

---

## 8. Conformance performance path

- **`PetriNet`** places indexed into **`u64`** bitmasks when **`places.len() ≤ 64`** for vectorized mask updates in **`token_replay`** / **`token_replay_projected`**.
- Larger place counts fall back to **`replay_trace_standard`** with **`PackedKeyTable`** markings (more general, slower).
- **`RlState.marking_mask`** in `lib.rs` is documented for BCINR-style Petri masks; keep bit width assumptions aligned with **`KTier`** / engine capacity when extending the hot path.

## 9. Examples

| Example | Role |
|---------|------|
| **`examples/doctor.rs`** | Prints **`Engine::doctor()`**, exercises **`DefaultKernel`** with a synthetic **`AutonomicEvent`**. |
| **`examples/autonomic_runner.rs`** | Multi-event simulation: infer → propose → accept → execute → **`adapt`** feedback in a loop. |

---

## 10. Test suites inside the library crate

- **`jtbd_tests` / `jtbd_counterfactual_tests`**: Scenario-driven **`Vision2030Kernel`** runs; assert health bounds, manifest prefixes (`VISION_2030_MANIFEST`), deterministic `hash=` substrings, drift/reward feedback, governance cases.
- **`reinforcement_tests`**: Convergence and serialization roundtrips for tabular agents.
- **`io/xes_tests`**, **`conformance/case_centric/adversarial_tests`**, **`automation`**, **`dteam::orchestration`**, **`autonomic::kernel`**, **`skeptic_harness`**: narrower unit tests.

Run everything: **`cargo test --lib`**.

---

## 11. Benchmark binaries (`benches/`, harness = false)

Registered in **`Cargo.toml`**: `reinforcement_bench`, `real_data_bench`, `algorithm_bench`, `dteam_bench`, `zero_allocation_bench`, `instruction_stability_bench`, `bcinr_primitives_bench`, `e2e_discovery_bench`, `ktier_scalability_bench`, `kernel_bench`, `comparison_bench`, `validation_bench`, `wasm_bridge_bench`, `adversarial_topology_bench`, `high_concurrency_update_bench`, `stochastic_noise_bench`, `vision_2030_bench`. Use **`cargo bench <filter>`** to shorten runs during development.

---

## 12. Data, artifacts, and thesis assets

- **Config-relative dirs**: `data/pdc2025/{training_logs,test_logs,ground_truth}` (see `dteam.toml`); **`artifacts/`**, **`tmp/dmanifest_bus`** for manifests.
- **`docs/thesis/`**: LaTeX sources; **`make doc`** produces **`docs/thesis/dteam-whitepaper.pdf`** (via two `pdflatex` passes on `main.tex`).

---

## 13. Security and determinism notes

- XES parsing uses **`quick-xml`** without a full DTD resolver in the snippet path; keep expansion limits and entity policies in mind when extending **`parse_content`**.
- Token replay and **`canonical_hash`** are written for **audit-style reproducibility**; preserve hashing and ordering semantics when changing event or net serialization.
- **`skeptic_contract`** explicitly calls out **no hidden state across traces** as a requirement class for fair evaluation—align RL evaluation loops with that when adding caches.

---

## 14. Conventions for agents

- **Scope**: Touch only files needed for the task; avoid drive-by refactors or unsolicited markdown except when updating this **`AGENTS.md`** alongside behavioral changes.
- **Lockfile**: When changing `Cargo.toml` dependencies, update **`Cargo.lock`** and commit both so CI and `cargo bench` stay reproducible.
- **Style**: Match module patterns (`anyhow::Result` in IO, `PackedKeyTable` + **`fnv1a_64`** for IDs, feature gates for optional conformance).
- **Git**: Per project policy, **do not** use destructive resets; fix forward; prefer **`git revert`** for rollback commits.
- **Performance**: Prefer existing **`ProjectedLog`** and bitmask paths before adding new allocations on hot paths.
- **Root artifacts**: LaTeX/PDF and roadmap files may live beside the crate; do not remove documentation PDFs or thesis outputs unless the task requires it.

---

## 15. Pre-merge verification checklist

1. `cargo check` (minimum).
2. `cargo clippy --all-targets -- -D warnings` when editing Rust sources intended for merge.
3. `cargo test --lib`.
4. If touching conformance or nets: run adversarial tests and a focused **`cargo bench`** group when performance claims change.
5. Update **`AGENTS.md`** if public layout, config schema, or primary commands change.
6. For `examples/` or `benches/`, run `cargo check --examples` / `cargo check --benches` if CI does not cover them yet.
