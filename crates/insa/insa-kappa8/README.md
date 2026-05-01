# `insa-kappa8`

Classical symbolic collapse engines for INSA.

This crate implements eight distinct breeds of classical artificial intelligence ("Old-AI") within the strict, bounded `Kappa8` memory model. Each breed is structurally constrained to the semantic equation `A = µ(O*)`. No engine is allowed to perform unbounded search or dynamic memory allocation.

## Engines
* **`prove_prolog`**: Horn clause / SLD resolution logic. Execution is constrained by a strict `ProofBudget(u8)`, evaluating `FactRow` and `HornClause` primitives entirely within bounded memory.
* **`rule_mycin`**: Certainty-factor based expert system routing.
* **`reflect_eliza`**: Pattern matching and reflective bindings.
* **`precondition_strips`**: Goal-state prerequisite checks.
* **`ground_shrdlu`**: Spatial / block-world grounding assertions.
* **`reconstruct_dendral`**: Combinatorial topology reductions.
* **`reduce_gap_gps`**: Means-ends analysis / gap reduction.
* **`fuse_hearsay`**: Blackboard / multi-expert evidence fusion.
