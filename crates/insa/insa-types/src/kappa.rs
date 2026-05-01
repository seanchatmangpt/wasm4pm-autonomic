//! KappaByte definitions.

/// A byte representing the cognitive collapse attribution.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct KappaByte(pub u8);

impl KappaByte {
    /// Reflect (ELIZA): clarify, restate, slow down premature action
    pub const REFLECT: Self = Self(1 << 0);
    /// Precondition (STRIPS): check whether action is allowed
    pub const PRECONDITION: Self = Self(1 << 1);
    /// Ground (SHRDLU): bind words/forms/docs to exact objects
    pub const GROUND: Self = Self(1 << 2);
    /// Prove (Prolog): prove eligibility, authority, relation, dependency
    pub const PROVE: Self = Self(1 << 3);
    /// Rule (MYCIN): apply expert/domain rules
    pub const RULE: Self = Self(1 << 4);
    /// Reconstruct (DENDRAL): infer hidden structure from fragments
    pub const RECONSTRUCT: Self = Self(1 << 5);
    /// Fuse (HEARSAY-II): combine multiple evidence sources
    pub const FUSE: Self = Self(1 << 6);
    /// ReduceGap (GPS): compute gap between current and required state
    pub const REDUCE_GAP: Self = Self(1 << 7);
}
