# ccog (Compiled Cognition) Refactoring Evaluation

**Date:** 2026-04-29  
**Status:** Planning phase  
**Scope:** Extract classical AI systems + AutoML infrastructure into separate `ccog` crate

---

## 1. Current Landscape

### 1.1 Three Projects, Two Implementations of Classical AI

| Project | Location | Implementation Style | Systems | Status |
|---------|----------|----------------------|---------|--------|
| **unibit** | `crates/unibit-ai-classic` | POWL8/POWL64 Motion programs (geometric/kinetic) | ELIZA, SHRDLU, STRIPS, MYCIN, Hearsay-II | Published crate |
| **dteam** | `src/ml/{eliza,mycin,strips,shrdlu,hearsay}.rs` | Standalone Rust + branchless u64 bit-packed ops | ELIZA, MYCIN, STRIPS, SHRDLU, Hearsay-II + AutoML pairs | Monolith |
| **chatmangpt/bcinr** | `bcinr/` | Bitwise Computation INtegration Reductions | Core bitwise primitives | Workspace |

### 1.2 Key Insight: Two Complementary Views

**unibit-ai-classic** encodes the *inference cycle shape* as POWL processes:
- Weizenbaum's ELIZA modeled as: `Loop(Seq(receive_input, Seq(match_pattern, apply_template)))`
- Shortliffe's MYCIN modeled as: multi-branch forward/backward chaining Motion
- Winograd's SHRDLU modeled as: spatial relation predicates in geometric coordinates

**dteam/src/ml** implements the *inference operations* as branchless evaluation:
- ELIZA: u64 keyword bitmask → template index lookup (5–50 ns)
- MYCIN: certainty-factor algebra on fixed rule base (branchless chains)
- SHRDLU: spatial predicates as type lattice queries (stack-allocated)

**Relationship:** Complementary, not competitive. The POWL shape is *what to do*; the branchless code is *how fast to do it*.

### 1.3 Current dteam Monolith State

**Cargo structure:**
- Single package: `dteam` version 1.3.0
- Workspace declared but empty (no members)
- Already depends on: `unibit-globe`, `unibit-lane`, `unibit-powl`, `unibit-graph`, `unibit-kernel`

**ML module (src/ml/) — 56 files, ~1400 KB:**

**Classical AI core (5 systems + AutoML pairs):**
- `eliza.rs` (23 KB) + `eliza_automl.rs` (7 KB)
- `mycin.rs` (20 KB) + `mycin_automl.rs` (7 KB)
- `strips.rs` (17 KB) + `strips_automl.rs` (6 KB)
- `shrdlu.rs` (24 KB) + `shrdlu_automl.rs` (8 KB)
- `hearsay.rs` (19 KB) + `hearsay_automl.rs` (9 KB)
- **Total:** ~170 KB (12% of src/ml/)

**Supporting infrastructure:**
- `drift_detector.rs` (13 KB) — Per-tier accuracy monitoring & signal emission
- `retraining_orchestrator.rs` (12 KB) — Routes drift signals to retraining actions
- `compiler.rs` (6 KB) — Compile-time code generation entry point
- `classic_ai_signals.rs` (5 KB) — Feature extraction for symbolic systems
- `automl.rs` (16 KB), `automl_config.rs` (12 KB), `automl_eval.rs` (12 KB) — Generic AutoML loop
- `hdit_automl.rs` (50 KB) — HDIT (Hyperparameter-Driven Inductive Training) loop
- **Total:** ~136 KB

**Learners & utilities (30+ files):**
- Decision trees, gradient boosting, logistic regression, Naive Bayes, neural networks, ensemble methods, statistical tests
- PCA, dimensionality reduction, clustering, recommenders, NLP, network analysis, synthetic training
- **Total:** ~1000 KB

---

## 2. What Should Go Into ccog

### 2.1 Tier 1: Core Classical AI Systems (MUST MOVE)

**Files to move — 170 KB, zero external dependencies within dteam**

```
ccog/src/
├── lib.rs (re-exports)
├── eliza.rs (+ eliza_automl.rs)
├── mycin.rs (+ mycin_automl.rs)
├── strips.rs (+ strips_automl.rs)
├── shrdlu.rs (+ shrdlu_automl.rs)
├── hearsay.rs (+ hearsay_automl.rs)
└── (no direct deps on src/ml/decision_tree, src/ml/gradient_boosting, etc.)
```

**Rationale:**
- Clean semantic boundary: "five historical symbolic/learned AI system pairs"
- No interdependencies between systems
- Each is a standalone Compiled Cognition reference implementation
- Documented in COMPILED_COGNITION.md §5.1–5.5

**Breaking changes:** None to dteam. ccog is a new crate.

### 2.2 Tier 2: Decision-Critical Infrastructure (SHOULD MOVE)

**Files — 50 KB, core to Compiled Cognition auditability**

```
ccog/src/
├── drift_detector.rs
├── retraining_orchestrator.rs
├── compiler.rs
├── classic_ai_signals.rs
└── (no changes to dteam/src/ml/)
```

**Rationale:**
- Drift detection + retraining orchestration are *integral* to ccog's auditability contract
- COMPILED_COGNITION.md §5.6 ("Auditability is Structural") depends on this infrastructure
- Clearer mental model: ccog owns the feedback loop
- These files currently have zero external callers in dteam/src/ (checked via grep)

**Integration:**
- ccog internal only (not exported to dteam consumers)
- dteam can still use generic `src/ml/automl.rs` and `src/ml/hdit_automl.rs` for other domains

### 2.3 Tier 3: Learners & Utilities (MAY MOVE or STAY)

**Decision point: reusability vs. ccog scope**

**Option A: Core learners stay in dteam/src/ml/**
- Decision trees, gradient boosting, logistic regression, Naive Bayes
- REASONING: These are domain-agnostic; ccog should not own them
- ccog imports them: `use dteam::ml::{DecisionTree, GradientBoosting, LogisticRegression, ...}`
- Keeps ccog focused on classical AI systems + their AutoML pairing

**Option B: All utilities move to ccog**
- Everything moves: learners, stats, linalg, clustering, etc.
- ccog becomes: comprehensive classical AI + modern ML library
- REASONING: Cleaner separation; dteam core stays focused on process intelligence
- Downside: Large crate, harder to version independently

**Recommendation: Option A** (learners stay, ccog is lightweight)
- ccog is ~220 KB (systems + infrastructure)
- Crisp scope: "Compiled Cognition classical AI reference implementations"
- dteam/src/ml stays as-is for use by other modules (discovery, autonomic, etc.)

---

## 3. Integration With unibit-ai-classic

### 3.1 Complementary, Not Redundant

| Aspect | unibit-ai-classic | ccog (dteam) |
|--------|------------------|--------------|
| **Abstraction** | Process shape (POWL8/POWL64 Motion) | Runtime operation (u64 branchless) |
| **Use case** | Workflow definition, governance, process model | Inline decision at nanosecond scale |
| **Determinism proof** | Shape invariants, kinetic/geometric match | Bit-exact reproducibility, no branches |
| **Auditability** | Process conformance traces | Prediction logs + drift signals |

### 3.2 Potential Bridge (Future Work)

A third crate `ccog-bridge` (or similar) could:
1. Take unibit-ai-classic Motion → extract POWL shape
2. Compile shape into ccog runtime decision tree
3. Provide dual-mode: "is this trace conformant to the declared POWL process?" + "is this input classified correctly?"

**Not in scope for this refactoring, but documented for future exploration.**

---

## 4. Integration With bcinr

### 4.1 Bitwise Primitives

bcinr provides core bitwise operations. dteam already uses them:
- Imported via Cargo: `bcinr = "26.4.18"` (public crate)
- Used in: `src/utils/dense_index.rs`, Petri net marking bitmasks, u64 branchless operations

### 4.2 ccog Use of bcinr

ccog will inherit bcinr usage from dteam. No new dependency; just documented:
- ELIZA keyword bitmask selection uses bcinr primitives
- MYCIN certainty-factor algebra can use bcinr conditional logic
- No API changes needed

---

## 5. Proposed Refactoring Steps

### Phase 1: Create ccog Crate (Lowest Risk)

```bash
# 1. Create crate structure
mkdir -p crates/ccog/src
touch crates/ccog/Cargo.toml

# 2. Write Cargo.toml
[package]
name = "ccog"
version = "0.1.0"
edition = "2021"
license = "BUSL-1.1"
description = "Compiled Cognition: Classical AI systems as deterministic compiled artifacts"

[dependencies]
# Minimal: only what's needed for the five systems
dteam = { path = ".." }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
blake3 = "1.5"

# 3. Update dteam/Cargo.toml
[workspace]
members = ["crates/ccog"]

# 4. Move files
cp src/ml/{eliza,mycin,strips,shrdlu,hearsay}.rs crates/ccog/src/
cp src/ml/{eliza_automl,mycin_automl,strips_automl,shrdlu_automl,hearsay_automl}.rs crates/ccog/src/
cp src/ml/{drift_detector,retraining_orchestrator,compiler,classic_ai_signals}.rs crates/ccog/src/

# 5. Update imports in moved files
# s/use crate::ml::/use dteam::ml::/g
# etc.

# 6. Update dteam/src/lib.rs
# Remove pub mod ml::{eliza, mycin, ...}
# Add pub use ccog::{eliza, mycin, ...} for re-export

# 7. Test
cargo test -p ccog
cargo test -p dteam
```

### Phase 2: Documentation & Boundary Clarity (Low Risk)

```bash
# 1. Write ccog/README.md
# - Overview of the five systems
# - Link to COMPILED_COGNITION.md §5.1–5.5
# - Example: using ELIZA for intent classification

# 2. Write ccog/ARCHITECTURE.md
# - How each system encodes its inference cycle
# - Branchless evaluation strategy
# - Auditability contract (drift detection, retraining triggers)

# 3. Add module-level docs to each system
# - /// ELIZA (Weizenbaum 1966) — Intent Classification via Compiled Cognition
# - Link to reference paper
# - Explain S ⊕ L decomposition
```

### Phase 3: Learner Dependencies (Optional, Higher Risk)

**Decision:** Leave as-is (Option A) for now. Can revisit in Phase 4 if scope expands.

---

## 6. Risk Assessment

### 6.1 Low-Risk Moves
- **Classical AI systems** (5 files × 2 = 10 files): Zero dependencies on each other, well-encapsulated
- **Drift detector, retraining orchestrator**: No external callers; internal to ccog
- **Workspace conversion**: dteam already has `[workspace]` declared; just adding members

### 6.2 Medium-Risk Areas
- **Import rewriting**: Need to grep-replace `use crate::ml::` with `use dteam::ml::` in moved files
- **Re-export in dteam**: Ensure backward compatibility for existing code that imports from `dteam::ml::eliza`
- **Testing**: Ensure all integration tests still pass

### 6.3 Mitigation
1. **Commit incrementally**: One system per commit (ELIZA, MYCIN, etc.) so rollback is surgical
2. **Test each step**: `cargo test -p ccog && cargo test -p dteam` after each move
3. **Grep-verify**: Ensure no internal imports are broken
4. **Branch strategy**: Do this on `fix/dteam-round3` and open a PR for review before merge

---

## 7. Files Affected

### To Move (220 KB total)

```
dteam/src/ml/eliza.rs                    → ccog/src/eliza.rs
dteam/src/ml/eliza_automl.rs             → ccog/src/eliza_automl.rs
dteam/src/ml/mycin.rs                    → ccog/src/mycin.rs
dteam/src/ml/mycin_automl.rs             → ccog/src/mycin_automl.rs
dteam/src/ml/strips.rs                   → ccog/src/strips.rs
dteam/src/ml/strips_automl.rs            → ccog/src/strips_automl.rs
dteam/src/ml/shrdlu.rs                   → ccog/src/shrdlu.rs
dteam/src/ml/shrdlu_automl.rs            → ccog/src/shrdlu_automl.rs
dteam/src/ml/hearsay.rs                  → ccog/src/hearsay.rs
dteam/src/ml/hearsay_automl.rs           → ccog/src/hearsay_automl.rs
dteam/src/ml/drift_detector.rs           → ccog/src/drift_detector.rs
dteam/src/ml/retraining_orchestrator.rs  → ccog/src/retraining_orchestrator.rs
dteam/src/ml/compiler.rs                 → ccog/src/compiler.rs
dteam/src/ml/classic_ai_signals.rs       → ccog/src/classic_ai_signals.rs
```

### To Update (Imports, Re-exports)

```
dteam/Cargo.toml                 (add [workspace] members = ["crates/ccog"])
dteam/src/lib.rs                 (remove pub mod ml::{eliza, mycin, ...}; add re-exports)
dteam/src/ml/mod.rs              (remove moved modules from re-exports)
dteam/src/ml/*.rs                (any file that imports from moved modules)
```

### To Leave Untouched

```
dteam/src/ml/{decision_tree,gradient_boosting,logistic_regression,...}.rs
dteam/src/ml/{automl,hdit_automl,automl_config,automl_eval}.rs
```

---

## 8. Success Criteria

1. ✅ ccog crate compiles in isolation (`cargo test -p ccog`)
2. ✅ dteam still compiles (`cargo test -p dteam`)
3. ✅ All existing imports still work (backward compat via re-exports)
4. ✅ Integration tests pass (both ccog and dteam)
5. ✅ COMPILED_COGNITION.md §5.1–5.5 modules all addressable as `ccog::{eliza, mycin, ...}`
6. ✅ Drift detection + retraining orchestration work end-to-end
7. ✅ No breaking changes to public API

---

## 9. Open Questions

1. **Learner ownership**: Should gradient boosting, decision trees, logistic regression stay in dteam/src/ml/ (Option A) or move to ccog (Option B)?
   - **Current recommendation:** Option A (stay in dteam). Keep ccog focused.

2. **ccog visibility**: Should ccog be published separately (like unibit-ai-classic) or internal to dteam?
   - **Current recommendation:** Internal for now. Can be extracted later if external demand arises.

3. **Bridge to unibit-ai-classic**: Should Phase 2 include a `ccog-bridge` crate?
   - **Current recommendation:** Document for Phase 4, don't implement in this refactoring.

4. **Binary/tool**: Should we create a `ccog` CLI tool (like `ralph`) for profiling/benchmarking?
   - **Current recommendation:** No. Let existing binaries (ml_bench, etc.) continue.

---

## 10. Next Steps

1. **Get user approval** of this evaluation (you're reading it now)
2. **Decide: Option A vs. Option B** for learner ownership
3. **Decide: Phase 1 + 2 now, or Phase 1 only?**
4. **If approved:** Execute Phase 1 systematically (one system per commit)
5. **After Phase 1:** Measure compilation time, benchmarks, test coverage

---

## Appendix A: Dependency Graph

```
dteam
├── unibit-globe (*)
├── unibit-lane (*)
├── unibit-powl (*)
├── unibit-graph (*)
├── unibit-kernel (*)
├── bcinr "26.4.18" (public, for bitwise ops)
├── [other standard deps: serde, tokio, blake3, etc.]
└── ccog/ (new)
    ├── (depends on dteam for learners)
    └── (uses bcinr transitively via dteam)

(*) Already present in dteam/Cargo.toml
```

---

## Appendix B: Files in src/ml/ — Inventory

**Classical AI core (MOVE TO CCOG):**
1. eliza.rs, eliza_automl.rs
2. mycin.rs, mycin_automl.rs
3. strips.rs, strips_automl.rs
4. shrdlu.rs, shrdlu_automl.rs
5. hearsay.rs, hearsay_automl.rs
6. drift_detector.rs
7. retraining_orchestrator.rs
8. compiler.rs
9. classic_ai_signals.rs

**Learners & Utilities (STAY IN DTEAM):**
10. automl.rs, automl_config.rs, automl_eval.rs
11. hdit_automl.rs
12. decision_tree.rs, decision_stump.rs
13. gradient_boosting.rs
14. logistic_regression.rs, linear_regression.rs
15. gaussian_naive_bayes.rs, naive_bayes.rs
16. neural_network.rs, deep_learning.rs
17. pca.rs, hierarchical_clustering.rs, kmeans.rs, knn.rs, nearest_centroid.rs
18. network_analysis.rs, nlp.rs, word_vectors.rs
19. recommender.rs, pdc_ensemble.rs, pdc_features.rs, pdc_supervised.rs, pdc_unsupervised.rs, pdc_combinator.rs
20. stacking.rs, weighted_vote.rs, rank_fusion.rs
21. linalg.rs, stats.rs, gradient_descent.rs, synthetic_trainer.rs
22. hdc.rs (Hyperdimensional Computing)
23. perceptron.rs
24. tests.rs

---

**End of Evaluation Document**
