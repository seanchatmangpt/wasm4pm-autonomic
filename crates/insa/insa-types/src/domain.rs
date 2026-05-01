#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ObjectRef(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PolicyEpoch(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DictionaryDigest(pub [u8; 32]);

impl Default for DictionaryDigest {
    fn default() -> Self { Self([0; 32]) }
}
