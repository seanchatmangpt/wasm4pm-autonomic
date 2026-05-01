#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RelationId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TermId(pub u16);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FactRow {
    pub relation: RelationId,
    pub subject: TermId,
    pub object: TermId,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HornClause {
    pub id: u16,
    pub head: RelationId,
    pub body1: RelationId,
    pub body2: RelationId, // 0 if none
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProofGoal {
    pub relation: RelationId,
    pub subject: TermId,
    pub object: TermId,
}
