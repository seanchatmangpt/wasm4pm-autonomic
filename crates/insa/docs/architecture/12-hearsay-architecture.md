# KAPPA Template 04: Fuse / HEARSAY-II

Core meaning:
**Fuse = combine multi-source evidence into a single coherent field while detecting and bounding contradictions.**

This comes after Prove / Prolog because even proven facts must be resolved when multiple disparate systems claim conflicting truth about the same object.

---

## 1. Role in the INSA pipeline

```mermaid
flowchart TD
    OStar["O*<br/>closed field context"]
    Sources["Disparate Sources<br/>HR, IAM, Badge, Network, Cloud"]
    Blackboard["Blackboard Slots<br/>bounded evidence arena"]
    Fuse["Fuse / HEARSAY-II<br/>pairwise consistency check"]
    Kappa["KAPPA8 bit 6<br/>Fuse"]
    HEARSAY8["HEARSAY8<br/>fusion detail byte"]
    Instinct["INST8<br/>Settle / Inspect / Escalate / Ask"]
    Resolution["InstinctResolution"]
    POWL8["POWL8 motion"]
    Proof["POWL64<br/>route proof witness"]

    OStar --> Fuse
    Sources --> Blackboard
    Blackboard --> Fuse

    Fuse --> Kappa
    Fuse --> HEARSAY8
    Fuse --> Instinct

    Kappa --> Resolution
    HEARSAY8 --> Resolution
    Instinct --> Resolution
    Resolution --> POWL8
    POWL8 --> Proof
```

---

## 2. Internal 8-bit architecture: HEARSAY8

```mermaid
flowchart LR
    Byte["HEARSAY8 u8"]

    B0["bit 0<br/>SourceAgrees"]
    B1["bit 1<br/>SourceConflicts"]
    B2["bit 2<br/>SourceMissing"]
    B3["bit 3<br/>SourceStale"]
    B4["bit 4<br/>SourceAuthoritative"]
    B5["bit 5<br/>SourceWeak"]
    B6["bit 6<br/>FusionComplete"]
    B7["bit 7<br/>FusionRequiresInspection"]

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
* Success-like bits: SourceAgrees, SourceAuthoritative, FusionComplete
* Failure-like bits: SourceConflicts, SourceMissing, SourceStale, SourceWeak, FusionRequiresInspection

---

## 3. Rust module/component diagram

```mermaid
flowchart TD
    Crate["insa-kappa8::fuse_hearsay"]

    Slots["slots.rs<br/>BlackboardSlot, SourceId, Freshness"]
    Rules["rules.rs<br/>FusionRule, ConflictMask, AgreedMask"]
    Engine["engine.rs<br/>fuse(), pairwise_compare()"]
    Byte["byte.rs<br/>HearsayByte bit ops"]
    Result["result.rs<br/>FusionResult, Status"]
    Fixture["fixtures.rs<br/>canonical fusion cases"]
    Tests["tests/<br/>unit, prop, compile-fail, JTBD"]

    Crate --> Slots
    Crate --> Rules
    Crate --> Engine
    Crate --> Byte
    Crate --> Result
    Crate --> Fixture
    Crate --> Tests

    Slots --> Engine
    Rules --> Engine
    Engine --> Byte
    Engine --> Result
    Fixture --> Tests
```

---

## 4. Execution flow / sequence

```mermaid
sequenceDiagram
    participant Caller as COG8 / Security Closure
    participant Fuse as FuseHearsay
    participant Board as Blackboard
    participant Rules as FusionRules
    participant Result as FusionResult

    Caller->>Fuse: fuse(ctx)
    Fuse->>Board: scan active slots
    Board-->>Fuse: masked evidence facts
    Fuse->>Fuse: bitwise union of agreed masks
    Fuse->>Rules: check agreed vs conflict masks
    Rules-->>Fuse: matched conflict / required rules
    Fuse->>Fuse: assign HEARSAY8 detail
    Fuse-->>Result: HEARSAY8 + agreed mask + conflicted mask + INST8
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid
classDiagram
    class SourceId {
      +u32 raw
    }

    class BlackboardSlot {
      +SlotId id
      +ObjectRef object
      +FieldMask mask
      +SourceId source
      +PolicyEpoch epoch
    }

    class FusionRule {
      +RequiredMask required
      +ConflictMask conflict
      +InstinctByte emits
      +HearsayByte hearsay
    }

    class HearsayByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class FusionResult {
      +CollapseStatus status
      +FieldMask agreed
      +FieldMask conflicted
      +HearsayByte detail
      +InstinctByte emits
    }

    class FuseHearsay {
      +fuse(ctx) FusionResult
    }

    FuseHearsay --> BlackboardSlot
    FuseHearsay --> FusionRule
    FuseHearsay --> FusionResult
    FuseHearsay --> HearsayByte
```

---

## 6. Failure taxonomy

```mermaid
mindmap
  root((Fuse / HEARSAY-II Failures))
    SourceConflicts
      HR says terminated vs IAM says active
      Badge scan contradicts physical location
      Two valid sources claim incompatible states
    SourceMissing
      Required input system offline
      Agent failed to report
      Audit trail incomplete
    SourceStale
      Evidence older than policy epoch
      Cached state bypasses active lock
    FusionRequiresInspection
      Ambiguous agreement
      Threshold of confidence not met
```

---

## 7. Reference vs fast-path admission

```mermaid
flowchart TD
    Fixture["Canonical fusion fixture<br/>slots + rules + expected result"]
    Ref["ReferenceFusePath<br/>O(N^2) pairwise bitwise scalar checks"]
    Simd["SIMD path<br/>batch slot mask fusion"]
    Unsafe["unsafe-admitted path<br/>elided bounds"]

    Compare["Compare FusionResult<br/>agreed, conflicted, HEARSAY8, INST8, status"]
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

Fuse / HEARSAY-II is responsible for confirming that the disparate systems actually represent a real drift and aren't just stale caching.

```mermaid
flowchart TD
    HR["Slot: HR System<br/>mask: terminated"]
    Vendor["Slot: Vendor System<br/>mask: contract_expired"]
    Badge["Slot: Physical Access<br/>mask: badge_active + site_entry"]
    IAM["Slot: SSO/IAM<br/>mask: vpn_active + repo_active"]

    Fuse["Fuse / HEARSAY-II"]
    
    Agreed["Agreed Mask<br/>terminated + expired + active_access"]
    Conflict["Conflict Mask<br/>None (they agree on the drift)"]
    
    Status["HEARSAY8: SourceAgrees + FusionComplete"]
    Instinct["INST8: Inspect / Escalate"]
    Motion["POWL8: Block / Escalate"]

    HR --> Fuse
    Vendor --> Fuse
    Badge --> Fuse
    IAM --> Fuse

    Fuse --> Agreed
    Fuse --> Conflict
    Agreed --> Status
    Conflict --> Status
    Status --> Instinct
    Instinct --> Motion
```
