//! Static, `Copy` identifier newtypes for pack/rule/group/obligation ids.
//!
//! Phase 8.1 — these replace `String`/`Option<String>` ids on the runtime
//! decision path so [`crate::packs::PackDecision`] can be `Copy`-friendly
//! and allocation-free in the rule-firing loop.
//!
//! All four newtypes wrap `&'static str`. They are `Copy` (no clone cost),
//! `repr(transparent)` (no layout overhead), and serialize as plain strings
//! so on-disk evidence/scorecard JSON is unchanged.
//!
//! ## Identifier lifetime contract
//!
//! Static ids come from one of three sources:
//!
//! 1. **Compile-time literals** — packs that build their rule tables in
//!    Rust (e.g. `lifestyle_overlap`) call `RuleId::new("lifestyle.x.y")`
//!    with a `&'static str` literal. Zero allocation.
//! 2. **Const-friendly construction** — `pub const fn new` allows ids to
//!    appear in `const` and `static` contexts.
//! 3. **Load-time interning** — packs loaded from JSON arrive with
//!    `String` ids. The single supported promotion path lives in
//!    [`crate::packs::intern`] (`Box::leak` once-per-id, documented as a
//!    cold load-time boundary). The hot path NEVER promotes a `String`.
//!
//! ## What this module is not
//!
//! Not an interner. The leak-once helper lives next to `load_compiled` so
//! the contract — leakable only at load-time — stays visible to the
//! caller. This module's only job is the strongly-typed wrapper.

use serde::{Deserialize, Serialize};

macro_rules! static_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub &'static str);

        impl Default for $name {
            #[inline]
            fn default() -> Self {
                Self("")
            }
        }

        impl $name {
            /// Construct from a `&'static str`. `const`-callable.
            #[inline]
            #[must_use]
            pub const fn new(s: &'static str) -> Self {
                Self(s)
            }

            /// Borrow the underlying static string.
            #[inline]
            #[must_use]
            pub const fn as_str(&self) -> &'static str {
                self.0
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.0
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.0)
            }
        }

        impl PartialEq<&str> for $name {
            #[inline]
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<str> for $name {
            #[inline]
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }
    };
}

static_id!(PackId, "Pack-level identifier (e.g. `lifestyle.overlap.v30_1_1`).");
static_id!(GroupId, "Precedence-group identifier (e.g. `lifestyle.safety`).");
static_id!(RuleId, "Rule identifier (e.g. `lifestyle.capacity.fatigue_softens_routine`).");
static_id!(
    ObligationId,
    "Obligation identifier (Phase 8.2; reserved here so packs can author obligations against pack/rule/group ids without a circular dep)."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_copy_repr_transparent() {
        // Copy semantics: assigning is not a move, no clone needed.
        const RULE: RuleId = RuleId::new("rule.x");
        let a = RULE;
        let b = RULE;
        assert_eq!(a, b);
        // repr(transparent) means size_of equals &str.
        assert_eq!(std::mem::size_of::<RuleId>(), std::mem::size_of::<&'static str>());
        assert_eq!(std::mem::size_of::<Option<RuleId>>(), std::mem::size_of::<&'static str>());
    }

    #[test]
    fn ids_compare_to_str_literals() {
        let r = RuleId::new("rule.x");
        assert!(r == "rule.x");
        assert!(r != "rule.y");
        assert_eq!(r.as_str(), "rule.x");
    }

    #[test]
    fn ids_serialize_as_plain_strings() {
        let p = PackId::new("test.pack");
        let s = serde_json::to_string(&p).unwrap();
        assert_eq!(s, "\"test.pack\"");
        let back: PackId = serde_json::from_str("\"test.pack\"").unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn display_writes_underlying_str() {
        let g = GroupId::new("lifestyle.safety");
        assert_eq!(format!("{g}"), "lifestyle.safety");
    }
}
