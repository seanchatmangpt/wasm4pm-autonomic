# Performance Honesty

Runtime performance in `ccog` is not a suggestion; it is a hard invariant.

## Hot-Path Purity
The `decide()` and `select_instinct_v0()` functions are the most critical paths in the cognitive substrate. To avoid unpredictable GC pauses or heap fragmentation, we enforce a zero-allocation policy.

- **`CountingAlloc`**: A thread-local, `#[global_allocator]`-backed instrumentation gate that traps all `alloc`/`dealloc` calls during measured blocks.
- **Positive Control**: Every performance test includes a `control_allocation_is_detected` check, proving the allocator itself is functional and capable of catching leaks.

If a developer introduces a `format!` or `Vec` into the hot path, the benchmark tier boundary check will immediately fail the gauntlet.
