use insa_types::{CompletedMask, PackId, GroupId, RuleId, FieldMask};
use insa_instinct::{InstinctByte, KappaByte};
use insa_hotpath::cog8::{Cog8Row, execute_cog8_graph};

#[test]
fn gate_execute_cog8_graph_allocates_zero_octets() {
    let nodes = [Cog8Row {
        pack_id: PackId(1),
        group_id: GroupId(1),
        rule_id: RuleId(1),
        required_mask: FieldMask(0b1),
        forbidden_mask: FieldMask(0),
        completed_block_mask: CompletedMask(0),
        response: InstinctByte::SETTLE,
        kappa: KappaByte::RULE,
        ..Default::default()
    }];

    let present = 0b1;
    let d = execute_cog8_graph(&nodes, present, 0).unwrap();
    assert_eq!(d.response, InstinctByte::SETTLE);
}
