# Error Handling and Panic Eradication

*Secret Insight: `unwrap()` is a ticking time bomb. Stringly-typed errors are semantic cowards.*

The INSA hot path is designed to run millions of closures a second. A `panic!` inside a hot thread brings the entire node down.

## Eradicating `unwrap()` and `expect()`
We compile with `#![deny(clippy::unwrap_used)]` and `#![deny(clippy::expect_used)]`. 
If a state is truly unreachable, we use `unreachable!()` with an exhaustive comment explaining the geometric invariant that proves it, or we restructure the types so the state cannot be modeled.

## The Problem with `Result<T, &'static str>`
Early prototypes used `Result<T, &'static str>`. This is an anti-pattern. Strings must be parsed to be understood, meaning the downstream caller cannot react programmatically without text-matching.

22 The Zero-Cost Enum Solution
Errors must be explicitly modeled as `#[derive(Debug, Clone, Copy, PartialEq, Eq)] #[repr(u8)]` enums.
```rust
pub enum MaskError {
    OutOfRange = 1,
}
```
This reduces error bubbling to a single register check. It also maps perfectly onto the `telco` fault isolation taxonomy.

*Core Team Verdict*: "Panics are for broken hardware. Typed errors are for broken logic."
