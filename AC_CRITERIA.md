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
