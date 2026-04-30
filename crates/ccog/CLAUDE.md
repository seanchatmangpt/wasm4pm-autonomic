# CLAUDE.md — `ccog` crate

Compiled Cognition core: field-cognition facade over RDF graph closure with
nanoscale bark dispatch, deterministic receipts, and POWL8/POWL64 motion ABI.

## Doctrine

> ccog knows what the graph permits the field to do.
> Phase 4 made dispatch fast. Phase 5 made it honest. Phase 6 made the codebase
> truthful (no stubs, no mocks, no placeholders, no TODOs).

The governing equation is `A = μ(O*)` — action is lawful only when projected
from a semantically closed, compiled output set. The pipeline is:

```
O_raw + posture + history + context  →  O*  →  Decision  →  Materialize  →  Seal  →  Trace
```

The bark is not the action. **Snapshot + posture + context** is the cognition surface.

## Build

Always use `cargo` against the `ccog` package directly. The parent `dteam`
crate has unrelated WASM/RL compilation issues that don't affect ccog.

| Goal | Command |
|------|---------|
| Build | `cargo build -p ccog` |
| Test (lib + integration) | `cargo test -p ccog` |
| Single test by name | `cargo test -p ccog -- <name>` |
| Bench (full sheet) | `cargo bench -p ccog --bench ccog_hot_path_bench` |
| Bench tier filter | `cargo bench -p ccog --bench ccog_hot_path_bench -- kernel_floor` |
| Lint | `cargo clippy -p ccog -- -D warnings` |

The crate forbids unsafe and denies missing docs:

```rust
#![forbid(unsafe_code)]
#![deny(missing_docs)]
```

Every new public item must have `///` docs.

## Module map

### Hot path (decision → materialize → seal)

| File | Purpose |
|------|---------|
| `compiled.rs` | `CompiledFieldSnapshot` — one-pass HashMap indices over `field.graph.all_triples()` |
| `compiled_hook.rs` | `CompiledHook` + canonical `Predicate` bits + `compute_present_mask` |
| `bark_kernel.rs` | `BarkKernel` (POWL8-ordered) — `decide` (nanoscale) + `materialize` (µs) + `seal` (µs) |
| `bark_artifact.rs` | `&'static [BarkSlot]` const dispatch + `decide`/`materialize`/`seal` for built-ins |
| `admit.rs` | Branchless denial-polarity: `bool_mask`, `admit2/3/4`, `admitted`, `commit_masked_*` |

### Plan + ABI

| File | Purpose |
|------|---------|
| `powl.rs` | `Powl8` kinetic plan + `BinaryRelation` 64×64 + `Powl8::compile() → CompiledPowl8` |
| `powl64.rs` | `Powl64` geometric ABI: ordered chain `path`, polarity-folded BLAKE3, collision-preserving cells |
| `construct8.rs` | Bounded ≤8-triple `Construct8` writeback primitive |
| `receipt.rs` | `Receipt::derive_urn` + `canonical_material` for deterministic `urn:blake3:` activity URNs |

### Cognition + breeds

| File | Purpose |
|------|---------|
| `breeds/eliza.rs` | Phrase binding (skos:prefLabel) |
| `breeds/mycin.rs` | Evidence-gap detection |
| `breeds/strips.rs` | Transition admissibility + `admit_breed` per-breed probes + `admit_powl8` |
| `breeds/shrdlu.rs` | Object affordance |
| `breeds/prolog.rs` | Transitive relation proof (skos:broader) |
| `breeds/hearsay.rs` | Pack-posture fusion from outcomes |
| `breeds/dendral.rs` | PROV chain reconstruction |

### Multimodal + instinct (Phase 6)

| File | Purpose |
|------|---------|
| `multimodal.rs` | `PostureBundle` + `ContextBundle` masks; `PostureBit::*`, `ContextBit::*` |
| `instinct.rs` | `select_instinct_v0` decision lattice → `AutonomicInstinct::{Settle,Retrieve,Inspect,Ask,Refuse,Escalate,Ignore}` |
| `trace.rs` | `CcogTrace`, `BarkNodeTrace`, `BarkSkipReason`, `trace_bark`, `trace_default_builtins` |

### Runtime + warm path

| File | Purpose |
|------|---------|
| `field.rs` | `FieldContext` — opaque graph-bearing field handle |
| `graph.rs` | `GraphStore` SPARQL + direct `quads_for_pattern` primitives |
| `hooks.rs` | Warm-path `KnowledgeHook` 4-tuple registry (reference, NOT hot path) |
| `facade.rs` | `process` and `process_with_hooks` — full warm pipeline |
| `runtime/scheduler.rs` | Tick scheduler, ΔO detection |
| `runtime/delta.rs` | `GraphSnapshot` + `GraphDelta` |
| `runtime/posture.rs` | `PostureMachine` (signal-count-driven Calm/Alert/Engaged/Settled) |
| `runtime/step.rs` | `Runtime::step` — threads BLAKE3 chain via Powl64 |
| `verdict.rs` | `Verdict`, `Breed`, `PlanVerdict`, `PackPosture`, `ProvenanceChain`, etc. |
| `operation.rs` | Candidate `Operation` shape |
| `utils/dense.rs` | `PackedKeyTable` + FNV-1a — zero-alloc graph-snapshot table |

## Performance contract

Three benchmark tiers — **never conflate them in claims**:

| Tier | What it measures | Target |
|------|------------------|--------|
| **KernelFloor** | `decide()` only — pure mask arithmetic, no `Vec`/`format!`/`Utc::now`/`Construct8` | sub-µs |
| **CompiledBark** | `decide()` + `materialize()` — allocates `Construct8` deltas | ≤5µs |
| **Materialization** | Standalone act fns over snapshot | ≤2µs each |
| **ReceiptPath** | `seal()` — BLAKE3 + URN derivation | ≤5µs |
| **FullProcess** | `process_with_hooks` warm path | ≤30µs |
| **ConformanceReplay** | Replay against prior trace | (future) |

Bench tiers are documented at the top of `benches/ccog_hot_path_bench.rs`.

`decide()` MUST stay allocation-free — no `Vec`, no `format!`, no `Utc::now`,
no `Construct8`, no fn-pointer act calls. Only `BarkDecision` is returned.

## Three identity surfaces — keep them separate

1. **Receipt identity material** (`Receipt::canonical_material`): proves *what
   was sealed*. NUL-separated layout: `hook_id || plan_node_le_u16 ||
   delta_receipt_bytes || field_id || prior_chain_or_zero32 || polarity`.
   **Never** includes `Utc::now` or benchmark tier.
2. **Trace record material** (`CcogTrace`): proves *how it was earned*. Records
   present_mask, predecessor masks, typed `BarkSkipReason`.
3. **Benchmark tier** (`BenchmarkTier` annotation): proves *what it cost*.
   Lives only on bench fns; never hashed into identity.

`Utc::now` is allowed only as receipt **metadata timestamp**, never URN material.

## Conformance rules (boundary detectors)

These are non-negotiable and have negative tests in `bark_artifact.rs` and
`hooks.rs`. Violating any of them is a defect:

1. **Missing-evidence hook MUST NOT fabricate evidence.** After applying the
   delta, `check_any_doc_missing_value_snap` must still return `true` for the
   same document. Emit `schema:AskAction` gap-finding triples; never emit
   `<doc> prov:value "placeholder"`.
2. **Phrase-binding hook MUST emit a real provenance edge.** Use
   `prov:wasInformedBy <urn:blake3:{hash(label)}>`; never a `skos:definition
   "derived from prefLabel"` string.
3. **Transition-admissibility hook MUST NOT declare instances as SHACL shapes.**
   Use `prov:Activity` + `prov:used`; never `sh:targetClass` on instance
   subjects.
4. **Receipt activity IRIs MUST be `urn:blake3:` URNs.** No `example.org`
   activity IRIs in production code paths (test fixtures excepted).
5. **No `Utc::now` in URN material.** Timestamps are receipt metadata only.
6. **`HookRegistry::fire_matching` is the warm/reference path.** Hot loops
   must use `BarkKernel` or `bark_artifact::bark`.

## Public ontology vocabulary only

Allowed prefixes (declared in `graph.rs::PREFIXES`):

```
rdf:     http://www.w3.org/1999/02/22-rdf-syntax-ns#
xsd:     http://www.w3.org/2001/XMLSchema#
skos:    http://www.w3.org/2004/02/skos/core#
schema:  https://schema.org/
prov:    http://www.w3.org/ns/prov#
dcterms: http://purl.org/dc/terms/
sh:      http://www.w3.org/ns/shacl#
odrl:    http://www.w3.org/ns/odrl/2/
```

Plus `urn:blake3:{hex}` and `urn:ccog:*` URNs. **No `ccog:` namespace prefix**
in IRIs — verified by `grep '"ccog:"\|<ccog:' src/`.

## Code conventions

- **Hot path is allocation-free.** Decide stages return only fixed-size structs.
- **Const fns where possible.** `admit*`, mask helpers, `GlobeCell::new` are const.
- **Snapshot-native checks first.** Prefer `HookCheck::SnapshotFn`/`SnapshotAdmit`
  over `Fn`/`Admit` (which take a full `FieldContext`).
- **Determinism before convenience.** Receipt URNs come from canonical material,
  not timestamps; chain hashes fold polarity at every step (genesis included).
- **No `Vec` in u64-mask domains.** `BarkDecision` and `Powl8::predecessor_masks`
  use 64-bit masks indexed by plan-node position.
- **Public-ontology IRIs use full URLs.** Never store prefix strings.

## Adding a new built-in hook

1. Add a canonical predicate bit in `compiled_hook::Predicate` if needed.
2. Update `compute_present_mask` to set the bit.
3. Add a `BarkSlot` to `bark_artifact::BUILTINS` with name, mask, act fn,
   `emit_receipt`.
4. Write the act fn — must be `fn(&CompiledFieldSnapshot) -> Result<Construct8>`.
   Emit only public-ontology IRIs and `urn:blake3:` URNs.
5. Optionally add a warm-path `KnowledgeHook` constructor in `hooks.rs` for the
   reference registry.
6. Add a negative test asserting the hook does NOT fabricate facts (the boundary
   detector pattern from `bark_artifact::tests`).

## Adding a new breed

1. Add the variant to `verdict::Breed` with an explicit `repr(u8)` value.
2. Wire its admissibility precondition in `breeds::strips::admit_breed`.
3. Implement the breed's pass in `breeds/<name>.rs`.
4. Add the breed activity case in any plan-builder code paths (the match must be
   exhaustive — `verdict::Breed` is not `#[non_exhaustive]`).
5. Add tests covering positive and negative admissibility.

## Test policy

- 170+ tests must remain green at all times.
- Every public fn needs at least one positive test.
- Conformance/negative tests live alongside the code they protect — do not
  remove them when refactoring (they encode boundary detectors).
- Property tests (`proptest`) cover Construct8 invariants and POWL8 cycle
  detection.

## See also

- `/Users/sac/dteam/CLAUDE.md` — parent `dteam` crate guidance
- `/Users/sac/dteam/docs/COMPILED_COGNITION.md` — architecture paper
- `/Users/sac/.claude/plans/yes-i-think-synchronous-squirrel.md` — most recent phase plan
