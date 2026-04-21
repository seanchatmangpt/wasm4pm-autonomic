fn branchless_search(arr: &[(u64, i32, i32)], hash: u64) -> Option<usize> {
    let mut base = 0;
    let mut len = arr.len();
    while len > 1 {
        let half = len / 2;
        let mid = base + half;
        // SAFETY: mid is always < arr.len()
        let val = unsafe { arr.get_unchecked(mid).0 };
        let cmp = (val <= hash) as usize;
        base += cmp * half;
        len -= half;
    }
    if len > 0 && arr[base].0 == hash {
        Some(base)
    } else {
        None
    }
}
fn main() {
    let mut arr = vec![];
    for i in 0..1000 {
        arr.push((i, 0, 0));
    }
    let res = branchless_search(&arr, 500);
    println!("{:?}", res);
}
