use insa_hotpath::cog8::Cog8Row;
use insa_proof::powl64::Powl64RouteCell;
use std::mem::{size_of, align_of};

#[test]
fn gate_cog8row_layout_exactly_32_bytes_aligned() {
    assert_eq!(size_of::<Cog8Row>(), 32, "Cog8Row must be exactly 32 bytes to fit two per L1 cache line.");
    assert_eq!(align_of::<Cog8Row>(), 32, "Cog8Row must be 32-byte aligned.");
}

#[test]
fn gate_powl64cell_layout_exactly_64_bytes_aligned() {
    assert_eq!(size_of::<Powl64RouteCell>(), 64, "Powl64RouteCell must be exactly 64 bytes to match proof spine laws.");
    assert_eq!(align_of::<Powl64RouteCell>(), 64, "Powl64RouteCell must be 64-byte aligned.");
}
