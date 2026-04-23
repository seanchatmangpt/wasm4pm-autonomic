# Edge, Fog, Cloud, Browser, and AtomVM Deployment

## Two orthogonal tier systems

dteam uses two independent tier classifications that must be respected simultaneously. Confusing them is the most common deployment design error.

**K-tier** sizes the Petri net: it determines how many places the bitmask kernel can handle without falling back to the BCINR path. K-tier is set in `dteam.toml` under `[kernel].tier` and is enforced by `EngineBuilder`, which selects the appropriate `KBitSet<WORDS>` monomorphization at construction time.

| K-Tier | `WORDS` | Max places | Approx memory | Epoch latency |
|--------|---------|------------|---------------|---------------|
| K64 | 1 | 64 | 16 KB | 2–5 µs |
| K128 | 2 | 128 | 16 KB | 2–5 µs |
| K256 | 4 | 256 | 16 KB | 2–5 µs |
| K512 | 8 | 512 | 64 KB | 14–20 µs |
| K1024 | 16 | 1024 | 128 KB | 30–50 µs |

**T-tier** sizes the signal: it determines the compute budget for each signal in the ML portfolio. T-tier is derived automatically from timing measurements using `Tier::from_timing_us`.

| T-Tier | Latency range | Semantic |
|--------|---------------|----------|
| T0 | < 100 µs | Branchless kernel candidate — safe for real-time loop |
| T1 | 100 µs – 2 ms | Folded signature or small projection — safe for batch |
| T2 | 2 ms – 100 ms | Wider vector or moderate cost — offline only |
| Warm | > 100 ms | Planning layer only — not on any hot path |

A deployment decision requires specifying both: "K256, T0/T1 signals only" is a complete description. "T0 signals" without a K-tier says nothing about whether the net fits in the bitmask kernel. "K64" without a T-tier says nothing about which signals can run within the memory budget.

## Which signals run where

The 15 signals in the PDC 2025 pool span all four T-tiers. The table below shows the deployment contexts where each signal is available:

| Signal | T-tier | Edge (MCU) | Fog (gateway) | Cloud | Browser (WASM) |
|--------|--------|------------|---------------|-------|----------------|
| H_inlang_fill | T0 | Yes | Yes | Yes | Yes |
| F_fitness | T0 | Yes | Yes | Yes | Yes |
| G_generalization | T0 | Yes | Yes | Yes | Yes |
| TF_IDF | T1 | No | Yes | Yes | Yes |
| NGram | T1 | No | Yes | Yes | Yes |
| E_edit_dist | T1 | No | Yes | Yes | Yes |
| PageRank | T2 | No | No | Yes | No |
| HDC_prototype | T2 | No | No | Yes | No |
| AutoML_hyper | T2 | No | No | Yes | No |
| RL_AutoML | Warm | No | No | Yes | No |
| Stacking ensemble | Warm | No | No | Yes | No |

The Edge column is the most restrictive: only T0 conformance signals fit within the 50 µs budget and the MCU's memory constraints. The Browser column excludes T2/Warm signals because the WASM single-threaded model prevents the parallel evaluation that makes T2 signals feasible in bounded time.

## Hardware mapping

**Edge (MCU, K64, T0 only, 16 KB):** Microcontrollers with kilobytes of RAM and no OS. The K64 bitmask fits in 16 KB. Only T0 conformance signals (F, G, H) run within the cycle budget. No `rayon`, no threads, no heap beyond boot-time initialization. This is the target for the Vision 2030 roadmap's "constant latency loops" requirement — WCET-bounded execution on embedded hardware.

**Fog (gateway, K256, T0/T1, 2–5 µs):** Edge gateway nodes with megabytes of RAM and a real-time OS or bare-metal Rust. K256 covers most industrial process nets. T0 and T1 signals are available — conformance signals plus TF-IDF, NGram, and edit distance. The 50 µs autonomic loop budget accommodates K256 epochs with headroom for signal evaluation.

**Cloud (full stack, all tiers):** Unrestricted compute. All K-tiers, all T-tiers, multi-threaded via `rayon`, heap allocation permitted for initialization. The PDC 2025 benchmark pipeline runs here — 15-log evaluation with OOF stacking, successive halving, and Pareto front computation. This is the only deployment context where Warm-tier signals (stacking ensembles, full AutoML search) are available.

**Browser (K64, WASM, T0/T1, 1 MB limit):** WebAssembly targets the K64 tier. The WASM linear memory model limits total allocation to the configured `max_pages` (default: 16 pages = 1 MB). WASM is single-threaded — `rayon`'s `par_iter` must be disabled or replaced with sequential iteration. T2/Warm signals are excluded because their sequential runtime exceeds acceptable browser interaction latency. `wasm-bindgen` provides the Rust-to-JS bridge; `serde-wasm-bindgen` handles `JsValue` serialization for the `AutomlPlan` JSON artifact.

**AtomVM (disorder membrane, not a Rust computation host):** AtomVM sits at the IO boundary as a fault-tolerant message-passing membrane — not as a Rust computation host. It inherits the BEAM (Erlang VM) actor model: per-process heaps, preemptive scheduling, copy-on-send message passing, per-process garbage collection. These are exactly the properties the Rust hot path refuses to have inside.

The role division is architecturally precise: AtomVM handles everything the outside world demands — variable message sizes, retries, timeouts, fault isolation, process supervision, "let it crash" semantics. The Rust engine handles everything the inside requires — branchless state transitions, deterministic cycle counts, zero allocation on the hot path, audit-reproducible fingerprints.

Canonical events cross the membrane as fixed-size packets. AtomVM deserializes the external message, marshals it into a stack-allocated `MotionPacket`, calls the Rust NIF, receives a fixed-size `Snapshot`, and serializes it back to BEAM terms. Memory allocation happens exactly three times: inbound message receipt, NIF marshal to stack, and NIF unmarshal to BEAM heap. Inside the NIF call, zero bytes move.

This is the two-clock architecture: AtomVM operates on wall-clock time with GC pauses, OS preemption, and network jitter as real concerns. The Rust engine operates on instruction-count time with exact cycle budgets and no runtime surprises. AtomVM is the translator between the two clocks.

## WASM considerations

The WASM target requires specific configuration choices:

**`WasmConfig.batch_size = 10`:** Host calls (JS ↔ WASM boundary crossings) are expensive relative to pure computation. Amortizing 10 traces per host call reduces the JS-side overhead without requiring synchronous JS promises for each individual trace.

**`WasmConfig.max_pages = 16`:** 16 WebAssembly pages = 1 MB of linear memory. This constrains the K-tier selection: K64 (16 KB) fits easily; K512 (64 KB) is feasible but leaves less headroom for signal computation buffers; K1024 (128 KB) is not recommended for browser targets.

**Single-threaded (no `rayon`):** The WASM runtime does not support POSIX threads in the default browser environment (WebAssembly threads require `SharedArrayBuffer`, which has cross-origin isolation requirements). Any code path using `rayon::par_iter` must be replaced with sequential iteration for WASM builds. Feature flags or conditional compilation should gate rayon usage.

**K64 as the browser target:** The combination of 1 MB linear memory, single-threaded execution, and the need for predictable JS frame timing makes K64 the appropriate browser target. Most process intelligence use cases with ≤ 64-place nets are browser-deployable with full bitmask semantics.

## JSON as universal deployment contract

The `AutomlPlan` is the deployment artifact that crosses all tier boundaries. It is a JSON document containing:

- `selected`: the list of signal names in selection order
- `tiers`: the `(signal_name, T-tier)` pairs for each selected signal — tells the deployer exactly which latency class is required
- `fusion`: the operator that combines signals (`Single`, `WeightedVote`, `BordaCount`, `Stack`)
- `predictions`: the calibrated boolean predictions for each test trace
- `plan_accuracy`: accuracy vs anchor
- `signals_evaluated`, `signals_rejected_correlation`, `signals_rejected_no_gain`: accounting fields
- `oracle_signal`, `oracle_vs_gt`, `oracle_gap`: honest disclosure of the difference between HDIT's selection and the ground-truth-optimal selection
- `pareto_front`: the full non-dominated frontier for the deployer to choose their own tradeoff

The `tiers` field is the key deployment-contract field. A deployer reading an `AutomlPlan` does not need to know anything about the signal implementations to understand the deployment requirements: if all `tiers` entries are `T0`, the plan can run on an MCU. If any entry is `Warm`, the plan requires cloud compute.

The JSON format makes the plan diffable across log indices, human-readable without special tooling, deserializable in any language (JavaScript for browser deployment, Elixir for AtomVM boundary processing, Python for offline analysis), and auditable without rebuilding the Rust binary.

The `oracle_gap` field is the plan's honest capability disclosure: it quantifies the performance the system left on the table due to the anchor bias. A plan with `oracle_gap = 0.0` means HDIT selected the ground-truth-optimal signal. A plan with `oracle_gap = 0.06` means a better signal existed that HDIT's conformance-anchored selector did not choose. The deployer can act on this — by overriding the selection, adjusting the anchor, or accepting the gap as the cost of operating without ground truth labels during selection.
