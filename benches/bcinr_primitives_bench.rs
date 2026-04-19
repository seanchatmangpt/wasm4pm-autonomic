use divan::black_box;
use bcinr::int::popcount_u64;

fn main() {
    divan::main();
}

#[divan::bench]
fn fnv1a_hash() -> u64 {
    bcinr::sketch::fnv1a_64(black_box(b"activity_name"))
}

#[divan::bench]
fn bitset_popcount() -> u64 {
    popcount_u64(black_box(0xAAAAAAAA_BBBBBBBB))
}

#[divan::bench]
fn branchless_select() -> u64 {
    bcinr::mask::select_u64(black_box(u64::MAX), 0x1111, 0x2222)
}
