# `insa-truthforge`

Comprehensive verification harness for INSA.

This crate acts as the central gatekeeper for the INSA architecture. It contains property tests, benchmarks, and compile-fail assertions to guarantee layout stability, verify `memoffset` boundaries, and uphold the semantic invariants of the entire ecosystem. 

All verification runs ensure the strict INSA directive: *Never generate action or state mutations from unclosed fields.*
