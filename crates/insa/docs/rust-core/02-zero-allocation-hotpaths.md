# Zero-Allocation Hot Paths

*Secret Insight: Heap allocations are not slow because memory is slow. They are slow because synchronization is catastrophic.*
The INSA hot path (`execute_cog8_graph`, `InstinctResolution`, `POWL8` motion) must be absolutely, mathematically devoid of dynamic memory allocation.

## The Cost of `Vec<T>` and `Box<T>`
A `Vec` on the hot path introduces an unbounded synchronization point with the global allocator. At 10 million closures per second, the allocator lock contention will completely stall an M3 Max. 

## The INSA Constraint
- **No `String`**. Use static byte slices (`&\"static [u8]`) or fixed-width arrays (`[u8; N]`).
- **No `Vec`**. Use array-backed structures (e.g. `[Construct8Op; 8]`) or `slice` processing.
- **No `Box` or `dyn Trait`**. Dynamic dispatch destroys instruction cache locality and branch prediction.

## The Secret: Typestates and `repr(transparent)`
The real magic is using `#[repr(transparent)]` newtypes over primitives like `u64` and `u8`. We can imbue a simple byte with immense semantic meaning (`InstinctByte`, `KappaByte`) while the CPU literally just sees an 8-bit register load. 

*Core Team Verdict*: "If you ask for memory on the hot path, you have already failed the latency budget. The field must be closed within the L1 cache."
