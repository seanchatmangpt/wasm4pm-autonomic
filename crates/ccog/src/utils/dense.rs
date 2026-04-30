//! Zero-alloc dense hash primitives: `fnv1a_64` + `PackedKeyTable`.
//!
//! Minimal subset copied from `dteam/src/utils/dense_kernel.rs`, rewritten to
//! avoid `unsafe` (ccog enforces `#![forbid(unsafe_code)]`) and to drop serde
//! coupling. Layout: a dense `Vec<(hash, key, value)>` of entries plus a
//! power-of-two `Vec<u32>` open-addressing index with linear probing.
//!
//! Hash collisions on `insert` (same `hash`, different keys) overwrite the
//! existing entry — callers using this as a structural set must hash inputs
//! deterministically and treat the table as keyed solely by `hash`.

/// Sentinel index meaning "this slot is empty".
const EMPTY_INDEX: u32 = u32::MAX;

/// FNV-1a 64-bit hash over `bytes`. Deterministic, used crate-wide for
/// content-addressed lookup keys.
#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    let mut h = OFFSET;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Open-addressed dense hash table keyed by a precomputed `u64` hash.
///
/// `entries` is a packed dense vector of `(hash, key, value)` triples.
/// `indices` is a power-of-two open-addressing index from `hash & mask` to
/// the entry's position in `entries` (or [`EMPTY_INDEX`] if empty). Linear
/// probing resolves collisions.
///
/// `K` is opaque structural metadata (often `()`) — equality on the table is
/// keyed by `hash`, not `K`. Hash-equal/key-different collisions overwrite.
#[derive(Debug, Clone)]
pub struct PackedKeyTable<K, V> {
    /// Dense list of all live entries in insertion order.
    entries: Vec<(u64, K, V)>,
    /// Open-addressing slot table; size is a power of two; values are entry
    /// indexes into `entries` or [`EMPTY_INDEX`] for empty slots.
    indices: Vec<u32>,
}

impl<K, V> PackedKeyTable<K, V> {
    /// Construct an empty table with no allocations.
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Construct an empty table sized to hold roughly `cap` entries before
    /// the next index rebuild. Rounded up to the next power of two, minimum 16.
    #[inline(always)]
    pub fn with_capacity(cap: usize) -> Self {
        if cap == 0 {
            return Self::new();
        }
        let cap = cap.next_power_of_two().max(16);
        Self {
            entries: Vec::with_capacity(cap),
            indices: vec![EMPTY_INDEX; cap],
        }
    }

    /// Resize and re-populate `indices` so its load factor stays below 50%.
    /// Called from `insert` when the index is empty or saturated.
    #[inline(never)]
    fn rebuild_indices_if_needed(&mut self) {
        if !self.indices.is_empty() && self.indices.len() > self.entries.len() * 2 {
            return;
        }
        let cap = self.entries.len().next_power_of_two().max(16) * 2;
        self.indices.clear();
        self.indices.resize(cap, EMPTY_INDEX);
        let mask = (cap - 1) as u64;
        for i in 0..self.entries.len() {
            let hash = self.entries[i].0;
            let mut idx = (hash & mask) as usize;
            loop {
                if self.indices[idx] == EMPTY_INDEX {
                    self.indices[idx] = i as u32;
                    break;
                }
                idx = (idx + 1) & mask as usize;
            }
        }
    }

    /// Insert `(hash, key, value)`. If `hash` already exists, the previous
    /// value is replaced and returned in `Some`. Otherwise returns `None`.
    ///
    /// NB: collisions (same `hash`, different `key`) overwrite. The caller
    /// is responsible for ensuring `hash` uniquely identifies the logical key.
    #[inline]
    pub fn insert(&mut self, hash: u64, key: K, value: V) -> Option<V> {
        if self.indices.is_empty() || self.entries.len() * 2 >= self.indices.len() {
            self.rebuild_indices_if_needed();
        }
        let mask = (self.indices.len() - 1) as u64;
        let mut idx = (hash & mask) as usize;
        loop {
            let entry_idx = self.indices[idx];
            if entry_idx == EMPTY_INDEX {
                let new_idx = self.entries.len() as u32;
                self.indices[idx] = new_idx;
                self.entries.push((hash, key, value));
                return None;
            }
            let entry_pos = entry_idx as usize;
            if self.entries[entry_pos].0 == hash {
                let old = std::mem::replace(&mut self.entries[entry_pos], (hash, key, value));
                return Some(old.2);
            }
            idx = (idx + 1) & mask as usize;
        }
    }

    /// Mutable lookup by `hash`. Returns `None` if not present.
    #[inline]
    pub fn get_mut(&mut self, hash: u64) -> Option<&mut V> {
        if self.indices.is_empty() {
            return None;
        }
        let mask = (self.indices.len() - 1) as u64;
        let mut idx = (hash & mask) as usize;
        loop {
            let entry_idx = self.indices[idx];
            if entry_idx == EMPTY_INDEX {
                return None;
            }
            let entry_pos = entry_idx as usize;
            if self.entries[entry_pos].0 == hash {
                return Some(&mut self.entries[entry_pos].2);
            }
            idx = (idx + 1) & mask as usize;
        }
    }

    /// Look up a value by `hash`. Returns `None` if not present.
    #[inline]
    pub fn get(&self, hash: u64) -> Option<&V> {
        if self.indices.is_empty() {
            return None;
        }
        let mask = (self.indices.len() - 1) as u64;
        let mut idx = (hash & mask) as usize;
        loop {
            let entry_idx = self.indices[idx];
            if entry_idx == EMPTY_INDEX {
                return None;
            }
            let entry = &self.entries[entry_idx as usize];
            if entry.0 == hash {
                return Some(&entry.2);
            }
            idx = (idx + 1) & mask as usize;
        }
    }

    /// Number of live entries.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if no entries are stored.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate entries in insertion order, yielding `(&hash, &key, &value)`.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &K, &V)> + '_ {
        self.entries.iter().map(|(h, k, v)| (h, k, v))
    }

    /// Drop all entries and reset the index without releasing capacity.
    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear();
        for slot in self.indices.iter_mut() {
            *slot = EMPTY_INDEX;
        }
    }
}

impl<K, V> Default for PackedKeyTable<K, V> {
    /// Equivalent to [`PackedKeyTable::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for PackedKeyTable<K, V> {
    /// Two tables are equal when their dense entry vectors are element-wise equal.
    /// Order-sensitive: tables built by different insertion orders may differ.
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_64_is_deterministic_and_distinct() {
        assert_eq!(fnv1a_64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a_64(b"abc"), fnv1a_64(b"abc"));
        assert_ne!(fnv1a_64(b"abc"), fnv1a_64(b"abd"));
    }

    #[test]
    fn packed_key_table_basic_insert_get() {
        let mut t: PackedKeyTable<(), u32> = PackedKeyTable::new();
        assert!(t.is_empty());
        assert_eq!(t.insert(1, (), 10), None);
        assert_eq!(t.insert(2, (), 20), None);
        assert_eq!(t.len(), 2);
        assert_eq!(t.get(1), Some(&10));
        assert_eq!(t.get(2), Some(&20));
        assert_eq!(t.get(3), None);
    }

    #[test]
    fn packed_key_table_replaces_on_same_hash() {
        let mut t: PackedKeyTable<(), u32> = PackedKeyTable::new();
        assert_eq!(t.insert(7, (), 100), None);
        assert_eq!(t.insert(7, (), 200), Some(100));
        assert_eq!(t.len(), 1);
        assert_eq!(t.get(7), Some(&200));
    }

    #[test]
    fn packed_key_table_grows_through_rebuild() {
        let mut t: PackedKeyTable<(), u64> = PackedKeyTable::with_capacity(4);
        for i in 0..1000u64 {
            t.insert(i.wrapping_mul(0x9E3779B97F4A7C15), (), i);
        }
        assert_eq!(t.len(), 1000);
        for i in 0..1000u64 {
            assert_eq!(t.get(i.wrapping_mul(0x9E3779B97F4A7C15)), Some(&i));
        }
    }

    #[test]
    fn packed_key_table_iter_yields_all_entries() {
        let mut t: PackedKeyTable<(), u32> = PackedKeyTable::new();
        t.insert(1, (), 10);
        t.insert(2, (), 20);
        t.insert(3, (), 30);
        let mut seen: Vec<(u64, u32)> = t.iter().map(|(h, _, v)| (*h, *v)).collect();
        seen.sort();
        assert_eq!(seen, vec![(1, 10), (2, 20), (3, 30)]);
    }

    #[test]
    fn packed_key_table_clear_resets_len_but_keeps_capacity() {
        let mut t: PackedKeyTable<(), u32> = PackedKeyTable::with_capacity(64);
        for i in 0..32 {
            t.insert(i as u64, (), i);
        }
        assert_eq!(t.len(), 32);
        t.clear();
        assert_eq!(t.len(), 0);
        assert!(t.is_empty());
        assert_eq!(t.get(0), None);
        // re-insertion still works
        t.insert(99, (), 99);
        assert_eq!(t.get(99), Some(&99));
    }

    #[test]
    fn packed_key_table_default_equals_new() {
        let a: PackedKeyTable<(), u32> = PackedKeyTable::default();
        let b: PackedKeyTable<(), u32> = PackedKeyTable::new();
        assert_eq!(a, b);
        assert!(a.is_empty());
    }
}
