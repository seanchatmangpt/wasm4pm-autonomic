# AutoInstinct v30.1.1 — Sparse Priming Representation (SPR)

The constitutional doctrine. Every claim here is enforced by code in
`crates/autoinstinct/` and `crates/ccog/`; deletions or violations break
boundary tests by design.

## Identity

- AutoInstinct is the trace-to-instinct compiler for Autonomic Instincts.
- The command is `ainst`.
- `ccog` executes instincts. `ainst` learns, proves, compiles, publishes, and deploys them.
- AutoML learns predictive models from datasets. AutoInstinct learns lawful
  response policies from proof-backed worlds.
- AutoInstinct converts trace history into field packs.
- A field pack is a compiled, tested, traceable, receipt-backed instinct artifact.
- AutoInstinct does not train opaque authority. It synthesizes candidate μ
  policies and admits only those that survive proof.

## Governing Law

`A = μ(O*)` — raw observation does not authorize action. Closed context authorizes action.

- A signal is not the intelligence. Local interpretation is.
- An event is not action. A model output is not action. A workflow state is
  not action. A bark is not action.
- Action becomes lawful only after closure → decision → materialization →
  sealing → tracing → replay.
- AutoInstinct learns from `(O*, A, Trace, Receipt, Outcome)`, not from raw `O`.
- Training data is not rows; it is trace-backed worlds.

## Substrate Layering

- **OCEL** is the world substrate.
- **RDF** is the semantic substrate.
- **POWL8** is the motion substrate.
- **CONSTRUCT8** is bounded writeback.
- **POWL64** is the ABI / proof substrate.

LLMs generate worlds. AutoInstinct compiles instincts. `ccog` proves actions.

## Public Ontology Profiles

schema.org · PROV-O · SOSA/SSN · SKOS · OWL-Time · GeoSPARQL · QUDT ·
SHACL · ODRL.

LLMs are world generators, not action authorities. The field is the
authority. The model proposes. The gauntlet admits. `ccog` executes. Trace proves.

## Pipeline

`ontology profile → OCEL world → validation → trace corpus → motif discovery
→ candidate μ policy → generated JTBD tests → gauntlet → compiled field pack
→ deployment → outcome feedback`.

## CLI Grammar (canonical, space-separated)

```text
ainst generate ocel
ainst validate ocel
ainst ingest corpus
ainst discover motifs
ainst propose policy
ainst generate jtbd
ainst run gauntlet
ainst compile pack
ainst publish pack
ainst deploy edge
ainst verify replay
ainst export bundle
```

## Canonical Response Lattice

`Settle · Retrieve · Inspect · Ask · Refuse · Escalate · Ignore`.

AutoInstinct optimizes right-sized response, not maximum automation.

## Earned Action

An earned action survives perturbation, replay, receipt sensitivity,
ontology validation, POWL64 tamper, mutation testing, and benchmark-tier
separation.

A fast zero is not admissibility unless the trace proves why zero was
earned. **No unclassified zero may become authority.**

Earned-zero classes:
1. KernelFloor
2. ClosureEarnedAdmission
3. PredecessorSkip
4. RequirementMaskFailure
5. ContextDenial
6. ManualOnlySkip
7. ConformanceFailure

## The Gauntlet

Every JTBD has positive, negative, and perturbation assertions:

- **Positive** — job succeeds when context exists.
- **Negative boundary** — old forbidden behavior is absent.
- **Perturbation** — removing load-bearing context changes or denies the response.

A stub returning `Ok(())`, `true`, empty delta, fake receipt, or
unclassified zero MUST fail.

Boundary detectors are constitutional tests. Historical mistakes become
permanent regression seeds.

## Forbidden Regressions

- fake `prov:value`
- derived-from-prefLabel placeholder text
- SHACL instance misuse
- timestamp receipt identity
- fused decision/materialization
- manual trigger auto-fire
- mask-domain confusion
- decorative POWL64 paths
- pack-bit leakage
- healthcare overclaiming

## Three Identity Surfaces — Never Conflate

| Surface | Proves |
|---|---|
| Receipt material | what was sealed |
| Trace material | how it was earned |
| Benchmark tier | what it cost |

- Receipt identity is semantic, not temporal.
- Trace is causal explanation, not receipt identity.
- Benchmark tier is physical cost classification, not proof.

## Semantic Boundaries

- Missing evidence ⇒ Ask. Missing evidence NEVER ⇒ fabricate evidence.
- Detected gap ≠ supplied evidence.
- Phrase binding ⇒ provenance. NEVER fake definition.
- Transition admissibility ⇒ finding/provenance. NEVER SHACL-shape misuse.

## Benchmark Tiers

`KernelFloor · CompiledBark · Materialization · ReceiptPath · FullProcess · ConformanceReplay`.

Performance claims must never cross tiers. A nanosecond bark is not a
microsecond semantic path.

## Hot-Path Contract

- `decide()` stays allocation-free: no `Vec`, no `format!`, no `Utc::now`,
  no `Construct8`, no fn-pointer acts, no receipt construction, no graph mutation.
- `materialize()` produces CONSTRUCT8.
- `seal()` creates receipt and advances POWL64.
- `trace()` records causal path.
- `replay()` proves conformance.

## POWL8 / POWL64

- POWL8 is executable ISA, not diagram syntax. It compiles motion:
  sequence, parallel, partial order, choice, loop, activity, silence, boundary.
- POWL64 is data ABI, not decorative geometry. It records path, chain
  length, polarity, semantic receipt, collision semantics, and replay identity.
- Coordinate is location, not identity.
- DENDRAL reconstructs proof bundles.

## Field Pack Components

ontology profile · response lattice · bit allocations · admitted breeds ·
BarkSlots · POWL8 plan · CONSTRUCT8 acts · POWL64 policy · generated JTBD
tests · benchmark metadata · conformance expectations · human review docs.

Field packs are not configuration. They are admitted compiled instinct artifacts.

## Pack Profiles

- **Lifestyle** — routine, fatigue, overstimulation, transition smoothing,
  meaningful activity, avoidance vs incapacity, safety risk, missing
  context, trace replay. Bias: fatigue/overstimulation softens hard
  response into Ask or Settle when safety risk is absent.
- **Healthcare** — never diagnose. Support ask/inspect/settle/retrieve/
  refuse/escalate/ignore. Prevent unsupported action from pretending to be supported.
- **Supply Chain** — camera, drone, badge, GPS, IoT, scanner, cold-chain,
  dock, gate, route, customs, shipment, worker, pallet, truck, facility
  zone. Bias: resolve local meaning at edge/fog before downstream SaaS
  receives raw noise.
- **Enterprise** — evidence gaps, transition readiness, owner routing,
  compliance posture, proof-bundle export, process conformance.
- **Developer Governance** — boundary detector preservation, semantic
  mutation testing, benchmark-tier labeling, no auto-merge, nightly-feature
  policy, agent-code review. Dev pack never auto-merges.

## OCEL World Coverage

Generated worlds must include:
normal · missing-evidence · duplicate-signal · false-sensor ·
delayed-confirmation · authorized-exception · unauthorized-access ·
safety-escalation · harmless-settle · adversarial-namespace ·
temporal-disorder · object-relationship-missing.

Worlds are synthetic operational worlds, not flat synthetic rows.
AutoInstinct trains on worlds, not rows.

## Theses

- **Black hole telescope:** AutoInstinct reconstructs admissible action
  from synchronized partial context.
- **LHC:** AutoInstinct collides generated cognition with closure,
  perturbation, replay, mutation, and proof until hidden invariants appear.
- **Event horizon:** events become instinct-synthesis material, not
  downstream workflow noise.
- **Blue River Dam:** expensive SaaS monetizes unresolved context;
  AutoInstinct resolves context upstream.
- **Edge thesis:** whoever closes context first owns downstream layers.
- **SaaS consequence:** downstream SaaS becomes thinner UI, coordination,
  reporting, archive, escalation, and adapter surface.
- **Fortune-5 supply chain:** global supply chains need local semantic
  responsiveness, not more dashboards.
- **Healthcare:** healthcare needs systems that know when not to act,
  when to ask, when to settle, and when escalation is actually earned.
- **Developer:** LLM code generation becomes code manufacturing only when
  generated artifacts are forced through closure, tests, receipts,
  replay, and boundary detectors.
- **Ten-year research thesis:** language generation is not lawful action.

## Final Compression

`ainst` generates worlds, discovers motifs, proposes μ, generates JTBDs,
runs gauntlets, compiles packs, deploys instincts, verifies replay, and
exports proof.
