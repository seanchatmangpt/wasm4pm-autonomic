# KAPPA Template 07: Reconstruct / DENDRAL

Core meaning:
**Reconstruct = infer hidden structure or full timelines from fragmented evidence while rejecting impossible combinations.**

This is crucial because enterprises often only have partial logs, and need a deterministic way to prove what happened without hallucinating.

---

## 1. Role in the INSA pipeline

```mermaid
flowchart TD
    OStar["O*<br/>closed field context"]
    Fragments["Fragmented Evidence<br/>isolated logs, orphaned tickets, disconnected events"]
    Dendral["Reconstruct / DENDRAL<br/>generate and prune candidate structures"]
    Kappa["KAPPA8 bit 5<br/>Reconstruct"]
    DENDRAL8["DENDRAL8<br/>reconstruction detail byte"]
    Instinct["INST8<br/>Retrieve / Ask / Inspect / Escalate / Settle"]
    Resolution["InstinctResolution"]
    POWL8["POWL8 motion"]
    Proof["POWL64<br/>route proof witness"]

    OStar --> Dendral
    Fragments --> Dendral

    Dendral --> Kappa
    Dendral --> DENDRAL8
    Dendral --> Instinct

    Kappa --> Resolution
    DENDRAL8 --> Resolution
    Instinct --> Resolution
    Resolution --> POWL8
    POWL8 --> Proof
```

---

## 2. Internal 8-bit architecture: DENDRAL8

```mermaid
flowchart LR
    Byte["DENDRAL8 u8"]

    B0["bit 0<br/>FragmentsSufficient"]
    B1["bit 1<br/>CandidateGenerated"]
    B2["bit 2<br/>CandidatePruned"]
    B3["bit 3<br/>UniqueReconstruction"]
    B4["bit 4<br/>MultipleReconstructions"]
    B5["bit 5<br/>MissingFragment"]
    B6["bit 6<br/>ConstraintViolation"]
    B7["bit 7<br/>ReconstructionUnstable"]

    Byte --> B0
    Byte --> B1
    Byte --> B2
    Byte --> B3
    Byte --> B4
    Byte --> B5
    Byte --> B6
    Byte --> B7
```

Semantic law:
* Success-like bits: FragmentsSufficient, CandidateGenerated, CandidatePruned, UniqueReconstruction
* Failure-like bits: MultipleReconstructions, MissingFragment, ConstraintViolation, ReconstructionUnstable

---

## 3. Rust module/component diagram

```mermaid
flowchart TD
    Crate["insa-kappa8::reconstruct_dendral"]

    Domain["domain.rs<br/>Fragment, FragmentId, TimeRange"]
    Engine["engine.rs<br/>generate(), prune(), rank()"]
    Byte["byte.rs<br/>DendralByte bit ops"]
    Result["result.rs<br/>ReconstructionResult, Status"]
    Fixture["fixtures.rs<br/>canonical reconstruction cases"]
    Tests["tests/<br/>unit, prop, compile-fail, JTBD"]

    Crate --> Domain
    Crate --> Engine
    Crate --> Byte
    Crate --> Result
    Crate --> Fixture
    Crate --> Tests

    Domain --> Engine
    Engine --> Byte
    Engine --> Result
    Fixture --> Tests
```

---

## 4. Execution flow / sequence

```mermaid
sequenceDiagram
    participant Caller as COG8 / Security Closure
    participant Engine as ReconstructDendral
    participant Arena as CandidateArena
    participant Result as ReconstructionResult

    Caller->>Engine: evaluate(ctx)
    Engine->>Arena: load fragments
    Arena-->>Engine: combinatoric candidates (bounded)
    Engine->>Engine: prune candidates using forbidden constraints
    Engine->>Engine: rank remaining by completion mask
    Engine->>Engine: assign DENDRAL8 detail
    Engine-->>Result: DENDRAL8 + selected candidate + INST8
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid
classDiagram
    class Fragment {
      +FragmentId id
      +ObjectRef object
      +DictionaryDigest digest
      +FieldMask mask
    }

    class ReconstructionCandidate {
      +u32 id
      +FieldMask support
      +u64 satisfied
      +u32 score
    }

    class DendralByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class ReconstructionResult {
      +CollapseStatus status
      +Option~u32~ selected
      +DendralByte detail
      +InstinctByte emits
    }

    class ReconstructDendral {
      +fragments: slice
      +required_mask: FieldMask
      +evaluate(ctx) ReconstructionResult
    }

    ReconstructDendral --> Fragment
    ReconstructDendral --> ReconstructionCandidate
    ReconstructDendral --> ReconstructionResult
    ReconstructionResult --> DendralByte
```

---

## 6. Failure taxonomy

```mermaid
mindmap
  root((Reconstruct / DENDRAL Failures))
    MultipleReconstructions
      Too many valid timelines
      Requires human/HITL disambiguation
    MissingFragment
      Required evidence link is entirely missing
      Triggers RETRIEVE or ASK
    ConstraintViolation
      Fragments exist but contradict policy constraints (e.g. time travel)
    ReconstructionUnstable
      Combinatoric explosion exceeded bounded arena
```

---

## 7. Reference vs fast-path admission

```mermaid
flowchart TD
    Fixture["Canonical fragment fixture<br/>fragments + constraints"]
    Ref["ReferenceDendralPath<br/>scalar candidate generation and prune"]
    Simd["SIMD constraint prune path<br/>batch reject impossible structures"]
    Unsafe["unsafe-admitted arena path<br/>elided allocations"]

    Compare["Compare ReconstructionResult<br/>selected candidate, DENDRAL8, INST8"]
    Admit{"Equivalent?"}
    Good["Admit fast path"]
    Bad["Reject path"]

    Fixture --> Ref
    Fixture --> Simd
    Fixture --> Unsafe

    Ref --> Compare
    Simd --> Compare
    Unsafe --> Compare

    Compare --> Admit
    Admit -- yes --> Good
    Admit -- no --> Bad
```

---

## 8. JTBD instantiation: Access Drift case

Case:
terminated contractor still has active badge, VPN, repo access, vendor relationship, and recent site/device activity.

DENDRAL reconstructs the timeline of access: Did the termination event reach the downstream systems but fail to process, or was it never sent?

```mermaid
flowchart TD
    Fragments["Fragments<br/>HR termination log, IAM sync log, Badge exception log"]
    Constraints["Constraints<br/>sync must follow termination"]
    
    Dendral["Reconstruct / DENDRAL"]
    
    Status["DENDRAL8: CandidateGenerated + CandidatePruned + MissingFragment"]
    Instinct["INST8: Retrieve / Inspect"]
    Motion["POWL8: Block / Retrieve Sync Evidence"]

    Fragments --> Dendral
    Constraints --> Dendral
    Dendral --> Status
    Status --> Instinct
    Instinct --> Motion
```
