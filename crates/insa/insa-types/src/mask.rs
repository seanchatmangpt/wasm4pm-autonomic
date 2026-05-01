#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FieldMask(pub u64);

impl FieldMask {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
    #[inline(always)]
    pub const fn with_bit(self, bit: FieldBit) -> Self {
        Self(self.0 | (1 << bit.get()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CompletedMask(pub u64);

impl CompletedMask {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
    #[inline(always)]
    pub const fn with_bit(self, bit: FieldBit) -> Self {
        Self(self.0 | (1 << bit.get()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FieldBit(u8);

impl FieldBit {
    #[inline(always)]
    pub const fn new_checked(value: u8) -> Result<Self, &'static str> {
        if value < 64 {
            Ok(Self(value))
        } else {
            Err("FieldBit must be in range [0, 63]")
        }
    }
    #[inline(always)]
    pub const fn new_unchecked(value: u8) -> Self {
        Self(value)
    }
    #[inline(always)]
    pub const fn get(self) -> u8 {
        self.0
    }
}
