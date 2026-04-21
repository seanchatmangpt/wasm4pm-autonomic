<<<<<<< HEAD
<<<<<<< HEAD
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
=======
# AC_CRITERIA: DDS-Grade LinUCB Implementation

## 1. Objective
Implement a linear reinforcement learning agent (LinUCB) that adheres to the **Deterministic Data Science (DDS)** paradigms: zero-heap, branchless execution, and deterministic transformation kernel identity ($Var(\tau) = 0$).

## 2. Acceptance Criteria

### AC 1: Zero-Heap State Matrices
- **Requirement:** All state variables for LinUCB ($A^{-1} \in \mathbb{R}^{D \times D}$ and $b \in \mathbb{R}^D$) must be stack-allocated.
- **Verification:** 
    - Use `const` generics for dimensions ($D, D^2$).
    - Zero runtime heap allocations in `select_action` and `update` paths.
    - Verified by `dhat` or manual inspection of MIR/LLVM IR.

### AC 2: Branchless Decision Kernel
- **Requirement:** Arm selection must use bitwise mask calculus to identify the optimal action.
- **Verification:**
    - Replace `if/else` or `max_by` with `bcinr`-style `select_f32` or equivalent mask-based comparisons.
    - Constant execution time (latency jitter $\approx 0$) regardless of input context.

### AC 3: Non-Allocating Feature Projection
- **Requirement:** `WorkflowState::features` (or its equivalent in the LinUCB hot path) must return features without heap allocation.
- **Verification:**
    - Refactor `WorkflowState` to provide a fixed-size array or reference to a stack-allocated buffer.
    - No `Vec<f32>` generation during feature extraction.

### AC 4: Deterministic Transformation Kernel (μ)
- **Requirement:** The state update $S_{t+1} = \mu(S_t, x_t, r_t)$ must be perfectly deterministic across all targets (x86_64, WASM).
- **Verification:**
    - $Var(\tau) = 0$ for any fixed input trajectory $\tau = \{(x_i, r_i)\}$.
    - State hash $H(A^{-1}, b)$ must be bit-identical across runs with identical inputs.

### AC 5: Admissibility and Stability
- **Requirement:** $A^{-1}$ must remain positive semi-definite to prevent matrix collapse or numerical instability.
- **Verification:**
    - Diagonal protection (e.g., `a_inv[i][i] = max(a_inv[i][i], min_eigen)`).
    - Hard bounds on all matrix/vector components to prevent overflow/underflow in long-running autonomic loops.

### AC 6: Execution Provenance (Manifest)
- **Requirement:** The LinUCB agent must emit a compliant `ExecutionManifest` for auditability.
- **Verification:**
    - Inclusion of input log hash $H(L)$, action trajectory $\pi$, and output model hash $H(N)$.
    - Reproducibility: Re-running with the manifest must yield an identical artifact.

### AC 7: Structural Minimality (MDL)
- **Requirement:** The linear model complexity must be minimized relative to the discovery accuracy.
- **Verification:**
    - Enforce $\min \Phi(N)$ where $N$ is the discovered process model guided by LinUCB.

## 3. Verification Strategy
- **Property-Based Testing:** Use `proptest` in `src/reinforcement_tests.rs` to verify $Var(\tau) = 0$ across $10^5$ iterations.
- **Zero-Heap Audit:** Use `cargo test` with a check for heap allocations to assert the hot path is truly zero-heap.
- **Manifest Playback:** Implement a reproduction test in `src/dteam/orchestration.rs` that verifies bit-identical model generation from an `ExecutionManifest`.
- **Latency Benchmark:** Profile `src/ml/linucb.rs` with `criterion` to ensure sub-microsecond latency with zero variance.
>>>>>>> wreckit/linear-reinforcement-learning-implement-linucb-with-zero-heap-state-matrices
=======
# AC_CRITERIA: MDL Refinement & Structural Scoring Upgrade

## Objective
The goal of this story is to upgrade the structural scoring mechanism within the Petri net model to strictly adhere to the Deterministic Data Science (DDS) $\Phi(N)$ formula. This ensures that the reinforcement learning discovery loop correctly penalizes structural complexity, preventing overfitting and ensuring model minimality.

## Acceptance Criteria

### 1. Mathematical Compliance ($\Phi(N)$)
- **Primary Formula**: The structural score (MDL) MUST be calculated exactly as $\Phi(N) = |T| + (|A| \cdot \log_2 |T|)$.
- **Transition Bound ($|T|$)**: Represents the total number of transitions in the Petri net.
- **Arc Bound ($|A|$)**: Represents the total number of arcs in the Petri net.
- **Logarithmic Scaling**: The penalty for arcs must scale with the logarithm of the number of transitions, as per the DDS thesis.
- **Exactness**: The implementation must not use approximations or heuristic simplifications (like $|T| + |A|$).

### 2. Implementation in `src/models/petri_net.rs`
- **Method Refinement**: The existing `mdl_score()` method must be the definitive source of structural scoring.
- **Floating Point Precision**: Use `f64` for internal calculation to maintain precision, and provide a safe conversion to `f32` if required by reward functions.
- **Edge Case Robustness**:
    - If $|T| = 0$, the score is $0.0$.
    - If $|T| = 1$, the score is $1.0$.
    - Negative values are impossible by construction.

### 3. Loop Integration & Reward Signal
- **Automation Update**: The `src/automation.rs` discovery loop must be updated to use `model.mdl_score()` for structural penalty calculation.
- **Heuristic Removal**: Any legacy complexity measures (e.g., `_complexity_c = transitions + arcs`) must be removed or replaced by the formal MDL score.
- **Policy Adherence**: The `mdl_penalty` from `dteam.toml` (if available) should be applied correctly as a multiplier to this score.

### 4. DDS Technical Mandates
- **Zero-Heap**: The scoring logic must perform zero heap allocations in the hot path.
- **Branchless**: The calculation should be expressed as a pure mathematical expression without data-dependent branching where possible.
- **Deterministic**: The result must be bit-identical across runs for the same Petri net structure.

### 5. Verification
- **Unit Testing**: Add comprehensive tests for sequence, choice, and loop structures to ensure MDL scores are calculated correctly.
- **Manifest Check**: Ensure the `ExecutionManifest` exported by the `Engine` contains the correct, refined MDL score.
- **Auditability**: Code comments must explicitly link the implementation to Section 3 of `DDS_THESIS.md`.
>>>>>>> wreckit/mdl-refinement-upgrade-structural-scoring-in-src-models-petri-net-rs-to-follow-φ-n-exactly
