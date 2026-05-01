# KAPPA Template 01: Ground / SHRDLU

Core meaning:
Ground = bind symbols/references to the correct enterprise objects under context.
Without grounding, nothing else is lawful.

---

## 1. Role in the INSA pipeline

```mermaid
flowchart TD
    OStar["O*<br/>closed field context"]
    Symbols["Unbound references<br/>person, badge, vendor, site, app, account"]
    Ground["Ground / SHRDLU<br/>resolve reference to object"]
    Kappa["KAPPA8 bit 2<br/>Ground"]
    Ground8["SHRDLU8<br/>grounding detail byte"]
    Instinct["INST8<br/>Retrieve / Ask / Inspect / Refuse"]
    Resolution["InstinctResolution"]
    POWL8["POWL8 motion"]

    OStar --> Ground
    Symbols --> Ground
    Ground --> Kappa
    Ground --> Ground8
    Ground --> Instinct
    Kappa --> Resolution
    Ground8 --> Resolution
    Instinct --> Resolution
    Resolution --> POWL8
```

---

## 2. Internal 8-bit architecture: SHRDLU8

```mermaid
flowchart LR
    Byte["SHRDLU8 u8"]

    B0["bit 0<br/>SymbolResolved"]
    B1["bit 1<br/>ObjectUnique"]
    B2["bit 2<br/>AliasMatched"]
    B3["bit 3<br/>ContextDisambiguated"]
    B4["bit 4<br/>AmbiguousReference"]
    B5["bit 5<br/>MissingObject"]
    B6["bit 6<br/>AuthorityMismatch"]
    B7["bit 7<br/>GroundingFailed"]

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
* success-like bits: SymbolResolved, ObjectUnique, AliasMatched, ContextDisambiguated
* failure-like bits: AmbiguousReference, MissingObject, AuthorityMismatch, GroundingFailed

---

## 3. Rust module/component diagram

```mermaid
flowchart TD
    Crate["insa-kappa8::ground_shrdlu"]

    Domain["domain.rs<br/>SymbolId, ObjectRef, AliasEntry"]
    Dict["dictionary.rs<br/>alias tables, ontology mapping"]
    Ctx["context.rs<br/>GroundingCtx, policy, source authority"]
    Resolve["resolver.rs<br/>resolve(), disambiguate(), authority checks"]
    Byte["byte.rs<br/>ShrdluByte bit ops"]
    Result["result.rs<br/>GroundingResult, status, support"]
    Fixture["fixtures.rs<br/>canonical grounding cases"]
    Tests["tests/<br/>unit, prop, compile-fail, JTBD"]

    Crate --> Domain
    Crate --> Dict
    Crate --> Ctx
    Crate --> Resolve
    Crate --> Byte
    Crate --> Result
    Crate --> Fixture
    Crate --> Tests

    Domain --> Dict
    Dict --> Resolve
    Ctx --> Resolve
    Resolve --> Byte
    Resolve --> Result
    Fixture --> Tests
```

---

## 4. Execution flow / sequence

```mermaid
sequenceDiagram
    participant Caller as COG8 / Security Closure
    participant Ground as GroundShrdlu
    participant Dict as AliasDictionary
    participant Ctx as GroundingCtx
    participant Result as GroundingResult

    Caller->>Ground: ground(symbol, ctx)
    Ground->>Dict: lookup(symbol)
    Dict-->>Ground: candidate ObjectRefs
    Ground->>Ctx: apply context filters
    Ctx-->>Ground: allowed candidates
    Ground->>Ground: authority/disambiguation checks
    Ground-->>Result: SHRDLU8 + ObjectRef? + support + INST8
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid
classDiagram
    class SymbolId {
      +u32 raw
    }

    class ObjectRef {
      +ObjectKind kind
      +u64 id
    }

    class AliasEntry {
      +SymbolId symbol
      +ObjectRef object
      +AuthorityClass authority
      +PolicyEpoch epoch
    }

    class GroundingCtx {
      +FieldMask present
      +SourceClass source
      +PolicyEpoch policy_epoch
      +AuthorityMask allowed_authority
      +ObjectScope scope
    }

    class ShrdluByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class GroundingResult {
      +GroundingStatus status
      +ObjectRef object
      +ShrdluByte detail
      +InstinctByte emits
      +FieldMask support
    }

    class GroundShrdlu {
      +ground(symbol, ctx) GroundingResult
      +disambiguate(candidates, ctx) GroundingResult
    }

    GroundShrdlu --> AliasEntry
    GroundShrdlu --> GroundingCtx
    GroundShrdlu --> ShrdluByte
    GroundShrdlu --> GroundingResult
```

---

## 6. Failure taxonomy

```mermaid
mindmap
  root((Ground / SHRDLU Failures))
    MissingObject
      symbol not in dictionary
      object deleted or stale
      wrong namespace
    AmbiguousReference
      many candidate people
      many candidate sites
      badge shared label collision
    AuthorityMismatch
      source not authoritative
      policy scope excludes candidate
      stale vendor/HR authority
    ContextFailure
      wrong site
      wrong time window
      wrong business unit
      wrong tenant
    AliasFailure
      alias drift
      outdated synonym
      malformed import
    GroundingFailed
      no admissible object
      contradictions remain
```

---

## 7. Reference vs fast-path admission

```mermaid
flowchart TD
    Fixture["Canonical grounding fixture<br/>symbol + ctx + expected object"]
    Ref["ReferenceGroundPath<br/>clear, simple, exact"]
    Table["Alias table path<br/>stable lookup"]
    Simd["SIMD candidate filter path<br/>if admitted"]
    Intrinsic["target intrinsic path<br/>if admitted"]
    Unsafe["unsafe-admitted path<br/>only if proven"]

    Compare["Compare GroundingResult<br/>status, object, SHDRLU8, INST8, support"]
    Admit{"Equivalent?"}
    Good["Admit fast path"]
    Bad["Reject path"]

    Fixture --> Ref
    Fixture --> Table
    Fixture --> Simd
    Fixture --> Intrinsic
    Fixture --> Unsafe

    Ref --> Compare
    Table --> Compare
    Simd --> Compare
    Intrinsic --> Compare
    Unsafe --> Compare

    Compare --> Admit
    Admit -- yes --> Good
    Admit -- no --> Bad
```

**Rule:**
No fast grounding path without equivalence to the reference grounding law.

---

## 8. JTBD instantiation: Access Drift case

Case:
terminated contractor still has active badge, VPN, repo access, vendor relationship, and recent site/device activity

```mermaid
flowchart TD
    Raw["Raw references<br/>contractor name, badge ID, VPN account, repo user, vendor code, site event"]
    Ground["Ground / SHRDLU"]
    Person["Grounded Person Object"]
    Vendor["Grounded Vendor Object"]
    Badge["Grounded Badge Object"]
    Account["Grounded Digital Account Objects"]
    Site["Grounded Site Object"]

    Fuse["Fuse / HEARSAY-II<br/>do these grounded objects cohere?"]
    Rule["Rule / MYCIN<br/>policy says terminated/vendor-expired access must be removed"]
    Prove["Prove / Prolog<br/>prove active access still exists"]
    Instinct["INST8<br/>Refuse + Escalate + Retrieve"]
    Motion["POWL8<br/>block access / escalate / ask for exact missing item"]

    Raw --> Ground
    Ground --> Person
    Ground --> Vendor
    Ground --> Badge
    Ground --> Account
    Ground --> Site

    Person --> Fuse
    Vendor --> Fuse
    Badge --> Fuse
    Account --> Fuse
    Site --> Fuse

    Fuse --> Rule
    Rule --> Prove
    Prove --> Instinct
    Instinct --> Motion
```
