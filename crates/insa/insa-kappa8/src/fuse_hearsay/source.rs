#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SourceId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AuthorityByte(pub u8);

impl AuthorityByte {
    pub const WEAK: Self = Self(0);
    pub const STANDARD: Self = Self(1);
    pub const SYSTEM_OF_RECORD: Self = Self(2);
    pub const OVERRIDE: Self = Self(3);
}
