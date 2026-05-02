# Branchless & Constant-Time Execution

In critical performance and security bounds (the "Law Path"), predictability is paramount. Code that diverges based on data introduces timing vulnerabilities, non-deterministic performance profiles, and potential exploitation avenues.

## The Branchless Contract

Functions operating within the hot-path (`insa-hotpath`, `bcinr_extended`) carry a strict adversarial contract: **They must execute in constant time with zero data-dependent branches.**

### Example: COG8 Graph Execution
Here is a production example from `insa-hotpath/src/cog8.rs`. Notice how `if/else` logic is entirely flattened into bitwise math. We avoid checking if masks intersect by branching; instead, we evaluate the bits directly.

```rust
#[inline(always)]
pub fn execute_cog8_graph(
    nodes: &[Cog8Row],
    present: u64,
    completed: u64,
) -> Result<Cog8Decision, &'static str> {
    let mut best = Cog8Decision {
        response: InstinctByte::empty(),
        completed_mask: completed,
        ..Default::default()
    };

    for (node_index, row) in nodes.iter().enumerate() {
        // Bitwise evaluation of requirements and forbidden states
        // m1: Are required bits missing?
        let m1 = (present & row.required_mask.0) ^ row.required_mask.0;
        // m2: Are forbidden bits present?
        let m2 = present & row.forbidden_mask.0;
        // m3: Is the completed block mask missing?
        let m3 = (completed & row.completed_block_mask.0) ^ row.completed_block_mask.0;
        
        // A single boolean evaluation without branching on the data
        let matched = (m1 | m2 | m3) == 0;

        if matched {
            // Note: In strict SIMD versions, even this `if` is removed and 
            // replaced with bitmask-blending (e.g. `val = (matched_mask & new_val) | (!matched_mask & old_val)`).
            best.fired_mask |= 1 << (node_index as u64);
            best.response = row.response;
            best.matched_pack_id = Some(row.pack_id);
            best.matched_group_id = Some(row.group_id);
            best.matched_rule_id = Some(row.rule_id);
            best.kappa = row.kappa;
        }
    }

    Ok(best)
}
```

### SIMD and Scalar Fallbacks

When implementing these algorithms:
1. We utilize explicit SIMD lanes (`std::simd` via portable-simd on nightly) for wide, parallel evaluations to remove even the loop `if` conditions via bit-blending.
2. We maintain a structurally identical scalar fallback that relies purely on bitwise evaluation.
3. Both SIMD and scalar paths are subjected to the same Positive/Negative Contract proofs.