use crate::fuse_hearsay::blackboard::Blackboard;
use crate::fuse_hearsay::fusion_rule::FusionRule;
use crate::fuse_hearsay::result::{FusionResult, FusionStatus};
use crate::fuse_hearsay::witness::FusionWitnessId;
use insa_instinct::{HearsayByte, InstinctByte, KappaByte};
use insa_types::FieldMask;

pub struct FuseHearsay;

impl FuseHearsay {
    pub fn fuse(board: &Blackboard, rule: &FusionRule) -> FusionResult {
        let mut detail = HearsayByte::empty();
        let mut emits = InstinctByte::empty();

        let missing = (board.present.0 & rule.required_sources.0 .0) ^ rule.required_sources.0 .0;
        let conflicts = board.conflicted.0 & rule.conflict_mask.0 .0;

        if missing != 0 {
            detail = detail.union(HearsayByte::SOURCE_MISSING);
            emits = emits.union(InstinctByte::RETRIEVE).union(InstinctByte::ASK);
        }

        if conflicts != 0 {
            detail = detail.union(HearsayByte::SOURCE_CONFLICTS);
            emits = emits
                .union(InstinctByte::INSPECT)
                .union(InstinctByte::ESCALATE);
        }

        if board.stale.0 != 0 {
            detail = detail.union(HearsayByte::SOURCE_STALE);
            emits = emits
                .union(InstinctByte::AWAIT)
                .union(InstinctByte::RETRIEVE);
        }

        let is_complete = missing == 0 && conflicts == 0 && board.stale.0 == 0;

        let status = if is_complete {
            detail = detail
                .union(HearsayByte::FUSION_COMPLETE)
                .union(HearsayByte::SOURCE_AGREES);
            emits = emits.union(InstinctByte::SETTLE);
            FusionStatus::Complete
        } else {
            if conflicts != 0 {
                detail = detail.union(HearsayByte::FUSION_REQUIRES_INSPECTION);
            }
            FusionStatus::Incomplete
        };

        FusionResult {
            status,
            detail,
            kappa: KappaByte::FUSE,
            emits,
            agreed: FieldMask(board.present.0 & !board.conflicted.0),
            conflicted: FieldMask(conflicts),
            missing: FieldMask(missing),
            stale: board.stale,
            witness_index: FusionWitnessId(0), // Mocked for now
        }
    }
}
