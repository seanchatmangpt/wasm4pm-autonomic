use insa_instinct::{ElizaByte, InstinctByte};
use insa_types::{CompletedMask, FieldMask};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PatternId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TemplateId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AskKind {
    #[default]
    None = 0,
    Clarify = 1,
    MissingSlot = 2,
    ConfirmIntent = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReflectPattern {
    pub id: PatternId,
    pub required_context: FieldMask,
    pub template_id: TemplateId,
    pub emits: InstinctByte,
    pub eliza_detail: ElizaByte,
    pub ask_kind: AskKind,
    pub _padding: [u8; 3],
}
