# Byte-Width Semantic Multiplexing

A defining characteristic of the INSA architecture is **Byte-Width Semantic Multiplexing**. 

In the hot path, `u8` is treated as the primary "semantic lane." Instead of allocating complex structs or utilizing wide enums that mandate `match` branches, we encode complete state spaces, constraints, and instructions directly into a single byte (`u8`).

## Why `u8`?
- **Dense Packing:** It allows a power-set of logic to be compactly stored and processed. Memory layout guarantees are crucial for "Vibe Done".
- **SIMD Friendly:** Processing 8-bit integers is incredibly efficient across parallel SIMD lanes, allowing for simultaneous evaluation of many semantic channels.

### Example: Strict Layouts in `Cog8Row`
Below is a real memory-aligned struct from `insa-hotpath/src/cog8.rs`. Notice how `#[repr(C, align(32))]` is used alongside explicit byte-offset comments to guarantee the exact memory layout of the 8-bit semantic lanes (`InstinctByte`, `KappaByte`).

```rust
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::{CompletedMask, FieldMask, GroupId, PackId, RuleId};

/// A single atomic closure evaluation row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C, align(32))]
pub struct Cog8Row {
    pub required_mask: FieldMask,            // offset 0, 8 bytes
    pub forbidden_mask: FieldMask,           // offset 8, 8 bytes
    pub completed_block_mask: CompletedMask, // offset 16, 8 bytes

    pub pack_id: PackId,   // offset 24, 2 bytes
    pub group_id: GroupId, // offset 26, 2 bytes
    pub rule_id: RuleId,   // offset 28, 2 bytes

    pub response: InstinctByte, // offset 30, 1 byte
    pub kappa: KappaByte,       // offset 31, 1 byte
}
```

## The Cognitive 8-Bit Engines
This paradigm scales up into `insa-kappa8` and our Compiled Cognition pipelines (`cog8`, `powl8`, `construct8`). The logic of historical AI breeds (STRIPS, Prolog, MYCIN, Hearsay) are flattened down to operate on these byte-width lanes, allowing high-level cognitive closures to execute with hardware-level predictability.