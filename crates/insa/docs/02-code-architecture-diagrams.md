Below is the **code-architecture Mermaid pack** using the **Access Drift Closure** case study as the end-to-end JTBD test.

Case study:

```text
Terminated contractor still has active badge, VPN, repo access, vendor relationship, and recent site/device activity.
```

Target JTBD:

```text
When cross-field access drift appears across HR, vendor, badge, IAM, repo, device, and site fields,
the system must close the field, select lawful motion, block/refuse/escalate as needed,
produce a POWL64 evidence route, and replay the decision deterministically.
```

This uses the current INSA doctrine: byte-shaped hot path, `COG8 → KAPPA8/INST8 → POWL8 → CONSTRUCT8 → POWL64`, ReferenceLawPath equivalence, WireV1 canonical encoding, golden fixtures, and Truthforge admission.   

---

# 1. Workspace / Repo Architecture

```mermaid
flowchart TD
    Root["insa/"]

    Docs["docs/<br/>architecture, byte-law, TCPS, layout-admission, JTBD"]
    Ontology["ontology/<br/>insa.ttl, SHACL, SKOS vocab"]
    Crates["crates/"]
    Testdata["testdata/<br/>fixtures, golden wire bytes, graph cases"]
    Benches["benches/<br/>hot/warm/cold benchmarks"]
    Fuzz["fuzz/<br/>decode, route, closure, replay"]
    Examples["examples/<br/>access-drift-closure"]

    Root --> Docs
    Root --> Ontology
    Root --> Crates
    Root --> Testdata
    Root --> Benches
    Root --> Fuzz
    Root --> Examples

    Crates --> Types["insa-types"]
    Crates --> Instinct["insa-instinct"]
    Crates --> Kernel["insa-kernel"]
    Crates --> Motion["insa-motion"]
    Crates --> Construct["insa-construct"]
    Crates --> Powl64["insa-powl64"]
    Crates --> Replay["insa-replay"]
    Crates --> Pack["insa-pack"]
    Crates --> Graph["insa-graph"]
    Crates --> Security["insa-security"]
    Crates --> Import["insa-import"]
    Crates --> Truthforge["insa-truthforge"]
    Crates --> Cli["insa-cli"]
```

---

# 2. Crate Dependency DAG

```mermaid
flowchart TD
    Types["insa-types<br/>IDs, masks, digests, field bits, time"]
    Instinct["insa-instinct<br/>INST8, KAPPA8, inhibition, resolution, LUTs"]
    Kernel["insa-kernel<br/>COG8 rows, graphs, ReferenceLawPath"]
    Motion["insa-motion<br/>POWL8, AutonomicMotion, Route typestate"]
    Construct["insa-construct<br/>CONSTRUCT8, TypedDelta"]
    Powl64["insa-powl64<br/>RouteCell, WireV1, Evidence"]
    Replay["insa-replay<br/>ReplayValid / ReplayInvalid"]
    Pack["insa-pack<br/>.insa-pack bundling"]
    Graph["insa-graph<br/>enterprise configuration graph projection"]
    Security["insa-security<br/>security domain closure rows + JTBD"]
    Import["insa-import<br/>HR/IAM/badge/vendor/device/CVE connectors"]
    Truth["insa-truthforge<br/>admission, equivalence, falsification"]
    Cli["insa-cli<br/>operator commands"]

    Types --> Instinct
    Types --> Kernel
    Types --> Motion
    Types --> Construct
    Types --> Powl64
    Types --> Graph

    Instinct --> Kernel
    Instinct --> Motion
    Kernel --> Motion
    Motion --> Construct
    Motion --> Powl64
    Construct --> Powl64
    Powl64 --> Replay
    Powl64 --> Pack
    Replay --> Pack

    Import --> Graph
    Graph --> Security
    Security --> Kernel
    Security --> Motion
    Security --> Construct
    Security --> Powl64

    Truth --> Kernel
    Truth --> Instinct
    Truth --> Motion
    Truth --> Construct
    Truth --> Powl64
    Truth --> Replay

    Pack --> Cli
    Replay --> Cli
    Security --> Cli
```

---

# 3. Security Case Study Package Layout

```mermaid
flowchart TD
    Case["examples/access-drift-closure/"]

    Source["sources/<br/>hr.csv, iam.json, badge.csv, repo.json, vendor.csv, device.csv"]
    GraphFixture["graph/<br/>access_drift_case.ttl / graph.json"]
    Expected["expected/<br/>o_star.json, cog8_decision.json, resolution.json"]
    Evidence["expected/evidence/<br/>route_cell.bin, powl64_segment.bin"]
    Tests["tests/<br/>jtbd_access_drift.rs"]
    Report["report/<br/>board_closure_report.md"]

    Case --> Source
    Case --> GraphFixture
    Case --> Expected
    Case --> Evidence
    Case --> Tests
    Case --> Report
```

---

# 4. C4 Context: Code System Boundary

```mermaid
flowchart TD
    Board["Board / Audit Committee"]
    CISO["CISO / CSO"]
    SecurityOps["Security Operations"]
    HR["HR / People Ops"]
    Physical["Physical Security"]
    IAMTeam["IAM / Platform"]
    VendorRisk["Vendor Risk"]

    System["INSA Security Closure Runtime"]

    HRIS["HRIS"]
    IAM["IAM / SSO / PAM"]
    Badge["Badge System"]
    Repo["GitHub / GitLab"]
    MDM["MDM / EDR"]
    Vendor["Vendor Management"]
    GRC["GRC / Policy Exceptions"]

    Board --> System
    CISO --> System
    SecurityOps --> System
    HR --> System
    Physical --> System
    IAMTeam --> System
    VendorRisk --> System

    HRIS --> System
    IAM --> System
    Badge --> System
    Repo --> System
    MDM --> System
    Vendor --> System
    GRC --> System
```

---

# 5. C4 Container: Runtime Containers

```mermaid
flowchart TD
    Import["Import Container<br/>source adapters + normalization"]
    Graph["Graph Container<br/>enterprise config graph"]
    Closure["Closure Container<br/>O -> O*"]
    Hot["Hot Runtime Container<br/>COG8 / KAPPA8 / INST8 / LUTs"]
    Motion["Motion Container<br/>POWL8 + Route typestate"]
    Construct["Construct Container<br/>CONSTRUCT8 reentry"]
    Evidence["Evidence Container<br/>POWL64 WireV1 + pack"]
    Replay["Replay Container<br/>deterministic verification"]
    CLI["CLI/API Container"]
    Reports["Board Report Container"]

    Import --> Graph
    Graph --> Closure
    Closure --> Hot
    Hot --> Motion
    Motion --> Construct
    Construct --> Graph
    Motion --> Evidence
    Evidence --> Replay
    Replay --> Reports
    CLI --> Graph
    CLI --> Replay
    CLI --> Reports
```

---

# 6. Component Diagram: `insa-security`

```mermaid
flowchart TD
    Sec["insa-security"]

    Domain["domain/<br/>Person, Vendor, Badge, Device, Site, Policy"]
    Fields["fields/<br/>SecurityFieldBit assignments"]
    Rows["rows/<br/>AccessDriftCog8Rows"]
    Closures["closures/<br/>AccessDriftClosure"]
    Policies["policies/<br/>BadgePolicy, VendorPolicy, AccessPolicy"]
    JTBD["jtbd/<br/>access_drift_case"]
    Board["reporting/<br/>BoardClosureView"]

    Sec --> Domain
    Sec --> Fields
    Sec --> Rows
    Sec --> Closures
    Sec --> Policies
    Sec --> JTBD
    Sec --> Board

    Domain --> Fields
    Fields --> Rows
    Policies --> Rows
    Rows --> Closures
    Closures --> JTBD
    JTBD --> Board
```

---

# 7. Access Drift Domain Graph to O*

```mermaid
flowchart TD
    HR["HR Event<br/>contractor terminated"]
    Vendor["Vendor Contract<br/>expired"]
    Badge["Badge Credential<br/>active"]
    VPN["VPN Account<br/>active"]
    Repo["Repo Access<br/>active"]
    Site["Site Entry<br/>recent"]
    Device["Device Seen<br/>on local network"]
    Policy["Policy<br/>terminated access must be disabled"]

    Graph["Enterprise Config Graph"]
    Projection["Security O* Projection"]
    Masks["FieldMask + CompletedMask"]

    HR --> Graph
    Vendor --> Graph
    Badge --> Graph
    VPN --> Graph
    Repo --> Graph
    Site --> Graph
    Device --> Graph
    Policy --> Graph

    Graph --> Projection
    Projection --> Masks
```

---

# 8. FieldBit Assignment for Access Drift

```mermaid
flowchart TD
    subgraph FieldBits["AccessDrift FieldBits"]
        F0["0: identity_terminated"]
        F1["1: vendor_contract_expired"]
        F2["2: badge_active"]
        F3["3: vpn_active"]
        F4["4: repo_access_active"]
        F5["5: recent_site_entry"]
        F6["6: device_seen_on_site_network"]
        F7["7: policy_requires_access_removal"]
    end

    FieldMask["FieldMask u64"]
    COG8["COG8 Support<br/>≤ 8 load-bearing fields"]

    F0 --> FieldMask
    F1 --> FieldMask
    F2 --> FieldMask
    F3 --> FieldMask
    F4 --> FieldMask
    F5 --> FieldMask
    F6 --> FieldMask
    F7 --> FieldMask

    FieldMask --> COG8
```

---

# 9. COG8 Row Set for Access Drift

```mermaid
flowchart TD
    Row1["COG8 Row: TerminatedButDigitallyActive<br/>required: terminated + vpn_active + repo_active<br/>emits: Refuse + Escalate<br/>kappa: Ground + Rule + Prove"]
    Row2["COG8 Row: TerminatedButPhysicallyActive<br/>required: terminated + badge_active + recent_site_entry<br/>emits: Refuse + Inspect + Escalate<br/>kappa: Fuse + Rule"]
    Row3["COG8 Row: VendorExpiredButAccessActive<br/>required: vendor_expired + badge_active/vpn_active<br/>emits: Refuse + Retrieve<br/>kappa: Precondition + ReduceGap"]
    Row4["COG8 Row: PolicyViolationClosure<br/>required: policy_requires_access_removal + active_access<br/>emits: Refuse<br/>kappa: Rule + Prove"]

    Graph["Cog8Graph: AccessDrift"]
    Decision["Cog8Decision<br/>INST8 + KAPPA8 + fired rows"]

    Row1 --> Graph
    Row2 --> Graph
    Row3 --> Graph
    Row4 --> Graph
    Graph --> Decision
```

---

# 10. Hot Path Execution: Access Drift

```mermaid
flowchart LR
    Masks["FieldMask / CompletedMask"]
    Rows["Cog8Rows"]
    Eval["evaluate_cog8_graph"]
    Decision["Cog8Decision<br/>instincts + collapse"]
    LUT["INST8 Resolution LUT"]
    Resolution["InstinctResolution<br/>activation many, selected one"]
    POWL8["POWL8 Motion"]
    Motion["AutonomicMotion"]

    Masks --> Eval
    Rows --> Eval
    Eval --> Decision
    Decision --> LUT
    LUT --> Resolution
    Resolution --> POWL8
    POWL8 --> Motion
```

---

# 11. Access Drift INST8 Resolution

```mermaid
flowchart TD
    Activation["INST8 Activation<br/>Refuse + Inspect + Escalate + Retrieve"]
    Inhibit["InhibitionByte<br/>suppress unsafe motion, suppress vague review, suppress human burden"]
    Conflict["ConflictStatus<br/>valid / suspicious / conflict"]
    Selected["SelectedInstinctByte<br/>one-hot"]
    Class["ResolutionClass<br/>Blocked / Escalating / Externalizing"]

    Activation --> Inhibit
    Activation --> Conflict
    Activation --> Selected
    Activation --> Class

    Selected --> Refuse["Selected: Refuse<br/>block access / block route"]
    Class --> Escalate["Class: Escalating<br/>notify owner / security"]
```

---

# 12. POWL8 Motion for Access Drift

```mermaid
stateDiagram-v2
    [*] --> Act
    Act --> Choice
    Choice --> Block: Refuse selected
    Choice --> Retrieve: evidence missing
    Choice --> Inspect: ambiguity remains
    Block --> Escalate: privileged or site access active
    Retrieve --> Await: waiting for owner/system evidence
    Inspect --> Escalate: physical + digital mismatch
    Escalate --> Emit
    Await --> Loop: bounded recheck
    Loop --> Escalate: budget exhausted
    Emit --> [*]

    state Block {
        [*] --> RefuseAccess
        RefuseAccess --> RecordBlockedAlternative
    }

    state Emit {
        [*] --> EmitRouteFact
        EmitRouteFact --> EmitBoardFinding
    }
```

---

# 13. CONSTRUCT8 Reentry for Remediation

```mermaid
flowchart TD
    Motion["AutonomicMotion<br/>Refuse/Escalate/Retrieve"]
    Remediation["External remediation result<br/>badge disabled / VPN revoked / repo removed"]
    Observation["Observation<br/>not authority"]
    Validate["Validate<br/>source, freshness, authority, schema"]
    Delta["CONSTRUCT8 Delta<br/>≤8 typed entries"]
    Reclose["Re-close graph field<br/>O + ΔO -> O*"]
    Settle["Settle if access drift field closes"]
    Reject["Reject / Await / Ask if invalid"]

    Motion --> Remediation
    Remediation --> Observation
    Observation --> Validate
    Validate -- valid --> Delta
    Delta --> Reclose
    Reclose --> Settle
    Validate -- invalid --> Reject
```

---

# 14. POWL64 Evidence Route

```mermaid
flowchart TD
    Motion["AutonomicMotion"]
    RouteCell["RouteCell64<br/>ordinal, node, edge, op, INST8, KAPPA8"]
    Firing["Cog8FiringRecord64<br/>required/forbidden/completed masks"]
    Blocked["BlockedAlternative64<br/>blocked access route + reason"]
    Checkpoint["Checkpoint<br/>input/output field state"]
    Segment["Powl64Segment<br/>WireV1 canonical bytes"]
    Replay["ReplayValid"]
    Pack["INSA Evidence Pack"]

    Motion --> RouteCell
    Motion --> Firing
    Motion --> Blocked
    Motion --> Checkpoint

    RouteCell --> Segment
    Firing --> Segment
    Blocked --> Segment
    Checkpoint --> Segment

    Segment --> Replay
    Replay --> Pack
```

---

# 15. WireV1 Encoding Boundary

```mermaid
flowchart TD
    Layout["AdmittedLayout<br/>RouteCell64 / Cog8FiringRecord64"]
    Encode["Explicit WireV1 Encoder<br/>little-endian"]
    Bytes["Canonical bytes"]
    Golden["Golden fixture"]
    Decode["Explicit Decoder<br/>TryFrom, reserved-zero policy"]
    Roundtrip["decode(encode(x)) = x<br/>encode(decode(bytes)) = canonical(bytes)"]

    Layout --> Encode
    Encode --> Bytes
    Bytes --> Golden
    Golden --> Decode
    Decode --> Roundtrip

    Bad["raw transmute"]
    Layout -. forbidden .-> Bad
    Bad -. rejected .-> Encode
```

WireV1 must remain platform-independent, while admitted layouts may be target-specific; the v0.4 doctrine also requires golden byte fixtures and cross-platform encoding gates. 

---

# 16. JTBD Test Architecture

```mermaid
flowchart TD
    JTBD["JTBD: Access Drift Closure"]
    Given["Given<br/>terminated contractor + active badge/VPN/repo + vendor expired + site/device activity"]
    When["When<br/>security graph closes field"]
    Then["Then<br/>Refuse/Escalate selected + evidence pack replay-valid"]
    And["And<br/>no unproofed emission, no raw transmute, no hidden work"]

    Unit["Unit Tests<br/>types, masks, bytes"]
    Prop["Property Tests<br/>field combinations"]
    Compile["Compile-Fail Tests<br/>illegal states impossible"]
    Fixture["Fixture Tests<br/>access drift graph"]
    E2E["End-to-End JTBD Test"]
    Replay["Replay Test"]
    Golden["Golden Wire Test"]
    Bench["Benchmark Gate"]

    JTBD --> Given --> When --> Then --> And

    Given --> Unit
    Given --> Prop
    When --> Fixture
    When --> E2E
    Then --> Replay
    Then --> Golden
    And --> Compile
    And --> Bench
```

---

# 17. End-to-End JTBD Test Flow

```mermaid
sequenceDiagram
    participant Test as jtbd_access_drift.rs
    participant Import as insa-import
    participant Graph as insa-graph
    participant Sec as insa-security
    participant Kernel as insa-kernel
    participant Inst as insa-instinct
    participant Motion as insa-motion
    participant P64 as insa-powl64
    participant Replay as insa-replay

    Test->>Import: load HR/IAM/badge/vendor/repo/device fixtures
    Import->>Graph: normalize and insert graph facts
    Test->>Sec: project AccessDrift O*
    Sec->>Kernel: build FieldMask + Cog8Graph
    Kernel->>Kernel: evaluate_cog8_graph()
    Kernel-->>Sec: Cog8Decision
    Sec->>Inst: resolve INST8/KAPPA8
    Inst-->>Sec: InstinctResolution
    Sec->>Motion: select POWL8 AutonomicMotion
    Motion-->>Sec: Route<Proofed candidate>
    Sec->>P64: write RouteCell + Firing + BlockedAlternative
    P64-->>Test: Powl64Segment WireV1
    Test->>Replay: replay segment
    Replay-->>Test: ReplayValid
```

---

# 18. JTBD Acceptance Criteria

```mermaid
flowchart TD
    Start["Access Drift JTBD Acceptance"]
    A1["A1: graph fixture loads deterministically"]
    A2["A2: O* projection sets expected FieldBits"]
    A3["A3: COG8 rows fire expected rows"]
    A4["A4: Cog8Decision emits Refuse + Inspect + Escalate/Retrieve"]
    A5["A5: KAPPA8 includes Ground/Rule/Prove/Fuse as expected"]
    A6["A6: InstinctResolution selected instinct is one-hot"]
    A7["A7: POWL8 blocks access route and escalates"]
    A8["A8: POWL64 contains RouteCell + BlockedAlternative"]
    A9["A9: WireV1 golden bytes match"]
    A10["A10: replay returns ReplayValid"]
    A11["A11: benchmark/allocation gates pass"]

    Start --> A1 --> A2 --> A3 --> A4 --> A5 --> A6 --> A7 --> A8 --> A9 --> A10 --> A11
```

---

# 19. Truthforge Admission Gate for Case Study

```mermaid
flowchart LR
    Design["Design Intent<br/>Access drift must not become hidden risk"]
    Type["Type Law<br/>FieldBit, masks, SelectedInstinctByte"]
    Reference["ReferenceLawPath<br/>clear closure implementation"]
    Fast["Candidate Fast Path<br/>tables/SIMD/intrinsics if admitted"]
    Tests["Truthforge<br/>unit, prop, compile-fail, fuzz, replay"]
    Wire["WireV1 Golden Fixtures"]
    Bench["Benchmark Evidence<br/>allocs/op, ns/closure, route cells/sec"]
    Review["Adversarial Review<br/>false settle/refuse/escalate"]
    Admit["Admitted JTBD"]

    Design --> Type --> Reference --> Fast --> Tests --> Wire --> Bench --> Review --> Admit
```

---

# 20. False-Positive / False-Negative Security Tests

```mermaid
mindmap
  root((Access Drift JTBD Defects))
    False Settle
      active access remains
      field incorrectly closed
    False Ignore
      duplicate misclassified
      real drift suppressed
    False Retrieve
      asks for evidence already present
      creates unnecessary work
    False Ask
      vague owner request
      no exact missing fact
    False Await
      waits without deadline
      retry spam possible
    False Refuse
      blocks lawful access
      business disruption
    False Escalate
      escalates local closure
      noise to security owner
    False Inspect
      endless ambiguity
      no bounded path
```

No-at-scale tests are required early because INSA is a scaled inhibition system, not only a mask executor. 

---

# 21. ReferenceLawPath vs Candidate Fast Path

```mermaid
flowchart TD
    Fixture["Canonical Access Drift Fixture"]
    Ref["ReferenceLawPath"]
    Table["Table Path<br/>256 / 65,536 LUTs"]
    Simd["SIMD Path"]
    Intrinsic["Intrinsic Path"]
    Unsafe["Unsafe-Admitted Path"]

    RefOut["Reference Outputs<br/>Cog8Decision, Resolution, Motion, RouteFact"]
    FastOut["Candidate Outputs"]

    Compare["Equivalence Compare"]
    Verdict{"Equivalent?"}
    Admit["Admit candidate path"]
    Reject["Reject / classify failure"]

    Fixture --> Ref --> RefOut
    Fixture --> Table --> FastOut
    Fixture --> Simd --> FastOut
    Fixture --> Intrinsic --> FastOut
    Fixture --> Unsafe --> FastOut

    RefOut --> Compare
    FastOut --> Compare
    Compare --> Verdict
    Verdict -- yes --> Admit
    Verdict -- no --> Reject
```

The byte-law documentation makes LUTs central because `INST8` is a `u8` activation surface with 256 possible states, and `(KAPPA8, INST8)` gives 65,536 bounded signatures. 

---

# 22. Fixture-to-Golden Traceability

```mermaid
flowchart TD
    Fixture["testdata/cases/access_drift/input/*"]
    ExpectedGraph["expected/graph_snapshot.json"]
    ExpectedOStar["expected/o_star_access_drift.json"]
    ExpectedDecision["expected/cog8_decision.json"]
    ExpectedResolution["expected/instinct_resolution.json"]
    ExpectedMotion["expected/autonomic_motion.json"]
    ExpectedWire["golden/wire/route_cell_v1_le.bin"]
    ExpectedReplay["expected/replay_valid.json"]

    Fixture --> ExpectedGraph
    ExpectedGraph --> ExpectedOStar
    ExpectedOStar --> ExpectedDecision
    ExpectedDecision --> ExpectedResolution
    ExpectedResolution --> ExpectedMotion
    ExpectedMotion --> ExpectedWire
    ExpectedWire --> ExpectedReplay
```

---

# 23. Testdata Directory Architecture

```mermaid
flowchart TD
    Testdata["testdata/"]

    Cases["cases/"]
    Golden["golden/"]
    Mutants["mutants/"]
    FuzzSeeds["fuzz-seeds/"]
    BenchInputs["bench-inputs/"]

    Access["cases/access_drift/"]
    Inputs["input/<br/>hr, iam, badge, repo, vendor, device"]
    Expected["expected/<br/>o_star, decision, resolution, motion, replay"]
    Negative["negative/<br/>false_settle, false_ignore, false_refuse"]

    Wire["golden/wire/<br/>route_cell_v1_le.bin, powl64_header_v1_le.bin"]
    Graphs["golden/graph/<br/>access_drift_closure.ttl"]

    Testdata --> Cases
    Testdata --> Golden
    Testdata --> Mutants
    Testdata --> FuzzSeeds
    Testdata --> BenchInputs

    Cases --> Access
    Access --> Inputs
    Access --> Expected
    Access --> Negative

    Golden --> Wire
    Golden --> Graphs
```

---

# 24. Compile-Fail Architecture

```mermaid
flowchart TD
    CompileFail["trybuild compile-fail tests"]

    CF1["selected_instinct_multiple_bits.rs<br/>must fail"]
    CF2["emit_unproofed_route.rs<br/>must fail"]
    CF3["construct8_nine_entries.rs<br/>must fail"]
    CF4["fieldbit_out_of_range.rs<br/>must fail"]
    CF5["raw_u64_required_mask.rs<br/>must fail"]
    CF6["wire_transmute.rs<br/>must fail"]

    CompileFail --> CF1
    CompileFail --> CF2
    CompileFail --> CF3
    CompileFail --> CF4
    CompileFail --> CF5
    CompileFail --> CF6
```

---

# 25. Route Typestate in Code

```mermaid
stateDiagram-v2
    [*] --> Unproofed
    Unproofed --> Proofed: prove_route()
    Proofed --> Emitted: emit()

    Unproofed --> Rejected: invalid topology / missing reason / replay gap
    Proofed --> Rejected: wire encoding failure / digest failure

    note right of Unproofed
      Route<Unproofed>
      cannot emit
    end note

    note right of Proofed
      Route<Proofed>
      can emit route fact
    end note
```

---

# 26. Hot / Warm / Cold Code Paths

```mermaid
flowchart TD
    subgraph Hot["HOT crates / modules"]
        T["insa-types"]
        I["insa-instinct"]
        K["insa-kernel"]
        Tables["kernel/tables"]
    end

    subgraph Warm["WARM crates / modules"]
        M["insa-motion"]
        C["insa-construct"]
        P["insa-powl64 segment builder"]
    end

    subgraph Cold["COLD crates / modules"]
        R["insa-replay"]
        Pack["insa-pack"]
        RDF["ontology export / SHACL"]
        Reports["board reports"]
    end

    Hot --> Warm
    Warm --> Cold
```

Proof and explanation witness the hot path; they do not govern it. 

---

# 27. Benchmark Architecture for JTBD

```mermaid
flowchart TD
    Bench["benches/access_drift.rs"]

    B1["ns/cog8_row"]
    B2["ns/cog8_decision"]
    B3["ns/instinct_resolution"]
    B4["ns/powl8_motion"]
    B5["ns/route_cell_write"]
    B6["ms/powl64_replay"]
    B7["allocs/op = 0 for hot path"]
    B8["cross-profile equivalence"]

    Bench --> B1
    Bench --> B2
    Bench --> B3
    Bench --> B4
    Bench --> B5
    Bench --> B6
    Bench --> B7
    Bench --> B8
```

---

# 28. CI Admission Pipeline

```mermaid
flowchart LR
    Fmt["fmt"]
    Lint["clippy / deny"]
    Unit["unit tests"]
    Trybuild["compile-fail"]
    Prop["proptest"]
    Fuzz["fuzz smoke"]
    Golden["golden wire tests"]
    Replay["replay tests"]
    Bench["benchmark evidence"]
    Audit["layout/offset gates"]
    Admit["admitted release artifact"]

    Fmt --> Lint --> Unit --> Trybuild --> Prop --> Fuzz --> Golden --> Replay --> Bench --> Audit --> Admit
```

---

# 29. Failure Classification Pipeline

```mermaid
flowchart TD
    Failure["Test / replay / benchmark failure"]

    Semantic["Semantic failure<br/>wrong decision, wrong instinct"]
    Layout["Layout failure<br/>size/align/offset mismatch"]
    Encoding["Encoding failure<br/>golden bytes mismatch"]
    Replay["Replay failure<br/>route not valid"]
    Target["Target contract failure<br/>unsupported CPU / flags"]
    Benchmark["Benchmark failure<br/>allocs/op, regression"]
    Ontology["Ontology failure<br/>missing mapping / SHACL violation"]

    Failure --> Semantic
    Failure --> Layout
    Failure --> Encoding
    Failure --> Replay
    Failure --> Target
    Failure --> Benchmark
    Failure --> Ontology

    Semantic --> Andon["ANDON: stop line"]
    Layout --> Andon
    Encoding --> Andon
    Replay --> Andon
    Target --> Andon
    Benchmark --> Andon
    Ontology --> Andon
```

TCPS frames “done” as implemented, proven, replayable, and explainable, with evidence beating status and defects stopping the line. 

---

# 30. End-to-End Code Spine

```mermaid
flowchart TD
    Import["insa-import<br/>load source observations"]
    Graph["insa-graph<br/>build enterprise graph"]
    Security["insa-security<br/>project access-drift O*"]
    Kernel["insa-kernel<br/>COG8 closure"]
    Instinct["insa-instinct<br/>KAPPA8/INST8 resolution"]
    Motion["insa-motion<br/>POWL8 route"]
    Construct["insa-construct<br/>bounded reentry"]
    Powl64["insa-powl64<br/>route evidence"]
    Replay["insa-replay<br/>verify route"]
    Pack["insa-pack<br/>bundle evidence"]
    Cli["insa-cli<br/>board/security report"]

    Import --> Graph --> Security --> Kernel --> Instinct --> Motion
    Motion --> Construct --> Graph
    Motion --> Powl64 --> Replay --> Pack --> Cli
```

---

# 31. The Exact JTBD “Done” Definition

```mermaid
flowchart TD
    Done["JTBD Done"]

    D1["Field closed or explicitly not closed"]
    D2["COG8 decision deterministic"]
    D3["INST8 activation many allowed"]
    D4["Selected instinct one-hot"]
    D5["POWL8 route lawful"]
    D6["CONSTRUCT8 delta bounded"]
    D7["POWL64 evidence written"]
    D8["WireV1 canonical bytes stable"]
    D9["ReplayValid"]
    D10["Board report generated from evidence, not prose"]

    Done --> D1
    Done --> D2
    Done --> D3
    Done --> D4
    Done --> D5
    Done --> D6
    Done --> D7
    Done --> D8
    Done --> D9
    Done --> D10
```

---

# 32. Minimal First Implementation Cut

```mermaid
flowchart TD
    M0["Milestone 0: Access Drift JTBD"]

    Types["insa-types<br/>FieldBit, FieldMask, IDs"]
    Instinct["insa-instinct<br/>INST8/KAPPA8/SelectedInstinctByte/LUT"]
    Kernel["insa-kernel<br/>Cog8Row32, Cog8Graph, ReferenceLawPath"]
    Security["insa-security<br/>access drift rows and fixtures"]
    Powl64["insa-powl64<br/>RouteCell WireV1 + golden"]
    Replay["insa-replay<br/>ReplayValid for one segment"]
    Truth["insa-truthforge<br/>E2E JTBD gate"]

    M0 --> Types
    Types --> Instinct
    Instinct --> Kernel
    Kernel --> Security
    Security --> Powl64
    Powl64 --> Replay
    Replay --> Truth
```

---

# 33. Final Case Study Architecture Spine

```mermaid
flowchart TD
    O["Raw observations<br/>HR, IAM, badge, repo, vendor, device"]
    G["Enterprise graph"]
    OStar["AccessDrift O*"]
    COG8["COG8 rows fire"]
    KAPPA8["KAPPA8<br/>Ground + Rule + Prove + Fuse"]
    INST8["INST8<br/>Refuse + Inspect + Escalate + Retrieve"]
    RES["InstinctResolution<br/>selected Refuse/Escalate path"]
    POWL8["POWL8<br/>Block + Escalate + Retrieve/Await"]
    DELTA["CONSTRUCT8<br/>badge/VPN/repo revoked after validation"]
    PROOF["POWL64<br/>RouteCell + BlockedAlternative + Checkpoint"]
    REPLAY["ReplayValid"]
    REPORT["Board-ready evidence report"]

    O --> G --> OStar --> COG8
    COG8 --> KAPPA8
    COG8 --> INST8
    KAPPA8 --> RES
    INST8 --> RES
    RES --> POWL8
    POWL8 --> DELTA
    DELTA --> G
    POWL8 --> PROOF
    PROOF --> REPLAY
    REPLAY --> REPORT
```