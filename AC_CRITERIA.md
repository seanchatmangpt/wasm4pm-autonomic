# AC_CRITERIA: Automated Activity-to-Index Mapping (Story 012)

## 1. Objective
Define the formal Acceptance Criteria (AC) for a DDS-grade implementation of activity-to-index mapping using FNV-1a with explicit collision guards, as required for the `dteam` Vision 2030 Roadmap.

## 2. Fundamental DDS Constraints (Consulted from DDS_THESIS.md)
| Principle | Requirement for Story 012 |
|-----------|---------------------------|
| **μ-Kernel Identity** | Mapping must be a pure, deterministic transformation: $Activities \to \{0..N-1\}$. |
| **Admissibility** | Hash collisions MUST result in an unrepresentable state (hard error), ensuring $BadOutcome \notin \mathcal{S}_{reachable}$. |
| **MDL Minimality** | The index space must be contiguous and minimal: $[0, |Activities|-1]$. |
| **Zero-Heap/Branchless** | While `compile` (Log Projection) may allocate, the *lookup* (Hot Path) must be branchless bitwise logic. |
| **Provenance** | Every mapping generation must be uniquely identifiable via its input hash $H(L)$ and output hash $H(N)$. |

## 3. Formal Acceptance Criteria

### AC 1: Deterministic Compilation (μ)
- [ ] The `DenseIndex::compile` function must produce identical output for identical input activity sets, regardless of input order.
- [ ] Implementation must use `fnv1a_64` as the base hashing primitive.
- [ ] Sorting of symbols prior to indexing must be stable and deterministic.

### AC 2: Collision Guard Admissibility
- [ ] `DenseIndex::compile` MUST detect FNV-1a collisions where $H(S_1) = H(S_2)$ but $S_1 \neq S_2$.
- [ ] Upon collision detection, the system MUST return a `DenseError::HashCollision` instead of proceeding.
- [ ] Verification: Property-based tests (Proptest) must prove that forced collisions are caught with 100% reliability.

### AC 3: Structural Minimality (MDL)
- [ ] The mapping must result in a `DenseId` (u32) that is a direct index into a flattened array.
- [ ] No "holes" are allowed in the index space $[0, N-1]$.
- [ ] The complexity $\Phi(N)$ of the resulting `ProjectedLog` must be verified as minimal.

### AC 4: Hot-Path Performance (Branchless/Zero-Heap)
- [ ] `DenseIndex::dense_id_by_hash` must perform lookup in $O(\log N)$ using binary search.
- [ ] The lookup path MUST NOT perform any heap allocations.
- [ ] Transition firing in `token_replay_projected` must remain branchless using the identity $M' = (M \land \neg I) \lor O$.

### AC 5: Execution Provenance
- [ ] The `ProjectedLog` must store the original activity symbols to allow for reverse mapping and auditability.
- [ ] Integration with `ExecutionManifest` must be demonstrated, capturing the hash of the activity ontology.

## 4. Verification Strategy
1. **Skeptic Contract**: Add a new contract in `src/skeptic_contract.rs` enforcing collision guard invariants.
2. **Proptests**: Implement `src/utils/dense_index_proptests.rs` to exercise the mapping with large, synthetic symbol sets.
3. **Benchmarking**: Use `benches/ktier_scalability_bench.rs` to ensure $Var(\tau) = 0$ across different log sizes.
4. **Audit**: Verify `AGENTS.md` is updated to reflect the new guarded mapping protocol.
