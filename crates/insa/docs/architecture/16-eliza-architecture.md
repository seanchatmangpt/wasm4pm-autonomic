# KAPPA Template 08: Reflect / ELIZA

Core meaning:
**Reflect = restate, clarify, and constrain language-based input before committing it as authoritative execution intent.**

This operates early in the pipeline when raw strings (human speech, agent tool outputs, unstructured logs) first enter the system.

---

## 1. Role in the INSA pipeline

```mermaid
flowchart TD
    OStar["O*<br/>closed field context"]
    Input["Raw String / Prompt / Tool Output"]
    Eliza["Reflect / ELIZA<br/>clarify missing owner or evidence"]
    Kappa["KAPPA8 bit 0<br/>Reflect"]
    ELIZA8["ELIZA8<br/>reflect detail byte"]
    Instinct["INST8<br/>Ask / Inspect / Await / Retrieve"]
    Resolution["InstinctResolution"]
    POWL8["POWL8 motion"]
    Proof["POWL64<br/>route proof witness"]

    OStar --> Eliza
    Input --> Eliza

    Eliza --> Kappa
    Eliza --> ELIZA8
    Eliza --> Instinct

    Kappa --> Resolution
    ELIZA8 --> Resolution
    Instinct --> Resolution
    Resolution --> POWL8
    POWL8 --> Proof
```

---

## 2. Internal 8-bit architecture: ELIZA8

```mermaid
flowchart LR
    Byte["ELIZA8 u8"]

    B0["bit 0<br/>MirrorIntent"]
    B1["bit 1<br/>RestateClaim"]
    B2["bit 2<br/>DetectAffect"]
    B3["bit 3<br/>DetectAmbiguity"]
    B4["bit 4<br/>DetectMissingSlot"]
    B5["bit 5<br/>AskClarifying"]
    B6["bit 6<br/>SlowPrematureAction"]
    B7["bit 7<br/>DeferToClosure"]

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
* Success-like bits: MirrorIntent, RestateClaim, AskClarifying, DeferToClosure
* Failure-like bits (requiring human loop): DetectAmbiguity, DetectMissingSlot, SlowPrematureAction

---

## 3. Rust module/component diagram

```mermaid
flowchart TD
    Crate["insa-kappa8::reflect_eliza"]

    Domain["domain.rs<br/>ReflectPattern, SlotGap, AskKind"]
    Engine["engine.rs<br/>detect_slot_gap(), evaluate()"]
    Byte["byte.rs<br/>ElizaByte bit ops"]
    Result["result.rs<br/>CollapseResult, Status"]
    Fixture["fixtures.rs<br/>canonical reflection cases"]
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
    participant Engine as ReflectEliza
    participant Patterns as ReflectPatternTable
    participant Result as CollapseResult

    Caller->>Engine: evaluate(ctx)
    Engine->>Patterns: check context against templates
    Patterns-->>Engine: required slot matches
    Engine->>Engine: detect_slot_gap(ctx.present)
    Engine->>Engine: assign ELIZA8 detail
    Engine-->>Result: ELIZA8 + INST8 (e.g. ASK) + partial/success
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid
classDiagram
    class AskKind {
      Clarify
      MissingEvidence
    }

    class SlotGap {
      +FieldBit missing
      +AskKind ask_kind
    }

    class ReflectPattern {
      +u32 id
      +FieldMask required_context
      +u32 template_id
      +InstinctByte emits
    }

    class ElizaByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class CollapseResult {
      +CollapseStatus status
      +ElizaByte detail
      +InstinctByte emits
      +FieldMask support
    }

    class ReflectEliza {
      +patterns: slice
      +expected_slots: FieldMask
      +evaluate(ctx) CollapseResult
      +detect_slot_gap(ctx) Option~SlotGap~
    }

    ReflectEliza --> ReflectPattern
    ReflectEliza --> SlotGap
    ReflectEliza --> CollapseResult
    CollapseResult --> ElizaByte
```

---

## 6. Failure taxonomy

```mermaid
mindmap
  root((Reflect / ELIZA Failures))
    DetectAmbiguity
      User input matches multiple distinct intents
      Requires clarification before grounding
    DetectMissingSlot
      Intent is clear but payload is missing (e.g., "revoke access" without stating who)
      Triggers ASK
    SlowPrematureAction
      User commands action before policy allows it
      Triggers AWAIT or ESCALATE
```

---

## 7. Reference vs fast-path admission

```mermaid
flowchart TD
    Fixture["Canonical reflection fixture<br/>input ctx + patterns"]
    Ref["ReferenceElizaPath<br/>scalar mask matching"]
    Simd["SIMD pattern matching<br/>batch evaluate template matches"]

    Compare["Compare CollapseResult<br/>ELIZA8, INST8, missing slots"]
    Admit{"Equivalent?"}
    Good["Admit fast path"]
    Bad["Reject path"]

    Fixture --> Ref
    Fixture --> Simd

    Ref --> Compare
    Simd --> Compare

    Compare --> Admit
    Admit -- yes --> Good
    Admit -- no --> Bad
```

---

## 8. JTBD instantiation: Access Drift case

Case:
terminated contractor still has active badge, VPN, repo access, vendor relationship, and recent site/device activity.

A Security Operator (or Agent) commands: 'Revoke their access.'
ELIZA prevents raw string action.

```mermaid
flowchart TD
    Input["Command: 'Revoke their access'"]
    Ctx["Context: Access Drift Incident"]
    
    Eliza["Reflect / ELIZA"]
    
    Gap["SlotGap Detected<br/>missing: specific asset to revoke"]
    
    Status["ELIZA8: DetectMissingSlot + AskClarifying + SlowPrematureAction"]
    Instinct["INST8: Inspect + Ask"]
    Motion["POWL8: Block Action / Prompt User for Exact Asset"]

    Input --> Ctx
    Ctx --> Eliza
    Eliza --> Gap
    Gap --> Status
    Status --> Instinct
    Instinct --> Motion
```
