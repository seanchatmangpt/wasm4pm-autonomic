use divan::black_box;
use dteam::utils::dense_kernel::PackedKeyTable;
use rustc_hash::FxHashMap;
use std::collections::HashMap;

fn main() {
    divan::main();
}

const SCALE: u64 = 1000;

#[divan::bench]
fn bench_packed_key_table() {
    let mut table = PackedKeyTable::new();
    for i in 0..SCALE {
        table.insert(i, i, i);
    }
    for i in 0..SCALE {
        black_box(table.get(black_box(i)));
    }
}

#[divan::bench]
fn bench_fxhashmap() {
    let mut map = FxHashMap::default();
    for i in 0..SCALE {
        map.insert(i, i);
    }
    for i in 0..SCALE {
        black_box(map.get(black_box(&i)));
    }
}

#[divan::bench]
fn bench_std_hashmap() {
    let mut map = HashMap::new();
    for i in 0..SCALE {
        map.insert(i, i);
    }
    for i in 0..SCALE {
        black_box(map.get(black_box(&i)));
    }
}
