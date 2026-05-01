use insa_hotpath::cog8::Cog8Row;
use insa_proof::powl64::RouteCell64;
use insa_proof::wire::WirePowl64HeaderV1;
use memoffset::offset_of;
use std::mem::{align_of, size_of};

#[test]
fn gate_cog8row_layout_exactly_32_bytes_aligned() {
    assert_eq!(
        size_of::<Cog8Row>(),
        32,
        "Cog8Row must be exactly 32 bytes to fit two per L1 cache line."
    );
    assert_eq!(
        align_of::<Cog8Row>(),
        32,
        "Cog8Row must be 32-byte aligned."
    );
}

#[test]
fn gate_cog8row_offsets() {
    assert_eq!(
        offset_of!(Cog8Row, required_mask),
        0,
        "required_mask must be at offset 0"
    );
    assert_eq!(
        offset_of!(Cog8Row, forbidden_mask),
        8,
        "forbidden_mask must be at offset 8"
    );
    assert_eq!(
        offset_of!(Cog8Row, completed_block_mask),
        16,
        "completed_block_mask must be at offset 16"
    );
}

#[test]
fn gate_route_cell_64_layout_exactly_64_bytes_aligned() {
    assert_eq!(
        size_of::<RouteCell64>(),
        64,
        "RouteCell64 must be exactly 64 bytes."
    );
    assert_eq!(
        align_of::<RouteCell64>(),
        64,
        "RouteCell64 must be 64-byte aligned."
    );
}

#[test]
fn gate_wire_powl64_header_v1_layout() {
    assert_eq!(
        size_of::<WirePowl64HeaderV1>(),
        256,
        "WirePowl64HeaderV1 must be exactly 256 bytes."
    );
    // Ensure repr(C) is respected
}
