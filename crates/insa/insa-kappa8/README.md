# `insa-kappa8`

**Classical Symbolic AI Collapse Engines.**

The `insa-kappa8` crate resurrects classical symbolic AI models (Prolog, Mycin, Strips, etc.) but fundamentally rewires them to comply with the INSA layout constraints. None of these engines are permitted to perform unbounded search, recursion, or dynamic heap allocation (`Box`, `Vec`).

## Bound Semantic Engines

### `prove_prolog`: Bounded Horn-Clause Resolution
Implements SLD resolution using pre-compiled `FactRow` and `HornClause` slices. 
* To prevent infinite recursion cycles typical in Prolog, the engine forces a static `ProofBudget(u8)` (e.g., maximum depth of 4).
* If the depth budget is exhausted, it ceases calculation and emits `ProofStatus::DepthExhausted` linked with an `InstinctByte::ESCALATE` or `REFUSE` vector.

### `rule_mycin`: Expert System Certainty Routing
Evaluates an array of `ExpertRule` structures against the current `ClosureCtx`. 
* Uses a `CertaintyLane(u8)` primitive to determine rule weight.
* Operates in a single pass without allocating priority queues or rule graphs, simply selecting the rule with the highest certainty factor whose preconditions are met.

### Additional Micro-Engines
* **`reflect_eliza`**: Pattern matching and intent reflection bounds (`ElizaByte::MIRROR_INTENT`).
* **`precondition_strips`**: State transition prerequisite gates (`StripsByte::MISSING_REQUIRED`).
* **`ground_shrdlu`**: Contextual spatial validation assertions (`ShrdluByte::AMBIGUOUS_REFERENCE`).
* **`reconstruct_dendral`**: Combinatorial fragment reductions bounds (`DendralByte::MISSING_FRAGMENT`).
* **`reduce_gap_gps`**: Means-end analysis routing (`GpsByte::GAP_LARGE`).
* **`fuse_hearsay`**: Multi-expert blackboard validation (`HearsayByte::SOURCE_CONFLICTS`).