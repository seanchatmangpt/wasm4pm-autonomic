//! dense_kernel.rs

use serde::{Deserialize, Serialize};

// ============================================================================
// FNV-1a HASHING
// ============================================================================

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

// ============================================================================
// BASIC TYPES
// ============================================================================

pub type DenseId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Generic,
    Place,
    Transition,
    Port,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenseError {
    HashCollision {
        hash: u64,
        left: String,
        right: String,
    },
    DuplicateSymbol {
        id: String,
    },
    UnknownSymbol {
        id: String,
    },
    UnknownDenseId {
        id: DenseId,
    },
    InvalidArc {
        from: String,
        to: String,
        reason: &'static str,
    },
    CapacityExceeded {
        requested: usize,
        capacity: usize,
    },
}

// ============================================================================
// DENSE INDEX
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseIndex {
    entries: Vec<IndexEntry>,
    dense_to_hash: Vec<u64>,
    symbols: Vec<String>,
    kinds: Vec<NodeKind>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct IndexEntry {
    hash: u64,
    dense: DenseId,
}

impl DenseIndex {
    pub fn compile<I, S>(symbols: I) -> Result<Self, DenseError>
    where
        I: IntoIterator<Item = (S, NodeKind)>,
        S: Into<String>,
    {
        let mut tmp: Vec<(u64, String, NodeKind)> = Vec::new();

        for (sym, kind) in symbols {
            let id = sym.into();
            let hash = fnv1a_64(id.as_bytes());
            tmp.push((hash, id, kind));
        }

        // AC 1: Sort symbols prior to indexing for deterministic DenseId assignment.
        // We sort by kind first to preserve range-based assumptions (e.g. Places < Transitions).
        tmp.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.1.cmp(&b.1)));

        // AC 2: Collision Guard Admissibility.
        // Check for duplicates and collisions using a hash-sorted view.
        let mut sorted_hashes: Vec<(u64, usize)> = tmp
            .iter()
            .enumerate()
            .map(|(i, (h, _, _))| (*h, i))
            .collect();
        sorted_hashes.sort_by_key(|&(h, _)| h);

        for pair in sorted_hashes.windows(2) {
            let (h1, i1) = (pair[0].0, pair[0].1);
            let (h2, i2) = (pair[1].0, pair[1].1);
            let s1 = &tmp[i1].1;
            let s2 = &tmp[i2].1;

            if s1 == s2 {
                return Err(DenseError::DuplicateSymbol { id: s1.clone() });
            }

            if h1 == h2 {
                return Err(DenseError::HashCollision {
                    hash: h1,
                    left: s1.clone(),
                    right: s2.clone(),
                });
            }
        }

        let mut entries = Vec::with_capacity(tmp.len());
        let mut dense_to_hash = Vec::with_capacity(tmp.len());
        let mut symbols = Vec::with_capacity(tmp.len());
        let mut kinds = Vec::with_capacity(tmp.len());

        for (dense, (hash, symbol, kind)) in tmp.into_iter().enumerate() {
            let dense = dense as DenseId;

            entries.push(IndexEntry { hash, dense });
            dense_to_hash.push(hash);
            symbols.push(symbol);
            kinds.push(kind);
        }

        // AC 4: Sort entries by hash for O(log N) binary search lookup.
        entries.sort_by_key(|e| e.hash);

        Ok(Self {
            entries,
            dense_to_hash,
            symbols,
            kinds,
        })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// AC 5: Captures the hash of the activity ontology for execution provenance.
    #[inline]
    pub fn ontology_hash(&self) -> u64 {
        const OFFSET: u64 = 0xcbf29ce484222325;
        const PRIME: u64 = 0x100000001b3;
        let mut h = OFFSET;
        for s in &self.symbols {
            h ^= fnv1a_64(s.as_bytes());
            h = h.wrapping_mul(PRIME);
        }
        h
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
    #[inline]
    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }
    #[inline]
    pub fn dense_id_by_symbol(&self, symbol: &str) -> Option<DenseId> {
        self.dense_id(symbol)
    }
    #[inline]
    pub fn dense_id(&self, symbol: &str) -> Option<DenseId> {
        self.dense_id_by_hash(fnv1a_64(symbol.as_bytes()))
    }
    #[inline]
    pub fn dense_id_by_hash(&self, hash: u64) -> Option<DenseId> {
        self.entries
            .binary_search_by_key(&hash, |e| e.hash)
            .ok()
            .map(|i| self.entries[i].dense)
    }
    #[inline]
    pub fn symbol(&self, dense: DenseId) -> Option<&str> {
        self.symbols.get(dense as usize).map(|s| s.as_str())
    }
    #[inline]
    pub fn kind(&self, dense: DenseId) -> Option<NodeKind> {
        self.kinds.get(dense as usize).copied()
    }
}

// ============================================================================
// K-BITSET
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KBitSet<const WORDS: usize> {
    pub words: [u64; WORDS],
}

impl<const WORDS: usize> Serialize for KBitSet<WORDS> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tup = serializer.serialize_tuple(WORDS)?;
        for w in &self.words {
            tup.serialize_element(w)?;
        }
        tup.end()
    }
}

impl<'de, const WORDS: usize> Deserialize<'de> for KBitSet<WORDS> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct KBitSetVisitor<const W: usize>;

        impl<'de, const W: usize> serde::de::Visitor<'de> for KBitSetVisitor<W> {
            type Value = [u64; W];

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a bitset array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut words = [0u64; W];
                for (i, word) in words.iter_mut().enumerate() {
                    *word = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(words)
            }
        }

        let words = deserializer.deserialize_tuple(WORDS, KBitSetVisitor::<WORDS>)?;
        Ok(KBitSet { words })
    }
}

impl<const WORDS: usize> Default for KBitSet<WORDS> {
    #[inline]
    fn default() -> Self {
        Self {
            words: [0u64; WORDS],
        }
    }
}

impl<const WORDS: usize> KBitSet<WORDS> {
    pub const BITS: usize = WORDS * 64;
    #[inline]
    pub const fn zero() -> Self {
        Self {
            words: [0u64; WORDS],
        }
    }
    #[inline]
    pub fn clear(&mut self) {
        for w in &mut self.words {
            *w = 0;
        }
    }
    #[inline]
    pub fn set(&mut self, bit: usize) -> Result<(), DenseError> {
        if bit >= Self::BITS {
            return Err(DenseError::CapacityExceeded {
                requested: bit + 1,
                capacity: Self::BITS,
            });
        }
        self.words[bit >> 6] |= 1u64 << (bit & 63);
        Ok(())
    }
    #[inline]
    pub fn contains(&self, bit: usize) -> bool {
        if bit >= Self::BITS {
            return false;
        }
        ((self.words[bit >> 6] >> (bit & 63)) & 1) != 0
    }

    #[inline]
    pub fn contains_all(self, required: Self) -> bool {
        let mut diff = 0u64;
        for i in 0..WORDS {
            diff |= required.words[i] & !self.words[i];
        }
        diff == 0
    }

    #[inline]
    pub fn missing_count(self, required: Self) -> u32 {
        let mut n = 0u32;
        for i in 0..WORDS {
            n += (required.words[i] & !self.words[i]).count_ones();
        }
        n
    }

    #[inline]
    pub fn bitwise_or(&self, other: Self) -> Self {
        let mut res = Self::zero();
        for i in 0..WORDS {
            res.words[i] = self.words[i] | other.words[i];
        }
        res
    }

    #[inline]
    pub fn bitwise_and(&self, other: Self) -> Self {
        let mut res = Self::zero();
        for i in 0..WORDS {
            res.words[i] = self.words[i] & other.words[i];
        }
        res
    }

    #[inline]
    pub fn bitwise_not(&self) -> Self {
        let mut res = Self::zero();
        for i in 0..WORDS {
            res.words[i] = !self.words[i];
        }
        res
    }

    #[inline]
<<<<<<< HEAD
    pub fn pop_count(&self) -> u32 {
        let mut n = 0u32;
        for w in &self.words {
            n += w.count_ones();
        }
        n
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        let mut res = 0u64;
        for i in 0..WORDS {
            res |= self.words[i];
        }
        res == 0
    }

    #[inline]
    pub fn is_enabled_mask(self, required: Self) -> u64 {
        let mut diff = 0u64;
        for i in 0..WORDS {
            diff |= required.words[i] & !self.words[i];
        }
        let is_nonzero = (diff | diff.wrapping_neg()) >> 63;
        1 - is_nonzero
=======
    pub fn is_empty(&self) -> bool {
        let mut mask = 0u64;
        for i in 0..WORDS {
            mask |= self.words[i];
        }
        mask == 0
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
    }
}

pub type K64 = KBitSet<1>;
pub type K128 = KBitSet<2>;
pub type K256 = KBitSet<4>;
pub type K512 = KBitSet<8>;
pub type K1024 = KBitSet<16>;

// ============================================================================
// PACKED KEY TABLE
// ============================================================================

<<<<<<< HEAD
const EMPTY_INDEX: u32 = u32::MAX;

#[derive(Debug, Clone, Serialize, Deserialize)]
=======
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
pub struct PackedKeyTable<K, V> {
    entries: Vec<(u64, K, V)>,
    #[serde(skip, default = "default_indices")]
    indices: Vec<u32>,
}

fn default_indices() -> Vec<u32> {
    Vec::new()
}

impl<K: PartialEq, V: PartialEq> PartialEq for PackedKeyTable<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl<K, V> PackedKeyTable<K, V> {
<<<<<<< HEAD
    #[inline(always)]
=======
    #[inline]
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            indices: Vec::new(),
        }
    }
<<<<<<< HEAD

    #[inline(always)]
=======
    #[inline]
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
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
<<<<<<< HEAD

    #[inline(never)]
    fn rebuild_indices_if_needed(&mut self) {
        if !self.indices.is_empty() && self.indices.len() > self.entries.len() * 2 {
            return;
        }
        let cap = self.entries.len().next_power_of_two().max(16) * 2;
        self.indices.clear();
        self.indices.resize(cap, EMPTY_INDEX);
        let mask = (cap - 1) as u64;
        for (i, &(hash, _, _)) in self.entries.iter().enumerate() {
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

    #[inline(always)]
=======
    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional);
    }
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
    pub fn insert(&mut self, hash: u64, key: K, value: V) {
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
                return;
            }
            if unsafe { self.entries.get_unchecked(entry_idx as usize).0 } == hash {
                unsafe { *self.entries.get_unchecked_mut(entry_idx as usize) = (hash, key, value) };
                return;
            }
            idx = (idx + 1) & mask as usize;
        }
    }

<<<<<<< HEAD
    #[inline(always)]
    pub fn get(&self, hash: u64) -> Option<&V> {
        if self.indices.is_empty() { return None; }
        let mask = (self.indices.len() - 1) as u64;
        let mut idx = (hash & mask) as usize;
        loop {
            let entry_idx = unsafe { *self.indices.get_unchecked(idx) };
            if entry_idx == EMPTY_INDEX {
                return None;
            }
            let entry = unsafe { self.entries.get_unchecked(entry_idx as usize) };
            if entry.0 == hash {
                return Some(&entry.2);
            }
            idx = (idx + 1) & mask as usize;
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, hash: u64) -> Option<&mut V> {
        if self.indices.is_empty() { return None; }
        let mask = (self.indices.len() - 1) as u64;
        let mut idx = (hash & mask) as usize;
        loop {
            let entry_idx = unsafe { *self.indices.get_unchecked(idx) };
            if entry_idx == EMPTY_INDEX {
                return None;
            }
            if unsafe { self.entries.get_unchecked(entry_idx as usize).0 } == hash {
                return Some(&mut unsafe { self.entries.get_unchecked_mut(entry_idx as usize) }.2);
            }
            idx = (idx + 1) & mask as usize;
        }
    }

    #[inline(always)]
=======
    /// Branchless lookup using a power-of-two decomposition.
    /// This ensures Var(τ) = 0 and eliminates data-dependent branching.
    #[inline]
    pub fn get(&self, hash: u64) -> Option<&V> {
        let entries = &self.entries;
        let n = entries.len();
        if n == 0 {
            return None;
        }

        let mut base = 0;
        let mut size = n;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            // Truly branchless comparison using boolean to integer conversion
            let cond = (entries[mid].0 <= hash) as usize;
            base += cond * half;
            size -= half;
        }

        if entries[base].0 == hash {
            Some(&entries[base].2)
        } else {
            None
        }
    }

    /// Branchless mutable lookup.
    #[inline]
    pub fn get_mut(&mut self, hash: u64) -> Option<&mut V> {
        let n = self.entries.len();
        if n == 0 {
            return None;
        }

        let mut base = 0;
        let mut size = n;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            let cond = (self.entries[mid].0 <= hash) as usize;
            base += cond * half;
            size -= half;
        }

        if self.entries[base].0 == hash {
            Some(&mut self.entries[base].2)
        } else {
            None
        }
    }

    #[inline]
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &(u64, K, V)> {
        self.entries.iter()
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.indices.fill(EMPTY_INDEX);
    }
}

// ============================================================================
// STATIC PACKED KEY TABLE
// ============================================================================

/// Truly zero-allocation, stack-allocated PackedKeyTable.
/// Requires K and V to be Copy.
#[derive(Debug, Clone, Copy)]
pub struct StaticPackedKeyTable<K, V, const N: usize>
where
    K: Copy + Default,
    V: Copy + Default,
{
    pub entries: [(u64, K, V); N],
    pub len: usize,
}

impl<K, V, const N: usize> Default for StaticPackedKeyTable<K, V, N>
where
    K: Copy + Default,
    V: Copy + Default,
{
    fn default() -> Self {
        Self {
            entries: [(0, K::default(), V::default()); N],
            len: 0,
        }
    }
}

impl<K, V, const N: usize> StaticPackedKeyTable<K, V, N>
where
    K: Copy + Default,
    V: Copy + Default,
{
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn get(&self, hash: u64) -> Option<&V> {
        if self.len == 0 {
            return None;
        }

        let mut base = 0;
        let mut size = self.len;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            let cond = (self.entries[mid].0 <= hash) as usize;
            base += cond * half;
            size -= half;
        }

        if self.entries[base].0 == hash {
            Some(&self.entries[base].2)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut(&mut self, hash: u64) -> Option<&mut V> {
        if self.len == 0 {
            return None;
        }

        let mut base = 0;
        let mut size = self.len;
        while size > 1 {
            let half = size / 2;
            let mid = base + half;
            let cond = (self.entries[mid].0 <= hash) as usize;
            base += cond * half;
            size -= half;
        }

        if self.entries[base].0 == hash {
            Some(&mut self.entries[base].2)
        } else {
            None
        }
    }

    pub fn insert(&mut self, hash: u64, key: K, value: V) -> Result<(), DenseError> {
        let pos = match self.entries[..self.len].binary_search_by_key(&hash, |(h, _, _)| *h) {
            Ok(i) => {
                self.entries[i] = (hash, key, value);
                return Ok(());
            }
            Err(i) => i,
        };

        if self.len >= N {
            return Err(DenseError::CapacityExceeded {
                requested: self.len + 1,
                capacity: N,
            });
        }

        // Shift elements to the right
        for j in (pos..self.len).rev() {
            self.entries[j + 1] = self.entries[j];
        }
        self.entries[pos] = (hash, key, value);
        self.len += 1;
        Ok(())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &(u64, K, V)> {
        self.entries[..self.len].iter()
    }
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }
}
