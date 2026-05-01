#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct KappaByte(pub u8);

impl KappaByte {
    pub const REFLECT: Self = Self(1 << 0);
    pub const PRECONDITION: Self = Self(1 << 1);
    pub const GROUND: Self = Self(1 << 2);
    pub const PROVE: Self = Self(1 << 3);
    pub const RULE: Self = Self(1 << 4);
    pub const RECONSTRUCT: Self = Self(1 << 5);
    pub const FUSE: Self = Self(1 << 6);
    pub const REDUCE_GAP: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct InstinctByte(pub u8);

impl InstinctByte {
    pub const SETTLE: Self = Self(1 << 0);
    pub const RETRIEVE: Self = Self(1 << 1);
    pub const INSPECT: Self = Self(1 << 2);
    pub const ASK: Self = Self(1 << 3);
    pub const AWAIT: Self = Self(1 << 4);
    pub const REFUSE: Self = Self(1 << 5);
    pub const ESCALATE: Self = Self(1 << 6);
    pub const IGNORE: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SelectedInstinctByte(pub u8);

impl SelectedInstinctByte {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn onehot(bits: u8) -> Self {
        if bits == 0 || bits.count_ones() == 1 {
            Self(bits)
        } else {
            Self(0)
        }
    }
    #[inline(always)]
    pub const fn new_decode(bits: u8) -> Option<Self> {
        if bits == 0 || bits.count_ones() == 1 {
            Some(Self(bits))
        } else {
            None
        }
    }
    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
}
