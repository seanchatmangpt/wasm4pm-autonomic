use crate::reconstruct_dendral::fragment::ReconstructionRule;
use crate::reconstruct_dendral::result::{DendralResult, DendralStatus};
use insa_instinct::{DendralByte, InstinctByte, KappaByte};
use insa_types::FieldMask;

pub struct ReconstructDendral;

impl ReconstructDendral {
    pub fn evaluate(rules: &[ReconstructionRule], fragments_present: FieldMask) -> DendralResult {
        let mut detail = DendralByte::empty();
        let mut emits = InstinctByte::empty();

        let mut valid_hypotheses = 0;

        for rule in rules {
            let missing =
                (fragments_present.0 & rule.fragments_required.0) ^ rule.fragments_required.0;
            let forbidden = fragments_present.0 & rule.constraints_forbidden.0;

            if missing != 0 {
                detail = detail.union(DendralByte::MISSING_FRAGMENT);
            }
            if forbidden != 0 {
                detail = detail
                    .union(DendralByte::CONSTRAINT_VIOLATION)
                    .union(DendralByte::CANDIDATE_PRUNED);
            }

            if missing == 0 && forbidden == 0 {
                valid_hypotheses += 1;
                detail = detail
                    .union(DendralByte::FRAGMENTS_SUFFICIENT)
                    .union(DendralByte::CANDIDATE_GENERATED);
                emits = emits.union(rule.emits);
            }
        }

        if valid_hypotheses == 1 {
            detail = detail.union(DendralByte::UNIQUE_RECONSTRUCTION);
            emits = emits.union(InstinctByte::SETTLE);
            DendralResult {
                status: DendralStatus::Unique,
                detail,
                kappa: KappaByte::RECONSTRUCT,
                emits,
            }
        } else if valid_hypotheses > 1 {
            detail = detail
                .union(DendralByte::MULTIPLE_RECONSTRUCTIONS)
                .union(DendralByte::RECONSTRUCTION_UNSTABLE);
            emits = emits.union(InstinctByte::INSPECT).union(InstinctByte::ASK);
            DendralResult {
                status: DendralStatus::Ambiguous,
                detail,
                kappa: KappaByte::RECONSTRUCT,
                emits,
            }
        } else {
            emits = emits.union(InstinctByte::RETRIEVE).union(InstinctByte::ASK);
            DendralResult {
                status: DendralStatus::Failed,
                detail,
                kappa: KappaByte::RECONSTRUCT,
                emits,
            }
        }
    }
}
