# `insa-hotpath`

**The Reference Law Path for INSA execution.**

This crate is the high-performance core of the architecture. It implements the `COG8` execution kernel, `CONSTRUCT8` state mutators, and fast logical Lookup Tables (`lut`). The hotpath never allocates memory, ensuring nanosecond-tier predictable execution bounded entirely within CPU caches.

## `COG8` Semantic Evaluation
At the heart of the hotpath is the `Cog8Row`. Marked with `#[repr(C, align(32))]`, every row maps perfectly into AVX/SIMD instruction boundaries. 

The evaluation logic (`execute_cog8_graph`) processes an array of these rules against the current environment state (`present: u64` and `completed: u64`). The core matching function utilizes a branchless XOR-mask check:
```rust,ignore,ignore
let m1 = (present & row.required_mask.0) ^ row.required_mask.0;
let m2 = present & row.forbidden_mask.0;
let m3 = (completed & row.completed_block_mask.0) ^ row.completed_block_mask.0;
let matched = (m1 | m2 | m3) == 0;
```
If matched, the rule emits its embedded `InstinctByte` and `KappaByte`.

## `CONSTRUCT8` Mutation Bounds
INSA actively prevents runaway state alterations via the `Construct8Delta` structure. 
* All output mutations must fit within a strictly bounded array: `pub ops: [Construct8Op; 8]`.
* Each `Construct8Op` is restricted to `Set` or `Clear` against a single bit index.
* If an execution attempts to mutate more than 8 state variables simultaneously, the `push` operation fails synchronously, escalating the violation.