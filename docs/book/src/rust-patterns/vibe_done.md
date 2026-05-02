# Evidentiary Completion ("Vibe Done" vs "Vibe Coding")

In the INSA architecture, "Vibe coding" (where code simply compiles, looks right to an LLM, and passes superficial tests) is actively rejected. Instead, we adhere to a standard called **"Vibe Done"**, which means **Evidentiary Done**.

## The Standard
Code is considered complete *only* when it passes:
1. **Strict Layout Offsets**: Data structures must have guaranteed memory layouts (e.g., `#[repr(C)]`, `#[repr(packed)]`) and predictable byte sizing.
2. **Cross-Platform Wire Encoding Checks**: Serialization must be deterministic across platforms to guarantee consensus and replayability.
3. **Truthforge Admission Gates**: A dedicated verification harness (`insa-truthforge`) that subjects the code to adversarial mutation and invariants tests.

## No Deferred Work
We enforce an **Exhaustive Completeness** standard:
- Never write placeholders, stubs, or mocks in any codebase. 
- Never use `TODO`, `FIXME`, `unimplemented!()`, or defer logic "for production".
- Every implementation must handle all edge cases exhaustively, utilizing comprehensive pattern matching and defensive boundaries.

## The PROV Receipt
Ultimately, the goal of a valid state transition in DTEAM is to generate a **PROV receipt** (`R_U`). If an operation (`A_U`) cannot generate an independent, cryptographically verifiable proof of its action across the cold-path (`powl64`), the code is fundamentally incomplete.