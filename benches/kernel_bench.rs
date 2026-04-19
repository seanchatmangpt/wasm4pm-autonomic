use divan::black_box;
use dteam::utils::dense_kernel::{K64, PackedKeyTable, DenseIndex, NodeKind};

fn main() {
    divan::main();
}

#[divan::bench]
fn kbitset_contains_all() -> bool {
    let mut a = K64::zero();
    // Setting bits corresponding to 0xAAAAAAAAAAAAAAAA
    for i in (0..64).step_by(2) { a.set(i).unwrap(); }
    
    let mut b = K64::zero();
    // Setting bits corresponding to 0x8888888888888888
    for i in (0..64).step_by(4) { b.set(i).unwrap(); }
    
    black_box(a).contains_all(black_box(b))
}

#[divan::bench]
fn kbitset_missing_count() -> u32 {
    let mut a = K64::zero();
    for i in (0..64).step_by(2) { a.set(i).unwrap(); }
    
    let mut b = K64::zero();
    for i in 16..48 { b.set(i).unwrap(); }

    black_box(a).missing_count(black_box(b))
}

#[divan::bench]
fn packed_key_table_get() -> Option<Vec<f32>> {
    let mut table = PackedKeyTable::new();
    for i in 0..100 {
        table.insert(i as u64, i, vec![i as f32; 3]);
    }
    black_box(&table).get(black_box(50)).cloned()
}

#[divan::bench]
fn dense_index_dense_id() -> Option<u32> {
    let symbols = vec![
        ("start".to_string(), NodeKind::Place),
        ("task1".to_string(), NodeKind::Transition),
        ("end".to_string(), NodeKind::Place),
    ];
    let index = DenseIndex::compile(symbols).unwrap();
    black_box(&index).dense_id(black_box("task1"))
}
