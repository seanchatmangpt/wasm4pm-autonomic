# Miri and Undefined Behavior (UB)

*Secret Insight: "It compiles" means nothing if it violates the memory model.*

UB in Rust allows the compiler to make terrifying optimizations that can silently alter program logic. Since INSA produces evidentiary receipts (`POWL64`), UB is not just a crash risk; it is a legal liability.

## Strict Provenance
Pointers in Rust are not just integers; they have "provenance" (a history of where they came from and what allocations they are allowed to touch).
Casting a pointer to a `usize`, doing math, and casting it back to a pointer destroys provenance and creates UB. 

INSA mandates the nightly `-Zmiri-strict-provenance` flag. We use `ptr::with_addr` and `ptr::addr` instead of raw `as usize` casts when manipulating bits for fast-path indexing.

## The Miri Gauntlet
Every single data structure manipulation in INSA must survive `cargo +nightly miri test`. Miri acts as a virtual machine that violently screams if you:
- Read uninitialized memory (like implicit padding).
- Create overlapping mutable references (aliasing violations).
- Perform unaligned reads/writes.

*Core Team Verdict*: "If Miri fails, the code is unadmitted. Evidence produced by a system with Undefined Behavior is not evidence; it is hallucination."
