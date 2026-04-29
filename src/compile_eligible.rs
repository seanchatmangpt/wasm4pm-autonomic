//! Compile eligibility predicate — determining when a domain is suitable for compilation.
//!
//! The five conditions from COMPILED_COGNITION.md §7.2 operationalize the decision:
//! is this domain stable enough, bounded enough, and sensitive to latency enough to warrant
//! compile-time transformation?

/// The five eligibility conditions from COMPILED_COGNITION.md §7.2.
/// Each field is `true` when the corresponding condition is met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileEligibleCheck {
    /// Condition 1: Stable(D) — domain distribution is stable (no drift detected).
    pub stable: bool,
    /// Condition 2: Finite(O*) — output space is finite (bounded cardinality).
    pub finite_output: bool,
    /// Condition 3: Fast(D) — latency requirement is sub-millisecond (< 1 ms).
    pub fast: bool,
    /// Condition 4: Auditable(D) — bit-exact replay is required.
    pub auditable: bool,
    /// Condition 5: Local(D) — no runtime external calls required.
    pub local: bool,
}

impl CompileEligibleCheck {
    /// Returns `true` only when ALL five conditions are met.
    pub fn is_eligible(&self) -> bool {
        self.stable && self.finite_output && self.fast && self.auditable && self.local
    }

    /// Human-readable summary of which conditions pass and which fail.
    pub fn summary(&self) -> [(&'static str, bool); 5] {
        [
            ("Stable(D)", self.stable),
            ("Finite(O*)", self.finite_output),
            ("Fast(D)<1ms", self.fast),
            ("Auditable(D)", self.auditable),
            ("Local(D)", self.local),
        ]
    }
}

/// Input descriptor for a domain that is being evaluated for compile eligibility.
/// Callers fill this in; `check()` maps it to a `CompileEligibleCheck`.
pub struct CompileEligibleInput {
    /// Whether the domain's distribution has been declared stable (no drift).
    pub distribution_stable: bool,
    /// Whether the output set is bounded to a compile-time-known cardinality.
    pub output_space_finite: bool,
    /// Observed or specified worst-case latency in microseconds.
    pub latency_us: u64,
    /// Whether audit/replay is a stated requirement for this domain.
    pub requires_audit: bool,
    /// Whether the domain requires any runtime network/file/external calls.
    pub has_external_calls: bool,
}

/// Evaluate all five conditions and return a per-condition pass/fail result.
pub fn check(input: &CompileEligibleInput) -> CompileEligibleCheck {
    CompileEligibleCheck {
        stable: input.distribution_stable,
        finite_output: input.output_space_finite,
        fast: input.latency_us < 1_000, // < 1 ms = < 1000 µs
        auditable: input.requires_audit,
        local: !input.has_external_calls,
    }
}

/// Convenience function: returns `true` iff the domain passes all five conditions.
pub fn is_compile_eligible(input: &CompileEligibleInput) -> bool {
    check(input).is_eligible()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MYCIN domain: stable rule base, finite organism set, <1ms,
    /// auditable CF arithmetic, no external calls. All five conditions pass.
    #[test]
    fn mycin_domain_is_compile_eligible() {
        let input = CompileEligibleInput {
            distribution_stable: true,
            output_space_finite: true,
            latency_us: 20, // infer_fast is ~20 ns = 0.02 µs
            requires_audit: true,
            has_external_calls: false,
        };
        let result = check(&input);
        assert!(
            result.is_eligible(),
            "MYCIN should be compile-eligible: {:?}",
            result
        );
        assert!(is_compile_eligible(&input));
    }

    /// Streaming LLM domain: distribution shifts continuously (not stable),
    /// output space unbounded (not finite), latency >> 1ms, no bit-exact replay,
    /// requires external API calls. All five conditions fail.
    #[test]
    fn streaming_llm_fails_all_conditions() {
        let input = CompileEligibleInput {
            distribution_stable: false,
            output_space_finite: false,
            latency_us: 500_000, // 500 ms typical LLM round-trip
            requires_audit: false,
            has_external_calls: true,
        };
        let result = check(&input);
        assert!(!result.is_eligible(), "Streaming LLM must not be compile-eligible");
        assert!(!result.stable);
        assert!(!result.finite_output);
        assert!(!result.fast);
        assert!(!result.auditable);
        assert!(!result.local);
    }

    /// Borderline case: stable + finite + auditable + local, but latency is
    /// exactly 1000 µs (not strictly less than 1 ms). Fast(D) fails.
    /// This exercises the < 1000 boundary precisely.
    #[test]
    fn borderline_latency_boundary_fails_fast_condition() {
        let input = CompileEligibleInput {
            distribution_stable: true,
            output_space_finite: true,
            latency_us: 1_000, // exactly 1 ms — NOT < 1 ms
            requires_audit: true,
            has_external_calls: false,
        };
        let result = check(&input);
        assert!(!result.is_eligible(), "Exactly 1ms should fail Fast(D)");
        assert!(result.stable, "Stable should still pass");
        assert!(result.finite_output, "Finite should still pass");
        assert!(!result.fast, "Fast must fail at exactly 1000 µs");
        assert!(result.auditable, "Auditable should still pass");
        assert!(result.local, "Local should still pass");
    }

    /// ELIZA domain: ~5 ns, stable rule table, finite template set, auditable,
    /// no external calls — verify per-field correctness explicitly.
    #[test]
    fn eliza_domain_check_fields() {
        let input = CompileEligibleInput {
            distribution_stable: true,
            output_space_finite: true,
            latency_us: 0, // 5 ns rounds to 0 µs
            requires_audit: true,
            has_external_calls: false,
        };
        let r = check(&input);
        assert_eq!(
            r,
            CompileEligibleCheck {
                stable: true,
                finite_output: true,
                fast: true,
                auditable: true,
                local: true,
            }
        );
    }
}
