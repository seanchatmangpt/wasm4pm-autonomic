import sys

# Fix engine.rs
with open("../insa/insa-kappa8/src/reconstruct_dendral/engine.rs", "r") as f:
    content = f.read()

content = content.replace("dendral: dendral.union", "detail: dendral.union")
content = content.replace("res.dendral", "res.detail")
content = content.replace("use crate::reconstruct_dendral::witness::{ReconstructionWitness, ReconstructionWitnessId};", "use crate::reconstruct_dendral::witness::ReconstructionWitnessId;")

with open("../insa/insa-kappa8/src/reconstruct_dendral/engine.rs", "w") as f:
    f.write(content)

# Fix prove_prolog/fixtures.rs
with open("../insa/insa-kappa8/src/prove_prolog/fixtures.rs", "r") as f:
    content = f.read()
content = content.replace("use crate::prove_prolog::clause::{FactRow, HornClause, ProofGoal, RelationId, TermId};", "use crate::prove_prolog::clause::{FactRow, HornClause, RelationId, TermId};")
with open("../insa/insa-kappa8/src/prove_prolog/fixtures.rs", "w") as f:
    f.write(content)

# Fix reflect_eliza/pattern.rs
with open("../insa/insa-kappa8/src/reflect_eliza/pattern.rs", "r") as f:
    content = f.read()
content = content.replace("use insa_types::{CompletedMask, FieldMask};", "use insa_types::FieldMask;")
with open("../insa/insa-kappa8/src/reflect_eliza/pattern.rs", "w") as f:
    f.write(content)
