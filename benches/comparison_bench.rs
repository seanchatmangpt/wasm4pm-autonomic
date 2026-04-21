use divan::black_box;
use dteam::utils::dense_kernel::PackedKeyTable;
use rustc_hash::FxHashMap;
use std::collections::HashMap;

fn main() {
    divan::main();
}

const SCALE_MIN: u64 = 1;
const SCALE_STANDARD: u64 = 1000;
const SCALE_MAX: u64 = 10000;

#[divan::bench]
fn bench_packed_key_table_min() {
    let mut table = PackedKeyTable::new();
    for i in 0..SCALE_MIN {
        table.insert(i, i, i);
    }
    for i in 0..SCALE_MIN {
        black_box(table.get(black_box(i)));
    }
}

#[divan::bench]
fn bench_packed_key_table_standard() {
    let mut table = PackedKeyTable::new();
    for i in 0..SCALE_STANDARD {
        table.insert(i, i, i);
    }
    for i in 0..SCALE_STANDARD {
        black_box(table.get(black_box(i)));
    }
}

#[divan::bench]
fn bench_packed_key_table_max() {
    let mut table = PackedKeyTable::new();
    for i in 0..SCALE_MAX {
        table.insert(i, i, i);
    }
    for i in 0..SCALE_MAX {
        black_box(table.get(black_box(i)));
    }
}

#[divan::bench]
fn bench_fxhashmap_min() {
    let mut map = FxHashMap::default();
    for i in 0..SCALE_MIN {
        map.insert(i, i);
    }
    for i in 0..SCALE_MIN {
        black_box(map.get(black_box(&i)));
    }
}

#[divan::bench]
fn bench_fxhashmap_standard() {
    let mut map = FxHashMap::default();
    for i in 0..SCALE_STANDARD {
        map.insert(i, i);
    }
    for i in 0..SCALE_STANDARD {
        black_box(map.get(black_box(&i)));
    }
}

#[divan::bench]
fn bench_fxhashmap_max() {
    let mut map = FxHashMap::default();
    for i in 0..SCALE_MAX {
        map.insert(i, i);
    }
    for i in 0..SCALE_MAX {
        black_box(map.get(black_box(&i)));
    }
}

#[divan::bench]
fn bench_std_hashmap_min() {
    let mut map = HashMap::new();
    for i in 0..SCALE_MIN {
        map.insert(i, i);
    }
    for i in 0..SCALE_MIN {
        black_box(map.get(black_box(&i)));
    }
}

#[divan::bench]
fn bench_std_hashmap_standard() {
    let mut map = HashMap::new();
    for i in 0..SCALE_STANDARD {
        map.insert(i, i);
    }
    for i in 0..SCALE_STANDARD {
        black_box(map.get(black_box(&i)));
    }
}

#[divan::bench]
fn bench_std_hashmap_max() {
    let mut map = HashMap::new();
    for i in 0..SCALE_MAX {
        map.insert(i, i);
    }
    for i in 0..SCALE_MAX {
        black_box(map.get(black_box(&i)));
    }
}
