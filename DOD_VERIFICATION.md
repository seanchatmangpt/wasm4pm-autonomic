# DOD Verification: Deterministic SARSA Refactor

## 1. ADMISSIBILITY
- All stochastic elements removed from `SARSAAgent`.
- Exploration is now handled via a **Deterministic Kernel Rotation (μ-rotation)** schedule (episode-dependent), ensuring that for any given state and episode, the action is perfectly repeatable.
- Verified by `test_sarsa_zero_variancy` and `test_sarsa_exploration_coverage`.
- Zero panics observed during intensive convergence testing.

## 2. MINIMALITY
- State representation complexity $\Phi(N)$ maintained as minimal. 
- SARSA agent now uses a **fixed-size array `[f32; 4]`** for Q-values in the `PackedKeyTable`, eliminating the need for `Vec<f32>` allocations per state.
- MDL formula $\Phi(N) = |T| + (|A| \cdot \log_2 |T|)$ is satisfied by the generated models (verified in `PetriNet::mdl_score`).

## 3. PERFORMANCE
- **Zero-heap allocation** achieved in the hot path:
    - State insertion in `update_with_next_action` uses stack-allocated arrays.
    - `select_action` and `greedy_action` perform no heap allocations.
- Branchless-friendly logic preserved in `greedy_action` and Bellman updates.

## 4. PROVENANCE
- `AGENTS.md` remains consistent with the deterministic architecture.
- `sarsa.rs` updated with μ-rotation and optimistic initialization (0.5).
- Every execution manifest emitted by the engine now reflects the deterministic trajectory.

## 5. RIGOR
- Property-based testing confirmed convergence in the corridor environment.
- Added `test_sarsa_zero_variancy` to assert `Var(τ) = 0`.
- Added `test_sarsa_exploration_coverage` to ensure all actions are explored deterministically.
- `test_sarsa_convergence` now passes consistently with `exploration_rate` set to 0.0 for evaluation.
