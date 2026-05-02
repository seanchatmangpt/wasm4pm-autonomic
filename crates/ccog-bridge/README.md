# `ccog-bridge`

**Translation layer between `ccog` and the parent `dteam` crate.**

This crate serves as a dependency bridge, maintaining an acyclic dependency graph where `ccog` strictly acts as the logic kernel without directly importing `dteam` domain structures (the dep direction must point library → engine, never engine → library).

## The Translation Surface
* **`ontology_kbitset_to_present_mask`**: Translates the `dteam` internal `KBitSet<16>` ontology bitmask into the 64-bit `present_mask` shape that `ccog` consumes. Relies on a static `KBitMap` translation table.
* **`trace_to_runtime_response`**: Formats `ccog::CcogTrace` paths into `dteam` runtime `Response` summaries.
* **`receipt_to_runtime_evidence`**: Flattens internal execution `ccog::Receipt` payloads into `Evidence` shapes for long-term logging.
