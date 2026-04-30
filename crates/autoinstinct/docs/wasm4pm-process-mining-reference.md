# Process Mining Reference — `~/wasm4pm` (pictl)

Survey of `~/wasm4pm` (a.k.a. pictl) for process-mining algorithms AutoInstinct
can reuse instead of reinventing. pictl ships as a Rust workspace with
WASM-targeted, branchless, deterministic implementations.

## Workspace shape

```
wasm4pm/
  Cargo.toml                    # workspace
  wasm4pm/                      # WASM bindings
  tps-metrics/                  # Toyota Production System metrics
  crates/
    pictl-types/                # core data types
    pictl-algos/                # discovery + conformance algorithms
```

## Reusable types — `pictl-types`

| File | Public API |
|---|---|
| `event_log.rs` | `AttributeValue`, `Event`, `Trace`, `EventLog` |
| `ocel.rs` | `OCEL`, `OCELEvent`, `OCELObject`, `OCELEventObjectRef` (OCEL 2.0) |
| `conformance.rs` | `TokenReplayResult`, `ConformanceResult` |
| `models.rs` | Petri net + DFG + heuristic-net models |
| `provenance.rs` | provenance metadata |
| `hash.rs` | deterministic hashing |
| `error.rs` | typed error enum |

The `OCEL` shape in pictl is richer than AutoInstinct's current `ocel.rs`: it
carries `attributes: Attributes`, `ordering: String`, `version: String`, plus
typed `OCELEvent.object_refs` and `OCELObject.ovmap`. AutoInstinct should
consider re-exporting or path-deping `pictl-types::ocel` once Phase 2 lands
LLM-driven world generation.

## Reusable algorithms — `pictl-algos`

| File | Function | Use in AutoInstinct |
|---|---|---|
| `dfg.rs` | `discover_dfg(log, activity_key) -> DFG` | first-pass motif candidate from event log |
| `alpha.rs` | `discover_alpha(log, activity_key) -> PetriNet` | structural model for POWL8 candidate |
| `heuristic.rs` | `discover_heuristic(log, activity_key) -> DFG` | noise-tolerant motif discovery |
| `conformance.rs` | `check_conformance_token_replay(log, model, key)` | gauntlet replay surface |
| `conformance.rs` | `check_conformance_alignment(log, model, key) -> Vec<TraceAlignment>` | trace-vs-model diffing |
| `streaming.rs` | streaming variants | edge / fog real-time discovery |
| `columnar.rs` | columnar layouts | cache-friendly large logs |

Notes:

- Implementations are explicitly **branchless + deterministic** — same input
  always produces the same output. This matches AutoInstinct's "deterministic
  motif discovery" requirement.
- `check_conformance_alignment` returns `TraceAlignment` with per-step
  classifications. This maps directly onto a Phase-2 gauntlet replay surface
  for AutoInstinct: synthesize a candidate POWL8 plan, replay traces against
  it, reject candidates whose alignment cost crosses a threshold.
- `discover_alpha` produces a Petri net (place/transition). AutoInstinct can
  translate these into POWL8 nodes (`Sequence`, `Parallel`, `Choice`, `Loop`)
  for plan candidates.

## Integration recommendation

When AutoInstinct Phase 2 lands:

1. Add path-dep on `pictl-types` in `crates/autoinstinct/Cargo.toml` so
   AutoInstinct's `OCEL` and `EventLog` surfaces are the same canonical types
   pictl already validates.
2. Pull `discover_dfg` + `discover_alpha` into `motifs.rs` as the structural
   motif discovery engine. Wrap them so motif outputs are still
   `(context_urn, response)` pairs for the gauntlet.
3. Pull `check_conformance_alignment` into `gauntlet.rs` as the trace-replay
   admission surface. Reject candidate policies whose alignment cost over a
   held-out trace exceeds the configured threshold.
4. Pull `streaming.rs` discovery into `drift.rs` for online drift detection.

## Versioning + license

- pictl version: `26.4.17` (workspace-pinned)
- pictl license: MIT OR Apache-2.0
- pictl-algos VERSION: `26.4.10`

These licenses are compatible with ccog/AutoInstinct's BUSL-1.1 stack as
upstream dependencies.

## Other notable pictl features

- **Van der Aalst Agents** — 8 autonomous adversarial agents for manufacturing
  integrity validation using process mining principles (soundness,
  conformance, multi-surface corroboration). Maps to AutoInstinct's
  gauntlet philosophy.
- **TPS (Toyota Production System) metrics** — `tps-metrics` crate with
  fail-fast principles. Compatible doctrine.
- **WASM deployment profiles** — mobile / edge / fog / iot / browser targets
  (~500KB to 2.78MB). Useful template for AutoInstinct's eventual edge/fog
  deployment story.

## What NOT to import

- pictl's WASM-binding layer (`wasm4pm/`) — out of scope; ccog runs native
  Rust, AutoInstinct runs cloud-side.
- pictl's HTTP service layer — AutoInstinct uses the registry + DENDRAL CLI.
- pictl's CLI tool — AutoInstinct will ship its own `autoinstinct` CLI.
