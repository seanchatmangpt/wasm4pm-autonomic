# Layout, Padding, and Cache Alignment

*Secret Insight: The compiler's default layout is an educated guess. In INSA, we don't guess.*

When operating at byte-speed, the physical distance between bits is a semantic property. `Cog8Row` is exactly 32 bytes. Not 31. Not 33. This means 2 rows fit perfectly in a single 64-byte cache line.

## The Padding Secret
Rust does not guarantee struct field ordering unless you use `#[repr(C)]`. More dangerously, Rust will insert implicit padding bytes to align fields, which can leak uninitialized memory during bitwise hashing or serialization (e.g. into `POWL64`).

We explicitly control padding:
```rust
#[repr(C, align(32))]
pub struct Cog8Row {
    // ...
    pub _padding: [u8; 4],
}
```
*Why?* Because an explicit `_padding` field ensures that when we zero out the struct, the padding is deterministic. This makes hashing for receipts stable across architectures.

## Structure of Arrays (SoA) vs Array of Structures (AoS)
The Reference Path uses AoS for clarity (`[Cog8Row]`).
The SIMD Admitted Path uses SoA for batching (e.g., `required_masks: [u64; N]`).

We write `layout_gates.rs` to enforce these sizes via `core::mem::size_of` and `core::mem::align_of`. If a developer adds a `bool` flag to `Cog8Row` and bumps the size to 40 bytes, the build breaks immediately. 

*Core Team Verdict*: "Bytes that span a cache line boundary cost cycles. Bytes that represent hidden padding cost determinism."
