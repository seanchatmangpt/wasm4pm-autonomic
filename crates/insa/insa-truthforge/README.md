# `insa-truthforge`

**Comprehensive verification harness for INSA.**

This crate acts as the central gatekeeper for the INSA architecture. It contains property tests, benchmarks, and compile-fail assertions to guarantee layout stability and uphold the semantic invariants of the entire ecosystem. 

## Architectural Enforcement
`insa-truthforge` acts as the `O*` boundary enforcer. All verification runs explicitly ensure the strict INSA directive: *Never generate action or state mutations from unclosed fields.*

* **Layout Gates (`gates.rs`)**: Uses `memoffset` and compile-time assertions to ensure structs like `Cog8Row` remain exactly 32 bytes and `RouteCell64` remains exactly 64 bytes. This guarantees that future commits cannot accidentally break the zero-allocation cache-line architecture by sneaking in hidden pointers or unaligned bytes.
* **Admission Testing (`admission.rs`)**: Validates that generated receipts logically match the `A = µ(O*)` equation against mocked runtime states.
