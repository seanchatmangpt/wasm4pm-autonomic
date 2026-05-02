# Adversarial & Contract Testing

To guarantee that implementations aren't just superficially correct, our testing strategy models both **Positive Contracts** (what the code *must* do) and **Negative Contracts** (what the code *must not* do), validated via `proptest`.

## Structure of a Verified Module

A standard algorithmic implementation includes:
1. The highly optimized, branchless target implementation.
2. An axiomatic, simple "reference oracle" implementation.
3. Explicit "mutants" injected into the test suite.

### Concrete Template from `bcinr_extended`

This pattern is aggressively enforced via scripts (like `fix_extraction.py`) ensuring every branchless implementation mathematically proves it catches flawed logic.

```rust
//! Branchless Implementation
//! Verified against axiomatic process intelligence constraints.

/// Positive Contract (Post-condition):
/// Result must be bitwise identical to the reference implementation.
///
/// Negative Contract (Adversarial):
/// This function must execute in constant time with zero data-dependent branches.
#[inline(always)]
#[no_mangle]
pub fn fast_algo(val: u64, aux: u64) -> u64 {
    // Highly optimized bitwise logic
    val ^ aux
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // 1. Reference Oracle
    fn fast_algo_reference(val: u64, aux: u64) -> u64 {
        val ^ aux // axiomatic proof logic here
    }

    // 2. Intentional Mutants
    /// Mutation 1: A "fake" implementation that returns a constant.
    /// This proves that our test suite rejects trivial placeholders.
    fn mutant_constant(_val: u64, _aux: u64) -> u64 {
        0
    }

    /// Mutation 2: An "overfit" implementation that passes small values but fails boundaries.
    fn mutant_overfit(val: u64, aux: u64) -> u64 {
        if val < 10 && aux < 10 {
            fast_algo_reference(val, aux)
        } else {
            0
        }
    }

    proptest! {
        /// Positive Proof: Prove the branchless implementation matches the reference oracle.
        #[test]
        fn test_positive_proof_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            let expected = fast_algo_reference(val, aux);
            let actual = fast_algo(val, aux);
            prop_assert_eq!(expected, actual, "Functional Equivalence Violation");
        }

        /// Negative Proof: Prove the test suite catches a constant mutant.
        #[test]
        fn test_negative_catch_constant_mutant(val in any::<u64>(), aux in any::<u64>()) {
            let expected = fast_algo_reference(val, aux);
            let mutant_val = mutant_constant(val, aux);
            
            if expected != 0 {
                prop_assert_ne!(mutant_val, expected, "Test suite failed to catch constant-zero mutant");
            }
        }

        /// Negative Proof: Prove the test suite catches an overfit mutant.
        #[test]
        fn test_negative_catch_overfit_mutant(val in 11..u64::MAX, aux in 11..u64::MAX) {
            let expected = fast_algo_reference(val, aux);
            let mutant_val = mutant_overfit(val, aux);
            
            if expected != 0 {
                prop_assert_ne!(mutant_val, expected, "Test suite failed to catch overfit mutant");
            }
        }
    }
}
```

This ensures the test suite actively tests the *tests* against adversarial subversion, catching "Vibe coding".