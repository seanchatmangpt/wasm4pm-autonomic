#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FreshnessByte(pub u8);

impl FreshnessByte {
    pub const STALE: Self = Self(0);
    pub const FRESH: Self = Self(1);
    pub const LIVE: Self = Self(2);
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Epoch(pub u64);
