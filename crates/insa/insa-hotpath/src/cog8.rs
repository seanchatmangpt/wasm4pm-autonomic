//! COG8 semantic closure substrate.

use insa_instinct::{InstinctByte, KappaByte};
use insa_types::{CompletedMask, FieldMask, GroupId, PackId, RuleId};

/// A single atomic closure evaluation row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C, align(32))]
pub struct Cog8Row {
    pub required_mask: FieldMask,            // offset 0, 8 bytes
    pub forbidden_mask: FieldMask,           // offset 8, 8 bytes
    pub completed_block_mask: CompletedMask, // offset 16, 8 bytes

    pub pack_id: PackId,   // offset 24, 2 bytes
    pub group_id: GroupId, // offset 26, 2 bytes
    pub rule_id: RuleId,   // offset 28, 2 bytes

    pub response: InstinctByte, // offset 30, 1 byte
    pub kappa: KappaByte,       // offset 31, 1 byte
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cog8Decision {
    pub response: InstinctByte,
    pub kappa: KappaByte,
    pub fired_mask: u64,
    pub completed_mask: u64,
    pub matched_pack_id: Option<PackId>,
    pub matched_group_id: Option<GroupId>,
    pub matched_rule_id: Option<RuleId>,
}

impl Default for Cog8Decision {
    fn default() -> Self {
        Self {
            response: InstinctByte::empty(),
            kappa: KappaByte::empty(),
            fired_mask: 0,
            completed_mask: 0,
            matched_pack_id: None,
            matched_group_id: None,
            matched_rule_id: None,
        }
    }
}

#[inline(always)]
pub fn execute_cog8_graph(
    nodes: &[Cog8Row],
    present: u64,
    completed: u64,
) -> Result<Cog8Decision, &'static str> {
    let mut best = Cog8Decision {
        response: InstinctByte::empty(),
        completed_mask: completed,
        ..Default::default()
    };

    for (node_index, row) in nodes.iter().enumerate() {
        let m1 = (present & row.required_mask.0) ^ row.required_mask.0;
        let m2 = present & row.forbidden_mask.0;
        let m3 = (completed & row.completed_block_mask.0) ^ row.completed_block_mask.0;
        let matched = (m1 | m2 | m3) == 0;

        if matched {
            best.fired_mask |= 1 << (node_index as u64);
            best.response = row.response;
            best.matched_pack_id = Some(row.pack_id);
            best.matched_group_id = Some(row.group_id);
            best.matched_rule_id = Some(row.rule_id);
            best.kappa = row.kappa;
        }
    }

    Ok(best)
}
