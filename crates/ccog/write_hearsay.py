import os

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

# Patch HearsayByte
byte_path = '../insa/insa-instinct/src/byte.rs'
with open(byte_path, 'r') as f:
    byte_content = f.read()

bits_method = """
    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
"""

if "pub const fn bits(self) -> u8" not in byte_content.split("impl HearsayByte {")[1].split("impl GpsByte {")[0]:
    byte_content = byte_content.replace(
"""impl HearsayByte {
    pub const SOURCE_AGREES: Self = Self(1 << 0);""",
"""impl HearsayByte {
    pub const SOURCE_AGREES: Self = Self(1 << 0);""" + bits_method)
    with open(byte_path, 'w') as f:
        f.write(byte_content)


mod_rs = """pub mod slot;
pub mod source;
pub mod freshness;
pub mod fusion_rule;
pub mod blackboard;
pub mod engine;
pub mod result;
pub mod witness;

pub use slot::*;
pub use source::*;
pub use freshness::*;
pub use fusion_rule::*;
pub use blackboard::*;
pub use engine::*;
pub use result::*;
pub use witness::*;
"""

slot_rs = """use insa_types::{FieldMask, ObjectRef};
use crate::fuse_hearsay::source::{SourceId, AuthorityByte};
use crate::fuse_hearsay::freshness::FreshnessByte;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SlotId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EvidenceKind(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DigestRef(pub u64);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EvidenceSlot {
    pub id: SlotId,
    pub source: SourceId,
    pub object: ObjectRef,
    pub kind: EvidenceKind,
    pub asserts: FieldMask,
    pub freshness: FreshnessByte,
    pub authority: AuthorityByte,
    pub digest: DigestRef,
}
"""

source_rs = """#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SourceId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AuthorityByte(pub u8);

impl AuthorityByte {
    pub const WEAK: Self = Self(0);
    pub const STANDARD: Self = Self(1);
    pub const SYSTEM_OF_RECORD: Self = Self(2);
    pub const OVERRIDE: Self = Self(3);
}
"""

freshness_rs = """#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FreshnessByte(pub u8);

impl FreshnessByte {
    pub const STALE: Self = Self(0);
    pub const FRESH: Self = Self(1);
    pub const LIVE: Self = Self(2);
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Epoch(pub u64);
"""

fusion_rule_rs = """use insa_types::FieldMask;
use insa_instinct::InstinctByte;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RuleId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequiredMask(pub FieldMask);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConflictMask(pub FieldMask);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuthorityMask(pub FieldMask);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FusionRule {
    pub id: RuleId,
    pub required_sources: RequiredMask,
    pub conflict_mask: ConflictMask,
    pub authority_required: AuthorityMask,
    pub emits_on_fail: InstinctByte,
}
"""

blackboard_rs = """use insa_types::FieldMask;
use crate::fuse_hearsay::slot::EvidenceSlot;

/// A bounded blackboard for evidence fusion.
/// Hardcoded to 16 slots to guarantee bounded execution.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blackboard {
    pub slots: [EvidenceSlot; 16],
    pub len: u8,
    pub present: FieldMask,
    pub missing: FieldMask,
    pub conflicted: FieldMask,
    pub stale: FieldMask,
}

impl Default for Blackboard {
    fn default() -> Self {
        Self {
            slots: [EvidenceSlot::default(); 16],
            len: 0,
            present: FieldMask(0),
            missing: FieldMask(0),
            conflicted: FieldMask(0),
            stale: FieldMask(0),
        }
    }
}

impl Blackboard {
    pub fn push(&mut self, slot: EvidenceSlot) -> Result<(), &'static str> {
        if self.len < 16 {
            self.slots[self.len as usize] = slot;
            self.len += 1;
            Ok(())
        } else {
            Err("Blackboard full")
        }
    }
}
"""

engine_rs = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, KappaByte, HearsayByte};
use crate::fuse_hearsay::fusion_rule::FusionRule;
use crate::fuse_hearsay::blackboard::Blackboard;
use crate::fuse_hearsay::result::{FusionResult, FusionStatus};
use crate::fuse_hearsay::witness::FusionWitnessId;

pub struct FuseHearsay;

impl FuseHearsay {
    pub fn fuse(board: &Blackboard, rule: &FusionRule) -> FusionResult {
        let mut detail = HearsayByte::empty();
        let mut emits = InstinctByte::empty();
        
        let missing = (board.present.0 & rule.required_sources.0.0) ^ rule.required_sources.0.0;
        let conflicts = board.conflicted.0 & rule.conflict_mask.0.0;
        
        if missing != 0 {
            detail = detail.union(HearsayByte::SOURCE_MISSING);
            emits = emits.union(InstinctByte::RETRIEVE).union(InstinctByte::ASK);
        }
        
        if conflicts != 0 {
            detail = detail.union(HearsayByte::SOURCE_CONFLICTS);
            emits = emits.union(InstinctByte::INSPECT).union(InstinctByte::ESCALATE);
        }
        
        if board.stale.0 != 0 {
            detail = detail.union(HearsayByte::SOURCE_STALE);
            emits = emits.union(InstinctByte::AWAIT).union(InstinctByte::RETRIEVE);
        }

        let is_complete = missing == 0 && conflicts == 0 && board.stale.0 == 0;
        
        let status = if is_complete {
            detail = detail.union(HearsayByte::FUSION_COMPLETE).union(HearsayByte::SOURCE_AGREES);
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
"""

result_rs = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, KappaByte, HearsayByte};
use crate::fuse_hearsay::witness::FusionWitnessId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FusionStatus {
    Complete = 0,
    Incomplete = 1,
}

impl Default for FusionStatus {
    fn default() -> Self {
        Self::Incomplete
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FusionResult {
    pub status: FusionStatus,
    pub detail: HearsayByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub agreed: FieldMask,
    pub conflicted: FieldMask,
    pub missing: FieldMask,
    pub stale: FieldMask,
    pub witness_index: FusionWitnessId,
}
"""

witness_rs = """#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FusionWitnessId(pub u64);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FusionWitness {
    pub id: FusionWitnessId,
}
"""

test_content = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, HearsayByte};
use insa_kappa8::fuse_hearsay::*;

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
"""

# Remove old single file if it exists
if os.path.exists('../insa/insa-kappa8/src/fuse_hearsay.rs'):
    os.remove('../insa/insa-kappa8/src/fuse_hearsay.rs')

write_file('../insa/insa-kappa8/src/fuse_hearsay/mod.rs', mod_rs)
write_file('../insa/insa-kappa8/src/fuse_hearsay/slot.rs', slot_rs)
write_file('../insa/insa-kappa8/src/fuse_hearsay/source.rs', source_rs)
write_file('../insa/insa-kappa8/src/fuse_hearsay/freshness.rs', freshness_rs)
write_file('../insa/insa-kappa8/src/fuse_hearsay/fusion_rule.rs', fusion_rule_rs)
write_file('../insa/insa-kappa8/src/fuse_hearsay/blackboard.rs', blackboard_rs)
write_file('../insa/insa-kappa8/src/fuse_hearsay/engine.rs', engine_rs)
write_file('../insa/insa-kappa8/src/fuse_hearsay/result.rs', result_rs)
write_file('../insa/insa-kappa8/src/fuse_hearsay/witness.rs', witness_rs)

write_file('../insa/insa-truthforge/tests/kappa_hearsay.rs', test_content)

print("Fuse / HEARSAY-II pack successfully generated.")
