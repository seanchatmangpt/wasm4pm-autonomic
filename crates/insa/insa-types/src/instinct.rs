//! InstinctByte definitions.

/// A byte representing simultaneous activation of autonomic instincts.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct InstinctByte(pub u8);

impl InstinctByte {
    /// Settle: excess processing
    pub const SETTLE: Self = Self(1 << 0);
    /// Retrieve: fetch missing known evidence
    pub const RETRIEVE: Self = Self(1 << 1);
    /// Inspect: bounds ambiguity
    pub const INSPECT: Self = Self(1 << 2);
    /// Ask: request missing input
    pub const ASK: Self = Self(1 << 3);
    /// Await: wait for expected future event
    pub const AWAIT: Self = Self(1 << 4);
    /// Refuse: block unlawful action
    pub const REFUSE: Self = Self(1 << 5);
    /// Escalate: insufficient local authority
    pub const ESCALATE: Self = Self(1 << 6);
    /// Ignore: duplicate/noise/stale signal
    pub const IGNORE: Self = Self(1 << 7);

    /// Combines two instinct bytes.
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if this byte contains all bits of the other.
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Exposes the underlying bits.
    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

/// A selected instinct, mathematically constrained to be zero or one-hot.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SelectedInstinctByte(u8);

impl SelectedInstinctByte {
    /// Creates an empty selected instinct (no selection made).
    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Creates a new SelectedInstinctByte, ensuring it is one-hot.
    #[inline(always)]
    pub const fn onehot(bits: u8) -> Option<Self> {
        if bits.count_ones() == 1 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Decodes raw bytes (e.g. from .powl64), enforcing the one-hot or empty invariant.
    #[inline(always)]
    pub const fn decode(bits: u8) -> Option<Self> {
        if bits == 0 || bits.count_ones() == 1 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying byte.
    #[inline(always)]
    pub const fn get(self) -> u8 {
        self.0
    }
}
