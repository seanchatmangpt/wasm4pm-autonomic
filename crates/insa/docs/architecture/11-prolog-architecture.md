# KAPPA Template 03: Prove / Prolog

Core meaning:
**Prove = bounded relational proof over grounded facts, rules, authority, ownership, and policy relations.**

This comes after Ground / SHRDLU and Precondition / STRIPS because once objects are grounded and action schemas are known, the system must prove relations like:
* person owns account
* vendor contract governs identity
* badge belongs to contractor
* repo access is active
* policy applies to this site

---

## 1. Role in the INSA pipeline

```mermaid
flowchart TD
    OStar["O*<br/>closed field context"]
    Grounded["Grounded objects<br/>Person, Vendor, Badge, Account, Site, Device"]
    Preconditions["STRIPS action schemas<br/>required / forbidden / effects"]
    Goal["Proof Goal<br/>can_access, owns, assigned_to, authorized, governed_by"]
    Prolog["Prove / Prolog<br/>bounded Horn-clause proof"]
    Kappa["KAPPA8 bit 3<br/>Prove"]
    PROLOG8["PROLOG8<br/>proof detail byte"]
    Instinct["INST8<br/>Settle / Retrieve / Ask / Await / Refuse / Escalate / Inspect"]
    Resolution["InstinctResolution"]
    POWL8["POWL8 motion"]
    Proof["POWL64<br/>route proof witness"]

    OStar --> Prolog
    Grounded --> Prolog
    Preconditions --> Prolog
    Goal --> Prolog

    Prolog --> Kappa
    Prolog --> PROLOG8
    Prolog --> Instinct

    Kappa --> Resolution
    PROLOG8 --> Resolution
    Instinct --> Resolution
    Resolution --> POWL8
    POWL8 --> Proof
```

---

## 2. Internal 8-bit architecture: PROLOG8

```mermaid
flowchart LR
    Byte["PROLOG8 u8"]

    B0["bit 0<br/>GoalProved"]
    B1["bit 1<br/>GoalFailed"]
    B2["bit 2<br/>FactMissing"]
    B3["bit 3<br/>RuleMatched"]
    B4["bit 4<br/>ContradictionFound"]
    B5["bit 5<br/>DepthExhausted"]
    B6["bit 6<br/>CycleDetected"]
    B7["bit 7<br/>ProofRequiresEscalation"]

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
* GoalProved means the proof closed under admitted facts/rules.
* GoalFailed means the goal was refuted or could not be satisfied under admitted closure.
* FactMissing means proof cannot proceed without exact missing evidence.
* ContradictionFound means admitted facts conflict.
* DepthExhausted means proof budget was exceeded.
* CycleDetected means recursive proof attempted unsafe repetition.
* ProofRequiresEscalation means local proof authority is insufficient.

---

## 3. Rust module/component diagram

```mermaid
flowchart TD
    Crate["insa-kappa8::prove_prolog"]

    Terms["terms.rs<br/>TermId, VarId, ConstantId"]
    Relations["relations.rs<br/>RelationId, PredicateKind"]
    Facts["facts.rs<br/>FactTable, FactRow, validity"]
    Rules["rules.rs<br/>HornClause, ClauseBody, RuleBudget"]
    Agenda["agenda.rs<br/>fixed-cap proof agenda"]
    Engine["engine.rs<br/>prove(), unify(), step()"]
    Byte["byte.rs<br/>PrologByte bit ops"]
    Result["result.rs<br/>ProofResult, ProofStatus"]
    Witness["witness.rs<br/>ProofWitness, used facts/rules"]
    Tests["tests/<br/>unit, prop, compile-fail, JTBD"]

    Crate --> Terms
    Crate --> Relations
    Crate --> Facts
    Crate --> Rules
    Crate --> Agenda
    Crate --> Engine
    Crate --> Byte
    Crate --> Result
    Crate --> Witness
    Crate --> Tests

    Terms --> Facts
    Relations --> Facts
    Relations --> Rules
    Facts --> Engine
    Rules --> Engine
    Agenda --> Engine
    Engine --> Byte
    Engine --> Result
    Engine --> Witness
```

---

## 4. Execution flow / sequence

```mermaid
sequenceDiagram
    participant Caller as COG8 / STRIPS / Security Closure
    participant Prolog as ProveProlog
    participant Facts as FactTable
    participant Rules as HornRuleTable
    participant Agenda as ProofAgenda
    participant Witness as ProofWitness
    participant Result as ProofResult

    Caller->>Prolog: prove(goal, ctx)
    Prolog->>Facts: lookup matching facts
    Facts-->>Prolog: fact candidates
    Prolog->>Rules: lookup matching clauses
    Rules-->>Prolog: rule candidates
    Prolog->>Agenda: push bounded subgoals
    loop bounded proof steps
        Prolog->>Agenda: pop next goal
        Prolog->>Facts: attempt fact match
        Prolog->>Rules: attempt rule expansion
        Prolog->>Prolog: detect contradiction / cycle / depth exhaustion
    end
    Prolog->>Witness: record used facts/rules/support
    Prolog-->>Result: PROLOG8 + KAPPA8 + INST8 + witness
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid
classDiagram
    class RelationId {
      +u16 raw
    }

    class TermId {
      +u32 raw
    }

    class FactRow {
      +RelationId relation
      +TermId subject
      +TermId object
      +Validity validity
      +SourceId source
      +PolicyEpoch epoch
    }

    class HornClause {
      +ClauseId id
      +RelationId head
      +SmallBody body
      +ProofBudget budget
      +PolicyEpoch epoch
    }

    class ProofGoal {
      +RelationId relation
      +TermId subject
      +TermId object
    }

    class ProofAgenda {
      +FixedCapStack goals
      +Depth depth
      +VisitedSet visited
    }

    class PrologByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class ProofResult {
      +ProofStatus status
      +PrologByte detail
      +KappaByte kappa
      +InstinctByte emits
      +FieldMask support
      +ProofWitness witness
    }

    class ProveProlog {
      +prove(goal, ctx) ProofResult
      +step(agenda, facts, rules) ProofStep
    }

    ProveProlog --> ProofGoal
    ProveProlog --> FactRow
    ProveProlog --> HornClause
    ProveProlog --> ProofAgenda
    ProveProlog --> ProofResult
```

---

## 6. Failure taxonomy

```mermaid
mindmap
  root((Prove / Prolog Failures))
    FactMissing
      missing ownership fact
      missing assignment fact
      missing authorization fact
      stale source not admitted
    GoalFailed
      relation absent
      relation explicitly false
      required proof not derivable
    ContradictionFound
      HR says terminated
      IAM says active employee
      vendor contract expired but exception says valid
      two owners conflict
    DepthExhausted
      recursive proof exceeds budget
      dependency chain too deep
      proof requires decomposition
    CycleDetected
      role inherits itself
      group nesting cycle
      policy dependency cycle
    RuleFailure
      no applicable clause
      stale policy epoch
      invalid clause body
    AuthorityFailure
      source cannot prove relation
      wrong system of authority
      delegated authority expired
    EscalationRequired
      proof local to runtime insufficient
      human/legal/security owner required
```

Core rule:
**Proof failure must produce an exact reason, not an empty “false.”**

---

## 7. Reference vs fast-path admission

```mermaid
flowchart TD
    Fixture["Canonical proof fixture<br/>facts + rules + goal + expected result"]
    Ref["ReferenceProofPath<br/>clear bounded proof"]
    Indexed["Indexed fact/rule path<br/>relation-indexed tables"]
    Table["Precomputed relation table<br/>if admissible"]
    SIMD["SIMD candidate<br/>batch relation matching"]
    Intrinsic["Intrinsic candidate<br/>bitset/popcnt acceleration"]
    Unsafe["unsafe-admitted arena path<br/>only if proven"]

    Compare["Compare ProofResult<br/>PROLOG8, status, witness, support, emitted INST8"]
    Replay["Replay proof witness<br/>facts/rules still derive result"]
    Admit{"Equivalent + evidenced?"}
    Good["Admit fast proof path"]
    Bad["Reject / classify failure"]

    Fixture --> Ref
    Fixture --> Indexed
    Fixture --> Table
    Fixture --> SIMD
    Fixture --> Intrinsic
    Fixture --> Unsafe

    Ref --> Compare
    Indexed --> Compare
    Table --> Compare
    SIMD --> Compare
    Intrinsic --> Compare
    Unsafe --> Compare

    Compare --> Replay
    Replay --> Admit
    Admit -- yes --> Good
    Admit -- no --> Bad
```

Admission law:
**A faster proof path is real only if it yields the same proof result and replay witness as ReferenceProofPath.**

---

## 8. JTBD instantiation: Access Drift case

Case:
terminated contractor still has active badge, VPN, repo access, vendor relationship, and recent site/device activity.

Ground / SHRDLU binds the objects.
STRIPS / Precondition determines which actions are enabled or blocked.
Prolog / Prove proves the relational facts:
* contractor belongs to vendor
* badge belongs to contractor
* vpn account belongs to contractor
* repo account belongs to contractor
* vendor contract is expired
* identity is terminated
* access is still active
* policy requires removal

```mermaid
flowchart TD
    Objects["Grounded objects<br/>Person, Vendor, Badge, VPN, Repo, Site, Device"]
    Facts["Admitted facts<br/>belongs_to, owns, has_access, expired, terminated"]
    Rules["Horn rules<br/>active_access_after_termination<br/>vendor_access_invalid<br/>access_removal_required"]
    Goals["Proof goals<br/>is_access_drift(person)<br/>must_revoke(access)<br/>cannot_allow_access(person)"]

    Prolog["Prove / Prolog"]
    Result["PROLOG8<br/>GoalProved + RuleMatched"]
    Kappa["KAPPA8<br/>Prove"]
    Instinct["INST8<br/>Refuse + Escalate + Retrieve"]
    Motion["POWL8<br/>BLOCK allow access<br/>ACT revoke<br/>ESCALATE owner"]
    Evidence["POWL64<br/>proof witness + blocked alternative"]

    Objects --> Facts
    Facts --> Prolog
    Rules --> Prolog
    Goals --> Prolog

    Prolog --> Result
    Result --> Kappa
    Result --> Instinct
    Instinct --> Motion
    Motion --> Evidence
```

---

# 9. Access Drift proof rules

```mermaid
flowchart TD
    F1["fact: terminated(Person)"]
    F2["fact: belongs_to(Badge, Person)"]
    F3["fact: active(Badge)"]
    F4["fact: belongs_to(VpnAccount, Person)"]
    F5["fact: active(VpnAccount)"]
    F6["fact: belongs_to(RepoAccount, Person)"]
    F7["fact: active(RepoAccount)"]
    F8["fact: vendor_expired(Vendor)"]
    F9["fact: contracted_through(Person, Vendor)"]

    R1["rule: active_access_after_termination(Person)<br/>terminated(Person) + belongs_to(Access, Person) + active(Access)"]
    R2["rule: vendor_access_invalid(Person)<br/>contracted_through(Person, Vendor) + vendor_expired(Vendor)"]
    R3["rule: cannot_allow_access(Person)<br/>active_access_after_termination(Person) OR vendor_access_invalid(Person)"]
    R4["rule: must_revoke(Access)<br/>belongs_to(Access, Person) + terminated(Person) + active(Access)"]

    Goal["goal: cannot_allow_access(Person)"]
    Proof["ProofResult: GoalProved"]

    F1 --> R1
    F2 --> R1
    F3 --> R1
    F4 --> R1
    F5 --> R1
    F6 --> R1
    F7 --> R1

    F8 --> R2
    F9 --> R2

    R1 --> R3
    R2 --> R3
    R3 --> Goal
    Goal --> Proof

    F1 --> R4
    F2 --> R4
    F3 --> R4
```

---

# 10. PROLOG8 → INST8 mapping

```mermaid
flowchart LR
    PROLOG8["PROLOG8"]

    Proved["GoalProved"]
    Failed["GoalFailed"]
    Missing["FactMissing"]
    RuleMatched["RuleMatched"]
    Contradiction["ContradictionFound"]
    Exhausted["DepthExhausted"]
    Cycle["CycleDetected"]
    Escalation["ProofRequiresEscalation"]

    Settle["INST8: Settle"]
    Refuse["INST8: Refuse"]
    Retrieve["INST8: Retrieve"]
    Ask["INST8: Ask"]
    Inspect["INST8: Inspect"]
    Escalate["INST8: Escalate"]
    Await["INST8: Await"]

    PROLOG8 --> Proved
    PROLOG8 --> Failed
    PROLOG8 --> Missing
    PROLOG8 --> RuleMatched
    PROLOG8 --> Contradiction
    PROLOG8 --> Exhausted
    PROLOG8 --> Cycle
    PROLOG8 --> Escalation

    Proved --> Settle
    Proved --> Refuse
    Missing --> Retrieve
    Missing --> Ask
    Failed --> Refuse
    Contradiction --> Inspect
    Contradiction --> Escalate
    Exhausted --> Escalate
    Cycle --> Inspect
    Escalation --> Escalate
    Missing --> Await
```

Mapping rule:
GoalProved -> Settle or Refuse depending on proven goal
FactMissing -> Retrieve / Ask / Await
ContradictionFound -> Inspect / Escalate
DepthExhausted -> Escalate / decompose
CycleDetected -> Inspect / reject rule graph
ProofRequiresEscalation -> Escalate

---

# 11. Prolog boundedness gates

```mermaid
flowchart TD
    Goal["Proof goal candidate"]
    ArityCheck{"arity admitted?"}
    FactCheck{"facts admitted?"}
    RuleCheck{"rules epoch-valid?"}
    DepthCheck{"depth <= budget?"}
    CycleCheck{"no cycle?"}
    SupportCheck{"support <= 8 or decomposed?"}
    Admit["Proof admitted"]
    BadArity["Reject<br/>unsupported relation arity"]
    MissingFact["Retrieve / Ask<br/>missing fact"]
    StaleRule["Await / Refuse<br/>stale rule epoch"]
    Exhausted["Escalate<br/>depth exhausted"]
    Cycle["Inspect<br/>cycle detected"]
    Need9["ANDON: Need9<br/>decompose proof support"]

    Goal --> ArityCheck
    ArityCheck -- no --> BadArity
    ArityCheck -- yes --> FactCheck

    FactCheck -- no --> MissingFact
    FactCheck -- yes --> RuleCheck

    RuleCheck -- no --> StaleRule
    RuleCheck -- yes --> DepthCheck

    DepthCheck -- no --> Exhausted
    DepthCheck -- yes --> CycleCheck

    CycleCheck -- no --> Cycle
    CycleCheck -- yes --> SupportCheck

    SupportCheck -- no --> Need9
    SupportCheck -- yes --> Admit
```

This prevents Prolog from becoming unbounded symbolic search.

---

# 12. Proof witness to POWL64

```mermaid
flowchart TD
    ProofResult["ProofResult"]
    UsedFacts["UsedFactRefs[]"]
    UsedRules["UsedRuleRefs[]"]
    Goal["ProofGoal"]
    Support["FieldMask support"]
    Status["PROLOG8 status bits"]
    Witness["ProofWitness"]
    RouteCell["POWL64 RouteCell"]
    Blocked["BlockedAlternative<br/>if proof blocks action"]
    Replay["Replay verifies proof"]

    ProofResult --> UsedFacts
    ProofResult --> UsedRules
    ProofResult --> Goal
    ProofResult --> Support
    ProofResult --> Status

    UsedFacts --> Witness
    UsedRules --> Witness
    Goal --> Witness
    Support --> Witness
    Status --> Witness

    Witness --> RouteCell
    Witness --> Blocked
    RouteCell --> Replay
    Blocked --> Replay
```
