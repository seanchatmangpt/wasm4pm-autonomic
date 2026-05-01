Next KAPPA pack:

[
\boxed{\text{Reconstruct / DENDRAL}}
]

Core meaning:

[
\boxed{
\text{Reconstruct} = \text{derive the most lawful bounded structure from fragments without hallucinating missing links.}
}
]

This is the layer for:

```text id="e61xk2"
timeline reconstruction
incident path reconstruction
dependency chain reconstruction
access path reconstruction
document bundle reconstruction
evidence chain reconstruction
CVE reachability path reconstruction
```

`DENDRAL / Reconstruct` is not “creative inference.” It is **bounded structural reconstruction** under constraints.

---

# KAPPA Template 07: `Reconstruct / DENDRAL`

## 1. Role in the INSA pipeline

```mermaid id="xz0y4v"
flowchart TD
    OStar["O*<br/>closed or partially closed field context"]
    Fragments["Fragments<br/>logs, badge events, repo events, IAM events, vendor records"]
    Constraints["Constraints<br/>time, identity, policy, topology, authority"]
    DENDRAL["Reconstruct / DENDRAL<br/>bounded candidate reconstruction"]
    Kappa["KAPPA8 bit 5<br/>Reconstruct"]
    DENDRAL8["DENDRAL8<br/>reconstruction detail byte"]
    Instinct["INST8<br/>Retrieve / Inspect / Ask / Await / Refuse / Escalate / Settle"]
    Resolution["InstinctResolution"]
    POWL8["POWL8 motion"]
    Proof["POWL64<br/>reconstruction witness + route proof"]

    OStar --> DENDRAL
    Fragments --> DENDRAL
    Constraints --> DENDRAL

    DENDRAL --> Kappa
    DENDRAL --> DENDRAL8
    DENDRAL --> Instinct

    Kappa --> Resolution
    DENDRAL8 --> Resolution
    Instinct --> Resolution
    Resolution --> POWL8
    POWL8 --> Proof
```

---

## 2. Internal 8-bit architecture: `DENDRAL8`

```mermaid id="m0p0zu"
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

[
UniqueReconstruction \Rightarrow FragmentsSufficient \land CandidateGenerated \land CandidatePruned \land \neg ConstraintViolation
]

[
ReconstructionUnstable \Rightarrow MultipleReconstructions \lor MissingFragment \lor ConstraintViolation
]

---

## 3. Rust module/component diagram

```mermaid id="leg0b5"
flowchart TD
    Crate["insa-kappa8::reconstruct_dendral"]

    Fragment["fragment.rs<br/>Fragment, FragmentKind, FragmentDigest"]
    Constraint["constraint.rs<br/>time, object, policy, topology constraints"]
    Candidate["candidate.rs<br/>ReconstructionCandidate, CandidateArena"]
    Generate["generate.rs<br/>bounded candidate generation"]
    Prune["prune.rs<br/>constraint pruning"]
    Rank["rank.rs<br/>deterministic ranking / tie policy"]
    Byte["byte.rs<br/>DendralByte bit ops"]
    Result["result.rs<br/>ReconstructionResult"]
    Witness["witness.rs<br/>ReconstructionWitness"]
    Tests["tests/<br/>unit, prop, compile-fail, JTBD"]

    Crate --> Fragment
    Crate --> Constraint
    Crate --> Candidate
    Crate --> Generate
    Crate --> Prune
    Crate --> Rank
    Crate --> Byte
    Crate --> Result
    Crate --> Witness
    Crate --> Tests

    Fragment --> Generate
    Constraint --> Generate
    Generate --> Candidate
    Candidate --> Prune
    Constraint --> Prune
    Prune --> Rank
    Rank --> Byte
    Rank --> Result
    Result --> Witness
```

---

## 4. Execution flow / sequence

```mermaid id="ylp7z1"
sequenceDiagram
    participant Caller as Security Closure / COG8
    participant Dendral as ReconstructDendral
    participant Frags as FragmentSet
    participant Cons as ConstraintSet
    participant Arena as CandidateArena
    participant Witness as ReconstructionWitness
    participant Result as ReconstructionResult

    Caller->>Dendral: reconstruct(fragments, constraints, ctx)
    Dendral->>Frags: load bounded fragments
    Frags-->>Dendral: fragment set
    Dendral->>Cons: load constraints
    Cons-->>Dendral: time/object/policy/topology constraints
    Dendral->>Arena: generate candidates within budget
    Arena-->>Dendral: candidate structures
    Dendral->>Dendral: prune candidates violating constraints
    Dendral->>Dendral: rank remaining candidates deterministically
    Dendral->>Witness: record fragments, constraints, candidates, prunes
    Dendral-->>Result: DENDRAL8 + KAPPA8 + INST8 + witness
    Result-->>Caller: CollapseResult
```

---

## 5. Type / data model

```mermaid id="b2qeaa"
classDiagram
    class Fragment {
      +FragmentId id
      +FragmentKind kind
      +ObjectRef object
      +TimeRange time
      +FieldMask asserts
      +DigestRef digest
      +SourceId source
    }

    class ReconstructionConstraint {
      +ConstraintId id
      +ConstraintKind kind
      +FieldMask required
      +FieldMask forbidden
      +TimeRange valid_time
      +PolicyEpoch epoch
    }

    class ReconstructionCandidate {
      +CandidateId id
      +FieldMask support
      +FieldMask inferred
      +ConstraintMask satisfied
      +ConstraintMask violated
      +RankScore score
    }

    class CandidateArena {
      +FixedCapArray candidates
      +u8 len
      +CandidateBudget budget
    }

    class DendralByte {
      +u8 bits
      +contains(bit)
      +set(bit)
    }

    class ReconstructionResult {
      +ReconstructionStatus status
      +DendralByte detail
      +KappaByte kappa
      +InstinctByte emits
      +CandidateId selected
      +FieldMask support
      +ReconstructionWitnessId witness
    }

    class ReconstructDendral {
      +reconstruct(fragments, constraints, ctx) ReconstructionResult
      +generate_candidates()
      +prune_candidates()
      +rank_candidates()
    }

    ReconstructDendral --> Fragment
    ReconstructDendral --> ReconstructionConstraint
    ReconstructDendral --> CandidateArena
    ReconstructDendral --> ReconstructionResult
    CandidateArena --> ReconstructionCandidate
```

---

## 6. Failure taxonomy

```mermaid id="z42k2k"
mindmap
  root((Reconstruct / DENDRAL Failures))
    MissingFragment
      missing log segment
      missing badge event
      missing IAM event
      missing repo audit event
      missing policy exception
    ConstraintViolation
      impossible timestamp order
      identity mismatch
      site mismatch
      policy contradiction
      topology contradiction
    MultipleReconstructions
      several plausible timelines
      several possible account owners
      several possible access paths
      ambiguous source order
    ReconstructionUnstable
      result changes with fragment order
      tie not deterministically resolved
      evidence insufficient for selected path
    CandidateExplosion
      too many candidates
      unbounded combinatorics
      reconstruction budget exhausted
    FragmentStale
      stale source snapshot
      old policy epoch
      delayed import
    HallucinatedLink
      inferred relation lacks support
      candidate uses non-admitted edge
      missing digest evidence
```

Hard law:

[
\boxed{
DENDRAL may reconstruct structure, but it may not invent authority.
}
]

Missing links become `Retrieve`, `Ask`, `Await`, or `Inspect`, not hallucinated closure.

---

## 7. Reference vs fast-path admission

```mermaid id="hufwve"
flowchart TD
    Fixture["Canonical reconstruction fixture<br/>fragments + constraints + expected result"]
    Ref["ReferenceReconstructionPath<br/>clear lawful generation/pruning"]
    Table["Constraint table path"]
    Arena["Fixed arena path"]
    SIMD["SIMD candidate pruning<br/>if admitted"]
    Intrinsic["Intrinsic path<br/>bitset/popcnt acceleration"]
    Unsafe["unsafe-admitted path<br/>arena/index optimization"]

    Compare["Compare ReconstructionResult<br/>DENDRAL8, selected candidate, support, emitted INST8"]
    WitnessCompare["Compare ReconstructionWitness<br/>fragments, constraints, pruned candidates"]
    Replay["Replay reconstruction witness"]
    Admit{"Equivalent + evidenced?"}
    Good["Admit fast reconstruction path"]
    Bad["Reject / classify failure"]

    Fixture --> Ref
    Fixture --> Table
    Fixture --> Arena
    Fixture --> SIMD
    Fixture --> Intrinsic
    Fixture --> Unsafe

    Ref --> Compare
    Table --> Compare
    Arena --> Compare
    SIMD --> Compare
    Intrinsic --> Compare
    Unsafe --> Compare

    Compare --> WitnessCompare
    WitnessCompare --> Replay
    Replay --> Admit

    Admit -- yes --> Good
    Admit -- no --> Bad
```

Admission law:

[
\boxed{
A faster reconstruction path is admitted only if it selects the same candidate, prunes the same invalid structures, and produces the same replay witness as ReferenceReconstructionPath.
}
]

---

## 8. JTBD instantiation: Access Drift case

Case:

```text id="f20znp"
Terminated contractor still has active badge, VPN, repo access, expired vendor relationship, and recent site/device activity.
```

`DENDRAL / Reconstruct` answers:

```text id="4f36dg"
What actually happened in sequence?
Which events form the access-drift path?
Which fragments are missing?
Is there one stable timeline or several possible reconstructions?
```

```mermaid id="xoy2ry"
flowchart TD
    F1["Fragment: HR termination event"]
    F2["Fragment: vendor contract expired"]
    F3["Fragment: badge active"]
    F4["Fragment: badge used after hours"]
    F5["Fragment: VPN login active"]
    F6["Fragment: repo access active"]
    F7["Fragment: device seen on site network"]
    F8["Fragment: policy requires removal"]

    Constraints["Constraints<br/>time order, same person, same vendor, same site, policy epoch"]
    DENDRAL["Reconstruct / DENDRAL"]

    Candidate1["Candidate timeline A<br/>termination -> access not removed -> badge/site/network activity"]
    Candidate2["Candidate timeline B<br/>identity mismatch or stale source path"]
    Prune["Prune impossible / unsupported candidates"]
    Selected["Selected reconstruction<br/>active access drift after termination"]
    Instinct["INST8<br/>Inspect + Retrieve + Escalate / Refuse"]
    Proof["POWL64<br/>ReconstructionWitness"]

    F1 --> DENDRAL
    F2 --> DENDRAL
    F3 --> DENDRAL
    F4 --> DENDRAL
    F5 --> DENDRAL
    F6 --> DENDRAL
    F7 --> DENDRAL
    F8 --> DENDRAL
    Constraints --> DENDRAL

    DENDRAL --> Candidate1
    DENDRAL --> Candidate2
    Candidate1 --> Prune
    Candidate2 --> Prune
    Prune --> Selected
    Selected --> Instinct
    Instinct --> Proof
```

---

# 9. Reconstruction candidates for Access Drift

```mermaid id="t7uqta"
flowchart TD
    subgraph C1["Candidate A: True Access Drift"]
        C1A["same contractor"]
        C1B["terminated before access events"]
        C1C["vendor expired"]
        C1D["badge/VPN/repo remained active"]
        C1E["device/site evidence supports overlap"]
    end

    subgraph C2["Candidate B: Stale Source Artifact"]
        C2A["HR terminated"]
        C2B["IAM/badge export stale"]
        C2C["no confirmed post-termination activity"]
    end

    subgraph C3["Candidate C: Identity Collision"]
        C3A["same name"]
        C3B["different account/person"]
        C3C["badge/account mismatch"]
    end

    Constraints["Constraint Pruning<br/>identity, time, authority, freshness"]
    Selected["Selected or Unstable Reconstruction"]

    C1 --> Constraints
    C2 --> Constraints
    C3 --> Constraints
    Constraints --> Selected
```

This is important because INSA should not accuse from fragments. It should either select a replayable reconstruction or mark the reconstruction unstable.

---

# 10. DENDRAL8 → INST8 mapping

```mermaid id="k5s2h6"
flowchart LR
    DENDRAL8["DENDRAL8"]

    Sufficient["FragmentsSufficient"]
    Generated["CandidateGenerated"]
    Pruned["CandidatePruned"]
    Unique["UniqueReconstruction"]
    Multiple["MultipleReconstructions"]
    Missing["MissingFragment"]
    Violation["ConstraintViolation"]
    Unstable["ReconstructionUnstable"]

    Settle["INST8: Settle"]
    Retrieve["INST8: Retrieve"]
    Ask["INST8: Ask"]
    Await["INST8: Await"]
    Inspect["INST8: Inspect"]
    Refuse["INST8: Refuse"]
    Escalate["INST8: Escalate"]

    DENDRAL8 --> Sufficient
    DENDRAL8 --> Generated
    DENDRAL8 --> Pruned
    DENDRAL8 --> Unique
    DENDRAL8 --> Multiple
    DENDRAL8 --> Missing
    DENDRAL8 --> Violation
    DENDRAL8 --> Unstable

    Unique --> Settle
    Unique --> Refuse
    Multiple --> Inspect
    Multiple --> Escalate
    Missing --> Retrieve
    Missing --> Ask
    Missing --> Await
    Violation --> Inspect
    Violation --> Refuse
    Unstable --> Inspect
    Unstable --> Escalate
```

Mapping rule:

```text id="aznsbn"
UniqueReconstruction -> Settle or proceed to Refuse/Escalate depending on content
MultipleReconstructions -> Inspect/Escalate
MissingFragment -> Retrieve/Ask/Await
ConstraintViolation -> Inspect/Refuse
ReconstructionUnstable -> Inspect/Escalate
```

---

# 11. Reconstruction boundedness gates

```mermaid id="gs8vu4"
flowchart TD
    Fragments["Fragment set"]
    FragmentCheck{"fragment count <= budget?"}
    ConstraintCheck{"constraints admitted?"}
    CandidateCheck{"candidate count <= budget?"}
    SupportCheck{"support <= 8 or decomposed?"}
    StabilityCheck{"deterministic candidate selection?"}
    AuthorityCheck{"no inferred authority without support?"}
    Admit["Reconstruction admitted"]

    TooManyFragments["ANDON<br/>split fragment set"]
    BadConstraint["Reject / Retrieve<br/>constraint missing/stale"]
    Explosion["ANDON<br/>candidate explosion"]
    Need9["ANDON: Need9<br/>split reconstruction"]
    Unstable["Inspect<br/>unstable reconstruction"]
    Hallucination["Reject<br/>hallucinated relation"]

    Fragments --> FragmentCheck
    FragmentCheck -- no --> TooManyFragments
    FragmentCheck -- yes --> ConstraintCheck

    ConstraintCheck -- no --> BadConstraint
    ConstraintCheck -- yes --> CandidateCheck

    CandidateCheck -- no --> Explosion
    CandidateCheck -- yes --> SupportCheck

    SupportCheck -- no --> Need9
    SupportCheck -- yes --> StabilityCheck

    StabilityCheck -- no --> Unstable
    StabilityCheck -- yes --> AuthorityCheck

    AuthorityCheck -- no --> Hallucination
    AuthorityCheck -- yes --> Admit
```

This is the DENDRAL equivalent of `Need9`: if reconstruction explodes, decompose rather than widen.

---

# 12. Reconstruction witness to POWL64

```mermaid id="qyzhfd"
flowchart TD
    ReconstructionResult["ReconstructionResult"]
    Fragments["FragmentRefs[]"]
    Constraints["ConstraintRefs[]"]
    Candidates["CandidateRefs[]"]
    Pruned["PrunedCandidateRefs[]"]
    Selected["SelectedCandidate"]
    Missing["MissingFragmentRefs"]
    Witness["ReconstructionWitness"]
    RouteCell["POWL64 RouteCell"]
    Blocked["BlockedAlternative<br/>if reconstruction blocks motion"]
    Replay["Replay verifies reconstruction"]

    ReconstructionResult --> Fragments
    ReconstructionResult --> Constraints
    ReconstructionResult --> Candidates
    ReconstructionResult --> Pruned
    ReconstructionResult --> Selected
    ReconstructionResult --> Missing

    Fragments --> Witness
    Constraints --> Witness
    Candidates --> Witness
    Pruned --> Witness
    Selected --> Witness
    Missing --> Witness

    Witness --> RouteCell
    Witness --> Blocked
    RouteCell --> Replay
    Blocked --> Replay
```

Replay question:

[
\boxed{
Given the same fragments and constraints, does the same reconstruction result emerge?
}
]

If not:

```text id="xlcdob"
ReplayInvalid
```

---

# 13. Immediate Rust surface

```rust id="kuk51f"
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct DendralByte(u8);

impl DendralByte {
    pub const FRAGMENTS_SUFFICIENT:     Self = Self(1 << 0);
    pub const CANDIDATE_GENERATED:      Self = Self(1 << 1);
    pub const CANDIDATE_PRUNED:         Self = Self(1 << 2);
    pub const UNIQUE_RECONSTRUCTION:    Self = Self(1 << 3);
    pub const MULTIPLE_RECONSTRUCTIONS: Self = Self(1 << 4);
    pub const MISSING_FRAGMENT:         Self = Self(1 << 5);
    pub const CONSTRAINT_VIOLATION:     Self = Self(1 << 6);
    pub const RECONSTRUCTION_UNSTABLE:  Self = Self(1 << 7);

    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Fragment {
    pub id: FragmentId,
    pub kind: FragmentKind,
    pub object: ObjectRef,
    pub time: TimeRange,
    pub asserts: FieldMask,
    pub digest: DigestRef,
    pub source: SourceId,
}

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ReconstructionCandidate {
    pub id: CandidateId,
    pub support: FieldMask,
    pub inferred: FieldMask,
    pub satisfied: ConstraintMask,
    pub violated: ConstraintMask,
    pub score: RankScore,
}

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ReconstructionResult {
    pub detail: DendralByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub selected: CandidateId,
    pub support: FieldMask,
    pub witness_index: ReconstructionWitnessId,
}
```

---

# Summary

[
\boxed{
Ground / SHRDLU binds objects.
}
]

[
\boxed{
STRIPS determines whether candidate action is enabled.
}
]

[
\boxed{
Prolog proves required relations.
}
]

[
\boxed{
HEARSAY-II fuses sources.
}
]

[
\boxed{
GPS reduces remaining gaps.
}
]

[
\boxed{
MYCIN applies policy/expert rules.
}
]

[
\boxed{
DENDRAL reconstructs bounded structures from fragments.
}
]

For the Access Drift JTBD:

```text id="9w6x4h"
Ground identifies contractor/vendor/badge/accounts/site/device.
STRIPS blocks AllowAccess and enables RevokeAccess.
Prolog proves active access after termination.
HEARSAY-II fuses HR/IAM/badge/vendor/device/policy evidence.
GPS selects smallest lawful next step.
MYCIN applies termination/vendor/badge/access policy.
DENDRAL reconstructs the timeline and access path from fragments.
```

Next KAPPA pack:

[
\boxed{\text{Reflect / ELIZA}}
]

That is the interface/conversation-control layer that reflects ambiguity, asks precise questions, slows premature action, and prevents LLM calls from becoming the default reflex.