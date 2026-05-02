# Typestate and Monomorphization

*Secret Insight: A runtime error is a failure of imagination at compile time.*

INSA uses Typestates extensively (e.g., `Route<Unproofed>` vs `Route<Proofed>`). This ensures that impossible state transitions (emitting an unproofed route) are rejected by the compiler, not the runtime.

## The Secret: PhantomData and Zero-Cost
```rust
pub struct Route<State> {
    inner: RouteInner,
    _state: core::marker::PhantomData<State>,
}
```
`PhantomData` occupies absolutely zero space at runtime. The compiler uses the `State` type parameter to enforce rules during type checking and then completely erases it.

## The Monomorphization Trap
Generics are powerful, but excessive generic specialization leads to binary bloat and instruction cache thrashing. If `execute_cog8_graph` is generic over too many traits, the compiler generates dozens of identical copies of the function.

To prevent this:
1. **Dynamic Dispatch is Banned on Hot Paths**: We don't use `&dyn Trait` because vtables are slow.
2. **Inner Functions**: For heavy generic functions, we extract the core logic into a non-generic `inner` function taking slices, and use the generic outer function just for type checking.

*Core Team Verdict*: "Use the type system to make illegal states unrepresentable, but don't let it bloat your instruction cache."
