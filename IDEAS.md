1. **Formal Ontology Closure**: Implement strict activity footprint boundaries in the Engine to enforce O* and prevent out-of-ontology state reachability.
2. **Minimal Description Length (MDL) Refinement**: Upgrade the structural scoring in src/models/petri_net.rs to follow the DDS Φ(N) formula exactly.
3. **Deterministic Kernel μ Verification**: Create a cross-architecture test suite to verify that Var(τ) = 0 for the core RL execution kernel.
4. **Admissibility Reachability Pruning**: Implement branchless reachability guards to ensure that "Bad States" are mathematically unrepresentable in KBitSet markings.
5. **Cryptographic Execution Provenance**: Enhance ExecutionManifest to include full hashing of the input log, action trajectory, and resulting artifact {H(L), π, H(N)}.
6. **Blue River Dam Interface**: Refactor the AutonomicKernel to focus on "Control Surface Synthesis" (governance) rather than "Anomaly Detection (observation)."
7. **Branchless State-Equation Calculus**: Eliminate all remaining conditional logic in src/models/petri_net.rs by using mask-calculus for structural soundness verification.
