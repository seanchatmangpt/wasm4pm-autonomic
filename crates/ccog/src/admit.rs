//! Branchless denial-polarity admission primitives.
//!
//! These primitives encode admissibility as a bitmask: a `verdict: u64` value
//! is "admitted" iff it equals `0`, and "denied" otherwise. The polarity is
//! deliberately inverted from the typical `bool` convention so that multiple
//! admissions can be composed via cheap, branchless bitwise OR:
//!
//! ```ignore
//! // Combined verdict is admitted iff *all* sub-verdicts are admitted.
//! let v = admit::admit3(check_a(), check_b(), check_c());
//! if admit::admitted(v) { /* fast path */ }
//! ```
//!
//! Polarity invariant:
//! - `0`            → admitted (the operation may proceed)
//! - non-zero       → denied   (the operation must be rejected)
//!
//! Note: ccog's polarity (`bool_mask(true) == 0`) is *flipped* from
//! `unibit-kernel`'s convention because in ccog `true` denotes admittance
//! crate-wide; the bitmask must therefore be `0` when the boolean condition
//! says "admitted".
//!
//! All primitives are `#[inline(always)]` so the compiler can fold a chain of
//! `admit*` calls into a sequence of `or` instructions with no branches.
//! `commit_masked_u64` and `commit_masked_bool` perform a *branchless select*
//! between two values based on the verdict; the generic `commit_masked<T>`
//! falls back to a conventional `if`/`else` (which LLVM typically lowers to a
//! `cmov`) so it can return arbitrary `Copy` types.

/// Returns `0` when `condition` is `true` (admitted), and `u64::MAX` when
/// `condition` is `false` (denied).
///
/// This is the canonical bridge from a `bool` predicate into the
/// denial-polarity bitmask space. The implementation is branchless:
/// `0u64.wrapping_sub((!condition) as u64)` produces `0` for `true` and
/// `0u64.wrapping_sub(1) == u64::MAX` for `false`.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(admit::bool_mask(true), 0);
/// assert_eq!(admit::bool_mask(false), u64::MAX);
/// ```
#[must_use]
#[inline(always)]
pub const fn bool_mask(condition: bool) -> u64 {
    0u64.wrapping_sub((!condition) as u64)
}

/// Composes two admission verdicts via bitwise OR.
///
/// The result is admitted (`0`) iff *both* inputs are admitted; otherwise the
/// denial bits propagate.
#[must_use]
#[inline(always)]
pub const fn admit2(a: u64, b: u64) -> u64 {
    a | b
}

/// Composes three admission verdicts via bitwise OR.
///
/// The result is admitted (`0`) iff *all three* inputs are admitted.
#[must_use]
#[inline(always)]
pub const fn admit3(a: u64, b: u64, c: u64) -> u64 {
    a | b | c
}

/// Composes four admission verdicts via bitwise OR.
///
/// The result is admitted (`0`) iff *all four* inputs are admitted.
#[must_use]
#[inline(always)]
pub const fn admit4(a: u64, b: u64, c: u64, d: u64) -> u64 {
    a | b | c | d
}

/// Returns `true` iff `verdict == 0` (i.e. the verdict is admitted).
///
/// This is the only place that converts a denial-polarity bitmask back into
/// the `bool` world. Callers should compose admissions with `admit2`/`admit3`
/// /`admit4` and keep the bitmask form as long as possible.
#[must_use]
#[inline(always)]
pub const fn admitted(verdict: u64) -> bool {
    verdict == 0
}

/// Branchless select between two `u64` values driven by `verdict`.
///
/// Returns `value` when `verdict` is admitted (`0`), and `fallback` otherwise.
/// The selection is performed via mask arithmetic with no conditional jumps:
///
/// ```text
///   m = bool_mask(admitted(verdict))   // 0 when admitted, u64::MAX when denied
///   result = (value & !m) | (fallback & m)
/// ```
///
/// When admitted: `m == 0`, so `!m == u64::MAX`, and the expression reduces to
/// `(value & u64::MAX) | (fallback & 0) == value`. When denied: `m == u64::MAX`,
/// so `!m == 0`, and the expression reduces to `fallback`.
#[must_use]
#[inline(always)]
pub const fn commit_masked_u64(verdict: u64, value: u64, fallback: u64) -> u64 {
    let m = bool_mask(admitted(verdict));
    (value & !m) | (fallback & m)
}

/// Branchless select between two `bool` values driven by `verdict`.
///
/// Derived from [`commit_masked_u64`] by reinterpreting each `bool` as `u64`
/// (`0` or `1`) and converting the result back via `!= 0`. Returns `value`
/// when admitted and `fallback` when denied.
#[must_use]
#[inline(always)]
pub const fn commit_masked_bool(verdict: u64, value: bool, fallback: bool) -> bool {
    commit_masked_u64(verdict, value as u64, fallback as u64) != 0
}

/// Generic select between two `Copy` values driven by `verdict`.
///
/// Returns `value` when admitted, otherwise `fallback`. This function is *not*
/// `const fn` because trait-bounded generics over arbitrary `Copy` types are
/// not yet stable in `const` context. The body is a plain `if`/`else`, which
/// LLVM consistently lowers to a conditional move (`cmov`) for primitive
/// payloads — so the runtime cost matches the bitmask path while preserving
/// type generality for callers.
#[must_use]
#[inline(always)]
pub fn commit_masked<T: Copy>(verdict: u64, value: T, fallback: T) -> T {
    if admitted(verdict) {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that `bool_mask(true)` returns `0` (admitted polarity).
    #[test]
    fn bool_mask_admitted_is_zero() {
        assert_eq!(bool_mask(true), 0);
    }

    /// Verifies that `bool_mask(false)` returns `u64::MAX` (denied polarity).
    #[test]
    fn bool_mask_denied_is_all_ones() {
        assert_eq!(bool_mask(false), u64::MAX);
    }

    /// Verifies that `admit2` admits only when both inputs are zero.
    #[test]
    fn admit2_admits_when_both_zero() {
        assert_eq!(admit2(0, 0), 0);
        assert_ne!(admit2(0, 1), 0);
        assert_ne!(admit2(1, 0), 0);
        assert_ne!(admit2(u64::MAX, 0), 0);
    }

    /// Verifies that `admit3` composes via bitwise OR.
    #[test]
    fn admit3_composes_via_or() {
        assert_eq!(admit3(0, 0, 0), 0);
        assert_eq!(admit3(0b001, 0b010, 0b100), 0b111);
        assert_ne!(admit3(0, 0, 1), 0);
        assert_ne!(admit3(u64::MAX, 0, 0), 0);
    }

    /// Verifies that `admit4` composes via bitwise OR.
    #[test]
    fn admit4_composes_via_or() {
        assert_eq!(admit4(0, 0, 0, 0), 0);
        assert_eq!(admit4(1, 2, 4, 8), 15);
        assert_ne!(admit4(0, 0, 0, 1), 0);
        assert_eq!(admit4(u64::MAX, u64::MAX, u64::MAX, u64::MAX), u64::MAX);
    }

    /// Verifies that `admitted` returns `true` only for the zero verdict.
    #[test]
    fn admitted_returns_true_only_for_zero() {
        assert!(admitted(0));
        assert!(!admitted(1));
        assert!(!admitted(u64::MAX));
        assert!(!admitted(0xDEAD_BEEF));
    }

    /// Verifies that `commit_masked_u64` selects the value when admitted.
    #[test]
    fn commit_masked_u64_selects_value_when_admitted() {
        assert_eq!(commit_masked_u64(0, 42, 99), 42);
        assert_eq!(commit_masked_u64(0, 0, 99), 0);
        assert_eq!(commit_masked_u64(0, u64::MAX, 0), u64::MAX);
    }

    /// Verifies that `commit_masked_u64` selects the fallback when denied.
    #[test]
    fn commit_masked_u64_selects_fallback_when_denied() {
        assert_eq!(commit_masked_u64(1, 42, 99), 99);
        assert_eq!(commit_masked_u64(u64::MAX, 42, 99), 99);
        assert_eq!(commit_masked_u64(0xDEAD_BEEF, 42, 99), 99);
    }

    /// Verifies that `commit_masked_bool` selects correctly across both polarities.
    #[test]
    fn commit_masked_bool_selects_correctly() {
        assert_eq!(commit_masked_bool(0, true, false), true);
        assert_eq!(commit_masked_bool(0, false, true), false);
        assert_eq!(commit_masked_bool(1, true, false), false);
        assert_eq!(commit_masked_bool(u64::MAX, false, true), true);
    }

    /// Verifies that the generic `commit_masked` selects correctly using a
    /// `(u64, u64)` tuple payload (exercising the generic path, not the
    /// `u64`/`bool` specializations).
    #[test]
    fn commit_masked_generic_selects_correctly() {
        let value: (u64, u64) = (1, 2);
        let fallback: (u64, u64) = (9, 8);
        assert_eq!(commit_masked::<(u64, u64)>(0, value, fallback), value);
        assert_eq!(commit_masked::<(u64, u64)>(1, value, fallback), fallback);
        assert_eq!(
            commit_masked::<(u64, u64)>(u64::MAX, value, fallback),
            fallback
        );
    }
}
