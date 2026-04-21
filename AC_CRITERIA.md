# AC_CRITERIA: Dr. Wil's Soundness Judge (Branchless Bitmask Checks)

## Objective
Implement a formal, DDS-grade soundness judge for Workflow Nets (WF-nets) that utilizes branchless bitmask calculus to ensure structural and behavioral soundness without data-dependent branching or heap churn.

## 1. Structural Soundness (Formal Properties)
- **AC 1.1: Unique Source/Sink Verification**: Assert exactly one source place (in-degree 0) and one sink place (out-degree 0) using bitwise reductions.
- **AC 1.2: Strong Connectivity of $\overline{N}$**: Verify that every node is on a path from source to sink. This is enforced by asserting that the short-circuited net $\overline{N}$ (original net plus an arc $o \to i$) is strongly connected.
- **AC 1.3: Branchless Transitive Closure**: Implementation must use bit-parallel transitive closure (e.g., Warshall's algorithm optimized for bitsets) or a bitmask-based BFS to check connectivity.

## 2. Behavioral Soundness (Structural Proxies)
- **AC 2.1: T-Invariant Admissibility**: Verify that for all transitions $t \in T$, there exists a firing sequence that includes $t$ and leads to the final marking. This is proxied by a branchless check for a positive T-invariant ($Wx = 0, x > 0$).
- **AC 2.2: Deadlock-Free Topology**: Enforce structural constraints (e.g., S-component coverage or Rank Theorem for Free-Choice nets) branchlessly to eliminate deadlock potential by construction.

## 3. Engineering Constraints (DDS-Grade)
- **AC 3.1: Zero-Heap Execution**: The judging kernel MUST NOT perform any heap allocations during evaluation. All temporary bitmasks must be stack-allocated or use pre-allocated buffers.
- **AC 3.2: Branchless Logic ($M' = (M \land \neg I) \lor O$)**: All decision-making paths in the judge must be expressed as bitmask transformations. Data-dependent `if/else` statements in the hot path are strictly prohibited.
- **AC 3.3: K-Tier Alignment**: All bitset operations must align with `KTier` word boundaries ($K \in \{64, 128, 256, 512, 1024\}$).

## 4. Integration & Governance
- **AC 4.1: Autonomic Guard Integration**: Integrate the judge into `src/autonomic/kernel.rs`. The `accept()` method must reject any `ActionType::Repair` that results in an unsound model when `strict_conformance` is active.
- **AC 4.2: Execution Provenance**: The soundness status of the model must be included in the `ExecutionManifest` $M$.

## 5. Verification Strategy
- **AC 5.1: Adversarial Topology Suite**: Implement a test suite in `tests/soundness_adversarial.rs` that includes:
    - Nets with disconnected islands.
    - Nets with multiple sources/sinks.
    - Nets with "sink-hole" cycles (unreachable from output).
    - Nets with dead transitions.
- **AC 5.2: Proptest Soundness Invariant**: Property-based tests in `src/models/petri_net.rs` to verify that the branchless judge matches the results of a classical (graph-traversal) soundness check.
- **AC 5.3: Performance Benchmarking**: Assert that the soundness judge contributes $< 500\text{ns}$ overhead to the autonomic proposal phase for nets up to $K=128$.

## 6. Migration & Backward Compatibility
- **AC 6.1: POWL Compatibility**: The `powl_to_wf_net` conversion must produce nets that pass the new soundness judge by construction.
- **AC 6.2: Legacy Replay Support**: The judge must not break existing token-based replay functionality for sound models.
