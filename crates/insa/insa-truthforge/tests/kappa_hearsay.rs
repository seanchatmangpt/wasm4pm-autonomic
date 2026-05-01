use insa_instinct::{HearsayByte, InstinctByte};
use insa_kappa8::fuse_hearsay::*;
use insa_types::FieldMask;

#[test]
fn test_hearsay_fusion_complete() {
    let mut board = Blackboard::default();
    board.present = FieldMask(0b111);

    let rule = FusionRule {
        id: RuleId(1),
        required_sources: RequiredMask(FieldMask(0b111)),
        conflict_mask: ConflictMask(FieldMask(0b111)),
        authority_required: AuthorityMask(FieldMask(0)),
        emits_on_fail: InstinctByte::empty(),
    };

    let res = FuseHearsay::fuse(&board, &rule);
    assert_eq!(res.status, FusionStatus::Complete);
    assert!(res.detail.contains(HearsayByte::FUSION_COMPLETE));
    assert!(res.detail.contains(HearsayByte::SOURCE_AGREES));
    assert!(res.emits.contains(InstinctByte::SETTLE));
}

#[test]
fn test_hearsay_fusion_conflict_and_missing() {
    let mut board = Blackboard::default();
    board.present = FieldMask(0b011);
    board.conflicted = FieldMask(0b010);

    let rule = FusionRule {
        id: RuleId(1),
        required_sources: RequiredMask(FieldMask(0b111)),
        conflict_mask: ConflictMask(FieldMask(0b111)),
        authority_required: AuthorityMask(FieldMask(0)),
        emits_on_fail: InstinctByte::empty(),
    };

    let res = FuseHearsay::fuse(&board, &rule);
    assert_eq!(res.status, FusionStatus::Incomplete);
    assert!(res.detail.contains(HearsayByte::SOURCE_MISSING));
    assert!(res.detail.contains(HearsayByte::SOURCE_CONFLICTS));
    assert!(res.detail.contains(HearsayByte::FUSION_REQUIRES_INSPECTION));

    assert!(res.emits.contains(InstinctByte::RETRIEVE));
    assert!(res.emits.contains(InstinctByte::ASK));
    assert!(res.emits.contains(InstinctByte::INSPECT));
    assert!(res.emits.contains(InstinctByte::ESCALATE));
}
