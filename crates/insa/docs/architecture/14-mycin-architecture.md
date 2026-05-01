# KAPPA Template 06: Rule / MYCIN

Core meaning:
**Rule = bounded application of expert rules over verified evidence fields, yielding a deterministic certainty conclusion.**

This comes after Fuse / HEARSAY-II because rules must operate on resolved, non-conflicting field states.

---

## 1. Role in the INSA pipeline

```mermaid
flowchart TD
    OStar["O*<br/>closed field context"]
    Fuse["Fused Evidence Field"]
    MYCIN["Rule / MYCIN<br/>apply expert rules"]
    Kappa["KAPPA8 bit 4<br/>Rule"]
    MYCIN8["MYCIN8<br/>rule detail byte"]
    Instinct["INST8<br/>Refuse / Inspect / Escalate / Settle"]
    Resolution["InstinctResolution"]
    POWL8["POWL8 motion"]
    Proof["POWL64<br/>route proof witness"]

    OStar --> MYCIN
    Fuse --> MYCIN

    MYCIN --> Kappa
    MYCIN --> MYCIN8
    MYCIN --> Instinct

    Kappa --> Resolution
    MYCIN8 --> Resolution
    Instinct --> Resolution
    Resolution --> POWL8
    POWL8 --> Proof
```

---

## 2. Internal 8-bit architecture: MYCIN8

```mermaid
flowchart LR
    Byte["MYCIN8 u8"]

    B0["bit 0<br/>RuleMatched"]
    B1["bit 1<br/>RuleFired"]
    B2["bit 2<br/>RuleConflict"]
    B3["bit 3<br/>ConfidenceHigh"]
    B4["bit 4<br/>ConfidenceLow"]
    B5["bit 5<br/>PolicyEpochValid"]
    B6["bit 6<br/>PolicyEpochStale"]
    B7["bit 7<br/>ExpertReviewRequired"]

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
* Success-like bits: RuleMatched, RuleFired, ConfidenceHigh, PolicyEpochValid
* Failure-like bits: RuleConflict, ConfidenceLow, PolicyEpochStale, ExpertReviewRequired

---

## 3. Rust module/component diagram

```mermaid
flowchart TD
    Crate["insa-kappa8::rule_mycin"]

    Domain["rules.rs<br/>ExpertRule, RuleId, CertaintyLane"]
    Engine["engine.rs<br/>evaluate_rules()"]
    Byte["byte.rs<br/>MycinByte bit ops"]
    Result["result.rs<br/>RuleClosureResult, Status"]
    Fixture["fixtures.rs<br/>canonical rule cases"]
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
    participant Engine as RuleMycin
    participant Rules as ExpertRuleTable
    participant Result as RuleClosureResult

    Caller->>Engine: evaluate(ctx)
    Engine->>Rules: scan for matches
    Rules-->>Engine: required/forbidden constraints
    Engine->>Engine: filter by context mask and policy epoch
    Engine->>Engine: select highest certainty / detect conflicts
    Engine->>Engine: assign MYCIN8 detail
    Engine-->>Result: MYCIN8 + fired rule + INST8
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid
classDiagram
    class CertaintyLane {
      +u8 value
    }

    class ExpertRule {
      +RuleId id
      +RequiredMask required
      +ForbiddenMask forbidden
      +InstinctByte emits
      +CertaintyLane certainty
    }

    class MycinByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class RuleClosureResult {
      +CollapseStatus status
      +Option~RuleId~ fired
      +MycinByte detail
      +InstinctByte emits
      +FieldMask support
    }

    class RuleMycin {
      +rules: slice
      +evaluate(ctx) RuleClosureResult
    }

    RuleMycin --> ExpertRule
    RuleMycin --> RuleClosureResult
    RuleClosureResult --> MycinByte
```

---

## 6. Failure taxonomy

```mermaid
mindmap
  root((Rule / MYCIN Failures))
    RuleConflict
      Multiple high-confidence rules match
      Requires manual disambiguation or stricter logic
    PolicyEpochStale
      Rule matches but policy is out of date
      Action is blocked pending policy refresh
    ConfidenceLow
      Rule matches but certainty is below threshold
      Triggers ASK or ESCALATE
    ExpertReviewRequired
      Hardcoded rule bounds exceeded
      Andon event triggered
```

---

## 7. Reference vs fast-path admission

```mermaid
flowchart TD
    Fixture["Canonical rule fixture<br/>rules + ctx state"]
    Ref["ReferenceMycinPath<br/>scalar missing/forbidden XOR checks"]
    Simd["SIMD rule batch path<br/>eval multiple rules in parallel"]
    Unsafe["unsafe-admitted path<br/>elided bounds checking"]

    Compare["Compare RuleClosureResult<br/>fired rule, MYCIN8, INST8, support"]
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

MYCIN evaluates the specific enterprise security policies (e.g., "Terminated users must lose physical access immediately").

```mermaid
flowchart TD
    State["Fused AccessDrift O*<br/>terminated + badge_active + site_entry"]
    Rule1["Rule: ImmediatePhysicalRevoke<br/>required: terminated, badge_active<br/>emits: REFUSE, ESCALATE<br/>certainty: High"]
    
    Mycin["Rule / MYCIN"]
    
    Status["MYCIN8: RuleMatched + RuleFired + ConfidenceHigh + PolicyEpochValid"]
    Instinct["INST8: Refuse + Escalate"]
    Motion["POWL8: Block Access / Escalate to Security"]

    State --> Mycin
    Rule1 --> Mycin
    Mycin --> Status
    Status --> Instinct
    Instinct --> Motion
```
