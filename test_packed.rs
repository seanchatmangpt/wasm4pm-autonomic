use dteam::utils::dense_kernel::PackedKeyTable;

fn main() {
    let mut t = PackedKeyTable::new();
    t.insert(1, 1, 1);
    println!("{:?}", t.get(1));
}
