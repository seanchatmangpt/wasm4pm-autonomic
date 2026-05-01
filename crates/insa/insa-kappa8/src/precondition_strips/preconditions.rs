use insa_types::FieldMask;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequiredMask(pub FieldMask);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ForbiddenMask(pub FieldMask);
