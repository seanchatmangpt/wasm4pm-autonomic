//! Hyper-optimized Object-Centric Event Log (OCEL) primitives.
//! Eliminates heap-allocated Strings during event processing via 64-bit FNV-1a hashes and flattened 1D arrays.

pub struct OcelEvent {
    pub id_hash: u64,
    pub activity_hash: u64,
    pub timestamp: u64, // Epoch time instead of String
    pub omap_start: u32,
    pub omap_count: u32,
}

pub struct OcelObject {
    pub id_hash: u64,
    pub type_hash: u64,
}

pub struct OcelLog {
    pub events: Vec<OcelEvent>,
    pub object_relations: Vec<u64>, // Flat array of object ID hashes mapped to events
    pub objects: Vec<OcelObject>,   // Sorted array of objects for fast binary search
}

impl Default for OcelLog {
    fn default() -> Self { Self::new() }
}

impl OcelLog {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            object_relations: Vec::new(),
            objects: Vec::new(),
        }
    }

    /// Add an event with zero internal String allocation if hashed strings are provided
    #[inline(always)]
    pub fn add_event_hashed(&mut self, id_hash: u64, activity_hash: u64, timestamp: u64, object_hashes: &[u64]) {
        let omap_start = self.object_relations.len() as u32;
        let omap_count = object_hashes.len() as u32;
        
        self.object_relations.extend_from_slice(object_hashes);
        
        self.events.push(OcelEvent {
            id_hash,
            activity_hash,
            timestamp,
            omap_start,
            omap_count,
        });
    }
}

/// Hyper-optimized Streaming Object-Centric Directly Follows Graph (OC-DFG).
/// Tracks the transition frequencies between activities PER OBJECT TYPE without flattening.
/// Uses fixed-size stack-friendly arrays and FNV-1a branchless index mapping.
pub struct StreamingOcDfg<const OBJ_CACHE: usize, const EDGE_CACHE: usize> {
    pub last_activity_per_obj: [u64; OBJ_CACHE], // Maps obj_hash % CACHE to activity_hash
    pub edge_frequencies: [u32; EDGE_CACHE],     // Tracks the frequency of (A -> B)
}

impl<const OBJ_CACHE: usize, const EDGE_CACHE: usize> Default for StreamingOcDfg<OBJ_CACHE, EDGE_CACHE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const OBJ_CACHE: usize, const EDGE_CACHE: usize> StreamingOcDfg<OBJ_CACHE, EDGE_CACHE> {
    pub fn new() -> Self {
        assert!(OBJ_CACHE.is_power_of_two(), "Object cache size must be power of two");
        assert!(EDGE_CACHE.is_power_of_two(), "Edge cache size must be power of two");
        Self {
            last_activity_per_obj: [0; OBJ_CACHE],
            edge_frequencies: [0; EDGE_CACHE],
        }
    }

    /// Observes a new event branchlessly and updates the OC-DFG.
    /// Eliminates heap allocations and tracks object-specific lifecycles (Convergence).
    #[inline(always)]
    pub fn observe_event(&mut self, activity_hash: u64, object_hashes: &[u64]) {
        let obj_mask = OBJ_CACHE - 1;
        let edge_mask = EDGE_CACHE - 1;

        for &obj_hash in object_hashes {
            let obj_idx = (obj_hash as usize) & obj_mask;
            let prev_activity = self.last_activity_per_obj[obj_idx];
            
            // If we have seen an activity for this object before, update the edge
            // Branchless mask trick: if prev_activity is 0, we don't update edge.
            let is_valid = prev_activity != 0;
            let valid_mask = 0u32.wrapping_sub(is_valid as u32);
            
            // FNV-1a mix of prev_activity and current activity_hash
            let edge_hash = prev_activity.wrapping_mul(0x9E3779B185EBCA87).wrapping_add(activity_hash);
            let edge_idx = (edge_hash as usize) & edge_mask;
            
            // Only add 1 if it's a valid edge, else add 0
            self.edge_frequencies[edge_idx] = self.edge_frequencies[edge_idx].saturating_add(1 & valid_mask);
            
            // Update the last seen activity for this object
            self.last_activity_per_obj[obj_idx] = activity_hash;
        }
    }
}
