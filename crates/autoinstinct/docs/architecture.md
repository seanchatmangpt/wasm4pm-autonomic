# AutoInstinct v30.1.1 — Architecture

> `ccog` executes Autonomic Instincts. AutoInstinct learns, proves, and compiles them.
>
> **Governing law:** `A = μ(O*)`. Raw observation does not authorize action.

## C4 Level 1 — System Context

External actors and systems:

- **Domain operators / experts** — supply corrections, outcomes, local meaning.
- **Edge / Fog / IoT sources** — camera, drone, badge, GPS, scanner, sensors, apps.
- **Downstream systems** — ERP, WMS, TMS, EHR, CRM, dashboards.
- **Public ontology commons** — schema.org, PROV-O, SOSA/SSN, SKOS, OWL-Time, GeoSPARQL, QUDT.
- **Proof / audit consumers** — DENDRAL, JSON-LD bundles, replay verifier.

Two software subjects:

- **`ccog` runtime** — executes Autonomic Instincts.
- **AutoInstinct v30.1.1** — trace-to-instinct compiler.

Edge sources push raw signals into ccog. ccog produces admitted responses, traces, and receipts. AutoInstinct ingests trace-backed episodes plus operator corrections, runs the gauntlet, and publishes admitted field packs back through the registry to ccog. Downstream systems receive admitted cognition events, never raw unresolved alerts. Audit consumers verify proof bundles independently.

## C4 Level 2 — Containers

| Container | Role |
|---|---|
| `ccog` runtime | O* closure → decide → materialize → seal → trace → replay |
| AutoInstinct compiler | corpus → motifs → synth → JTBD → gauntlet → field-pack compiler |
| Field-pack registry + drift monitor | versioned packs, deployment, outcome feedback |
| OCEL world generator | LLM- or scenario-spec-driven synthetic worlds for cold-start |

Core loop: **runtime traces → motif discovery → candidate instinct → gauntlet → compiled pack → redeploy → monitor → learn again.**

## C4 Level 3 — AutoInstinct components

Mapped to `crates/autoinstinct/src/*`:

| Module | Responsibility |
|---|---|
| `corpus.rs` | append-only `Episode`/`TraceCorpus` with `(context_urn, response, receipt_urn, outcome)` |
| `ocel.rs` | OCEL 2.0 log + public-ontology + integrity validation |
| `motifs.rs` | deterministic `(context_urn, response)` co-occurrence motif discovery |
| `synth.rs` | candidate μ policy synthesis (canonical lattice only) |
| `jtbd.rs` | generated `JtbdScenario` triad (positive + perturbation + forbidden-class boundary) |
| `gauntlet.rs` | admit/deny gate with typed `Counterexample` on failure |
| `compile.rs` | `FieldPackArtifact` emission with `urn:blake3` digest |
| `drift.rs` | runtime outcome / mismatch monitor |
| `registry.rs` | versioned pack registry keyed by `(name, digest_urn)` |

## Gauntlet test surfaces

A candidate is admitted only when **every** surface passes:

1. positive JTBD
2. negative boundary
3. perturbation
4. metamorphic invariance
5. warm-vs-hot differential
6. trace replay equivalence
7. receipt sensitivity
8. POWL64 path tamper detection
9. mutation testing
10. RDF / topology fuzzing
11. benchmark-tier validation
12. ontology + PII audit

The current ccog gauntlet (`crates/ccog/tests/gauntlet.rs`) implements 1, 4, 5, 6, 11. AutoInstinct's `gauntlet.rs` implements 1+3 over policy candidates. Phase 2 adds 7, 8, 9, 10, 12 surfaces.

## Field pack structure

A compiled `FieldPackArtifact` carries:

- ontology profile (allowed public IRIs)
- admitted breeds (`Eliza`, `Mycin`, `Strips`, `Shrdlu`, `Prolog`, `Hearsay`, `Dendral`, `Gps`, `Soar`, `Prs`, `Cbr`)
- `(context_urn, response)` rules (canonical lattice only)
- default fallback (safe `Ignore`)
- AutoInstinct version
- `urn:blake3` digest of canonical bytes (excluding the digest field itself)

## Runtime feedback loop

```
Source → ccog → trace → AutoInstinct → gauntlet → registry → ccog
                  ↑                                      │
                  └──────────── outcome monitor ─────────┘
```

ccog stays the execution authority. AutoInstinct learns and compiles; it never executes.

## See also

- `README.md` and `Cargo.toml` for crate surface
- `tests/pipeline_e2e.rs` for the supply-chain end-to-end proof
- `docs/wasm4pm-process-mining-reference.md` for reusable process mining algorithms
- `crates/ccog/CLAUDE.md` for the constitutional rules every pack must obey
