use crate::prove_prolog::clause::{FactRow, HornClause, ProofGoal, RelationId, TermId};

pub fn sample_facts() -> [FactRow; 2] {
    [
        FactRow {
            relation: RelationId(1),
            subject: TermId(10),
            object: TermId(20),
        }, // 10 is parent of 20
        FactRow {
            relation: RelationId(2),
            subject: TermId(20),
            object: TermId(30),
        }, // 20 is parent of 30
    ]
}

pub fn sample_clause() -> HornClause {
    HornClause {
        id: 1,
        head: RelationId(3), // Grandparent
        body1: RelationId(1),
        body2: RelationId(2),
    }
}
