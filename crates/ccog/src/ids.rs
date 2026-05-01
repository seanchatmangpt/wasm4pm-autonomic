//! Stable Rust Core Data Model — Numeric Identifiers (PRD v0.4, v0.8).

use serde::{Deserialize, Serialize};

macro_rules! numeric_id {
    ($name:ident, $inner:ty, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Debug,
            Default,
            Hash,
            Serialize,
            Deserialize,
        )]
        pub struct $name(pub $inner);

        impl $name {
            /// Sentinel for no value.
            pub const NONE: Self = Self(0);
        }

        impl From<$inner> for $name {
            #[inline]
            fn from(v: $inner) -> Self {
                Self(v)
            }
        }

        impl From<$name> for $inner {
            #[inline]
            fn from(v: $name) -> Self {
                v.0
            }
        }

        impl From<$name> for usize {
            #[inline]
            fn from(v: $name) -> Self {
                v.0 as usize
            }
        }

        impl core::ops::Deref for $name {
            type Target = $inner;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl AsRef<$inner> for $name {
            #[inline]
            fn as_ref(&self) -> &$inner {
                &self.0
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

numeric_id!(PackId, u16, "Pack-level identifier.");
numeric_id!(GroupId, u16, "Precedence-group identifier.");
numeric_id!(RuleId, u16, "Rule identifier.");
numeric_id!(
    FieldId,
    u16,
    "Field variable identifier (bit position or dense index)."
);
numeric_id!(BreedId, u8, "Cognitive breed identifier.");
numeric_id!(ObligationId, u16, "Obligation identifier.");
numeric_id!(NodeId, u16, "Graph node identifier.");
numeric_id!(EdgeId, u16, "Graph edge identifier.");
numeric_id!(AgentId, u16, "Autonomous agent identifier.");
numeric_id!(HumanRoleId, u16, "Human role identifier.");
numeric_id!(ToolId, u16, "Model context protocol tool identifier.");

/// Cognitive function that collapsed the field.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CollapseFn {
    /// No collapse attribution.
    #[default]
    None = 0,
    /// ELIZA: reflective conversational posture.
    ReflectivePosture = 1,
    /// MYCIN: rule-based expert inference.
    ExpertRule = 2,
    /// STRIPS: preconditions and legal actions.
    Preconditions = 3,
    /// SHRDLU: grounded signals to objects.
    Grounding = 4,
    /// Prolog: symbolic logic and relations.
    RelationalProof = 5,
    /// DENDRAL: structural reconstruction from fragments.
    Reconstruction = 6,
    /// HEARSAY-II: blackboard fusion.
    BlackboardFusion = 7,
    /// GPS: means-ends analysis (goal gaps).
    DifferenceReduction = 8,
    /// SOAR: chunking and production rules.
    Chunking = 9,
    /// PRS: belief-desire-intention reactive planning.
    ReactiveIntention = 10,
    /// CBR: case-based reasoning and analogy.
    CaseAnalogy = 11,
}
