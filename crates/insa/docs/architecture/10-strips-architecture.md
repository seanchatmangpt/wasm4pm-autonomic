# KAPPA Template 02: Precondition / STRIPS

Core meaning:
**Precondition = Is the action enabled or blocked under the current field state?**
Once objects are grounded, the system must evaluate whether the desired autonomic motion (or projected action) violates field invariants or lacks required completion constraints.

---

## 1. Role in the INSA pipeline

```mermaid
flowchart TD
    OStar["O*<br/>closed field context"]
    Action["Candidate Action / Motion Intent"]
    Strips["Precondition / STRIPS<br/>evaluate against schema"]
    Kappa["KAPPA8 bit 1<br/>Precondition"]
    Strips8["STRIPS8<br/>precondition detail byte"]
    Instinct["INST8<br/>Refuse / Await / Retrieve / Settle"]
    Resolution["InstinctResolution"]
    Motion["POWL8 motion<br/>BLOCK or EMIT"]

    OStar --> Strips
    Action --> Strips
    Strips --> Kappa
    Strips --> Strips8
    Strips --> Instinct
    Kappa --> Resolution
    Strips8 --> Resolution
    Instinct --> Resolution
    Resolution --> Motion
```

---

## 2. Internal 8-bit architecture: STRIPS8

```mermaid
flowchart LR
    Byte["STRIPS8 u8"]

    B0["bit 0<br/>PreconditionsSatisfied"]
    B1["bit 1<br/>MissingRequired"]
    B2["bit 2<br/>ForbiddenPresent"]
    B3["bit 3<br/>EffectsKnown"]
    B4["bit 4<br/>EffectsConflict"]
    B5["bit 5<br/>ActionEnabled"]
    B6["bit 6<br/>ActionBlocked"]
    B7["bit 7<br/>RequiresReplan"]

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
* success-like bits: PreconditionsSatisfied, EffectsKnown, ActionEnabled
* failure-like bits: MissingRequired, ForbiddenPresent, EffectsConflict, ActionBlocked, RequiresReplan

---

## 3. Rust module/component diagram

```mermaid
flowchart TD
    Crate["insa-kappa8::precondition_strips"]

    Domain["schema.rs<br/>ActionId, ActionSchema, Add/Clear Effects"]
    Ctx["context.rs<br/>State Mask, Policy Epoch"]
    Resolve["evaluator.rs<br/>evaluate_schema(), Missing/Forbidden check"]
    Byte["byte.rs<br/>StripsByte bit ops"]
    Result["result.rs<br/>PreconditionResult, Status"]
    Fixture["fixtures.rs<br/>canonical precondition scenarios"]
    Tests["tests/<br/>unit, prop, compile-fail, JTBD"]

    Crate --> Domain
    Crate --> Ctx
    Crate --> Resolve
    Crate --> Byte
    Crate --> Result
    Crate --> Fixture
    Crate --> Tests

    Domain --> Resolve
    Ctx --> Resolve
    Resolve --> Byte
    Resolve --> Result
    Fixture --> Tests
```

---

## 4. Execution flow / sequence

```mermaid
sequenceDiagram
    participant Caller as COG8 / Decision Graph
    participant Strips as PreconditionStrips
    participant Schema as ActionSchema
    participant Result as PreconditionResult

    Caller->>Strips: evaluate(action, ctx)
    Strips->>Schema: load constraints
    Schema-->>Strips: required/forbidden masks
    Strips->>Strips: missing = required ^ present
    Strips->>Strips: forbidden = present & schema.forbidden
    Strips-->>Result: STRIPS8 + satisfied boolean + INST8
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid
classDiagram
    class ActionId {
      +u32 raw
    }

    class ActionSchema {
      +ActionId id
      +FieldMask preconditions
      +FieldMask forbidden
      +FieldMask add_effects
      +FieldMask clear_effects
    }

    class ClosureCtx {
      +FieldMask present
    }

    class StripsByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class PreconditionResult {
      +bool satisfied
      +FieldMask missing_required
      +FieldMask present_forbidden
      +StripsByte detail
      +InstinctByte emits
    }

    class PreconditionStrips {
      +schemas: slice
      +evaluate_schema(schema, ctx) PreconditionResult
    }

    PreconditionStrips --> ActionSchema
    PreconditionStrips --> ClosureCtx
    PreconditionStrips --> StripsByte
    PreconditionStrips --> PreconditionResult
```

---

## 6. Failure taxonomy

```mermaid
mindmap
  root((Precondition / STRIPS Failures))
    ActionBlocked
      MissingRequired
        field closure gap
        evidence missing
        policy epoch mismatch
      ForbiddenPresent
        unsafe state
        conflicting evidence active
        terminal state reached
    EffectsConflict
      mutations overwrite locked state
      invalid topology state transition
    RequiresReplan
      deadlock detected
      goal unreachable from current state
```

---

## 7. Reference vs fast-path admission

```mermaid
flowchart TD
    Fixture["Canonical precondition fixture<br/>schemas + ctx state"]
    Ref["ReferenceStripsPath<br/>scalar missing/forbidden XOR checks"]
    Simd["SIMD schema batch path<br/>eval multiple schemas in parallel"]
    Unsafe["unsafe-admitted path<br/>elided bounds checking"]

    Compare["Compare PreconditionResult<br/>satisfied, missing, STRIPS8, INST8"]
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

**Rule:**
No accelerated batch precondition evaluation without strict equivalence to the bitwise scalar reference law.

---

## 8. JTBD instantiation: Access Drift case

Case:
terminated contractor still has active badge, VPN, repo access, vendor relationship, and recent site/device activity

Precondition check is needed before remediation action:
* Is RevokeBadgeAccess legally enabled under the PolicyEpoch?
* Does RevokeBadgeAccess conflict with a locked EmergencyException field?
* What are the delta effects (clear_effects)?

```mermaid
flowchart TD
    State["AccessDrift O* State<br/>badge active, VPN active, exception missing"]
    Action["Candidate Action: RevokeBadge"]
    Strips["Precondition / STRIPS"]
    
    Check1["MissingRequired: None"]
    Check2["ForbiddenPresent: ExceptionMask == 0 (Pass)"]
    Effects["EffectsKnown: clear BADGE_ACTIVE bit"]
    
    Status["STRIPS8: PreconditionsSatisfied + ActionEnabled"]
    Instinct["INST8: empty (proceed)"]
    Motion["POWL8: Block Access Drift / Emit Delta"]

    State --> Strips
    Action --> Strips
    Strips --> Check1
    Strips --> Check2
    Check1 --> Effects
    Check2 --> Effects
    Effects --> Status
    Effects --> Instinct
    Status --> Motion
    Instinct --> Motion
```
