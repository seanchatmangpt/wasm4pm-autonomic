# KAPPA Template 05: ReduceGap / GPS

Core meaning:
**ReduceGap = determine the deterministic bounded delta needed to transition the current state to the goal state.**

This comes after Fuse / HEARSAY-II because once the field is coherent, the system must decide *what exact operations* are needed to reach compliance.

---

## 1. Role in the INSA pipeline

```mermaid
flowchart TD
    OStar["O*<br/>closed field context"]
    Goal["Goal State<br/>required masks + forbidden masks"]
    GPS["ReduceGap / GPS<br/>compute difference and select operator"]
    Kappa["KAPPA8 bit 7<br/>ReduceGap"]
    GPS8["GPS8<br/>gap reduction detail byte"]
    Instinct["INST8<br/>Retrieve / Ask / Await / Settle / Escalate"]
    Resolution["InstinctResolution"]
    POWL8["POWL8 motion"]
    Proof["POWL64<br/>route proof witness"]

    OStar --> GPS
    Goal --> GPS

    GPS --> Kappa
    GPS --> GPS8
    GPS --> Instinct

    Kappa --> Resolution
    GPS8 --> Resolution
    Instinct --> Resolution
    Resolution --> POWL8
    POWL8 --> Proof
```

---

## 2. Internal 8-bit architecture: GPS8

```mermaid
flowchart LR
    Byte["GPS8 u8"]

    B0["bit 0<br/>GoalKnown"]
    B1["bit 1<br/>GapDetected"]
    B2["bit 2<br/>GapSmall"]
    B3["bit 3<br/>GapLarge"]
    B4["bit 4<br/>OperatorAvailable"]
    B5["bit 5<br/>OperatorBlocked"]
    B6["bit 6<br/>ProgressMade"]
    B7["bit 7<br/>NoProgress"]

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
* Success-like bits: GoalKnown, OperatorAvailable, ProgressMade
* Failure-like bits: GapDetected, GapLarge, OperatorBlocked, NoProgress

---

## 3. Rust module/component diagram

```mermaid
flowchart TD
    Crate["insa-kappa8::reduce_gap_gps"]

    State["state.rs<br/>GoalState, CurrentState, Gap"]
    Ops["operators.rs<br/>GapOperator, resolutions, preconditions"]
    Engine["engine.rs<br/>compute_gap(), search()"]
    Byte["byte.rs<br/>GpsByte bit ops"]
    Result["result.rs<br/>GapReductionResult, Status"]
    Fixture["fixtures.rs<br/>canonical gap cases"]
    Tests["tests/<br/>unit, prop, compile-fail, JTBD"]

    Crate --> State
    Crate --> Ops
    Crate --> Engine
    Crate --> Byte
    Crate --> Result
    Crate --> Fixture
    Crate --> Tests

    State --> Engine
    Ops --> Engine
    Engine --> Byte
    Engine --> Result
    Fixture --> Tests
```

---

## 4. Execution flow / sequence

```mermaid
sequenceDiagram
    participant Caller as COG8 / Security Closure
    participant GPS as ReduceGapGps
    participant State as GoalState
    participant Ops as GapOperators
    participant Result as GapReductionResult

    Caller->>GPS: evaluate(ctx)
    GPS->>State: calculate missing/forbidden gap
    State-->>GPS: gap mask + width
    GPS->>Ops: find operator resolving gap bits
    Ops-->>GPS: best operator matching preconditions
    GPS->>GPS: assign GPS8 detail
    GPS-->>Result: GPS8 + selected operator + INST8 + gap
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid
classDiagram
    class GoalState {
      +RequiredMask required
      +ForbiddenMask forbidden
      +CompletedMask completed
    }

    class Gap {
      +FieldMask missing_required
      +FieldMask present_forbidden
      +u8 width
    }

    class GapOperator {
      +OperatorId id
      +FieldMask required_preconditions
      +FieldMask resolves
      +InstinctByte emits
    }

    class GpsByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class GapReductionResult {
      +CollapseStatus status
      +Gap gap
      +OperatorId selected_operator
      +GpsByte detail
      +InstinctByte emits
    }

    class ReduceGapGps {
      +reduce(ctx) GapReductionResult
    }

    ReduceGapGps --> GoalState
    ReduceGapGps --> Gap
    ReduceGapGps --> GapOperator
    ReduceGapGps --> GapReductionResult
    ReduceGapGps --> GpsByte
```

---

## 6. Failure taxonomy

```mermaid
mindmap
  root((ReduceGap / GPS Failures))
    GapLarge
      too many bits to resolve in one step
      requires replanning / decomposing
    OperatorBlocked
      operator exists but preconditions fail
      requires backward chaining
    NoProgress
      gap exists but no operator applies
      deadlock detected
      stuck state requires escalation
```

---

## 7. Reference vs fast-path admission

```mermaid
flowchart TD
    Fixture["Canonical gap fixture<br/>state + goal + operators"]
    Ref["ReferenceGpsPath<br/>scalar missing/forbidden XOR checks"]
    Simd["SIMD path<br/>batch operator matching"]
    Unsafe["unsafe-admitted path<br/>elided bounds"]

    Compare["Compare GapReductionResult<br/>gap width, selected op, GPS8, INST8, status"]
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

ReduceGap / GPS is responsible for calculating exactly what operations are required to reach the PolicyCompliant state.

```mermaid
flowchart TD
    State["Current State<br/>badge_active + vpn_active + repo_active"]
    Goal["Goal State<br/>forbidden: badge_active, vpn_active, repo_active"]
    
    GPS["ReduceGap / GPS"]
    
    Gap["Gap Detected<br/>width: 3 bits to clear"]
    
    Op1["Operator: RevokeBadge"]
    Op2["Operator: RevokeVPN"]
    Op3["Operator: RevokeRepo"]
    
    Status["GPS8: GapDetected + ProgressMade + OperatorAvailable"]
    Instinct["INST8: Retrieve / Escalate"]
    Motion["POWL8: Loop/Act to apply operators"]

    State --> GPS
    Goal --> GPS
    GPS --> Gap
    Gap --> Op1
    Gap --> Op2
    Gap --> Op3

    Op1 --> Status
    Status --> Instinct
    Instinct --> Motion
```
