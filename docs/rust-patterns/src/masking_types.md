# Zero-Cost Masking & Constraints

To maintain strict boundaries between valid fields (`O*`) and raw data, DTEAM utilizes strongly-typed zero-cost abstractions, specifically within `no_std` environments (`insa-types`).

## Field Masks & Bits
Because of the heavy reliance on bitwise logic and multiplexing, masks are elevated to first-class types. This prevents integers from being accidentally confused with bit indices or raw masks.

### Example: FieldMask and FieldBit from `insa-types/src/mask.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FieldMask(pub u64);

impl FieldMask {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
    #[inline(always)]
    pub const fn with_bit(self, bit: FieldBit) -> Self {
        Self(self.0 | (1 << bit.get()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FieldBit(u8);

impl FieldBit {
    #[inline(always)]
    pub const fn new_checked(value: u8) -> Result<Self, &'static str> {
        if value < 64 {
            Ok(Self(value))
        } else {
            Err("FieldBit must be in range [0, 63]")
        }
    }
    #[inline(always)]
    pub const fn get(self) -> u8 {
        self.0
    }
}
```

## Domain Identifiers
We avoid passing raw `u64` or string IDs across boundaries. Instead, we wrap them in domain-specific types:
- `PackId`, `NodeId`, `BreedId`, `RouteId`, etc.
This ensures that at compile-time, an operation meant for a `Pack` cannot accidentally operate on a `Node`.

### The Immutable Boundary
By enforcing `A = µ(O*)` through types, if an entity has not been validated and reduced to a `CompletedMask` (part of the closed field `O*`), the API fundamentally will not accept it as input to the transition function `µ`. The type system itself prevents admission of unbound reality.