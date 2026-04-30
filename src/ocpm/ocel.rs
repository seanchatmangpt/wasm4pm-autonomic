//! Hyper-optimized Object-Centric Event Log (OCEL 2.0) primitives.
//! Eliminates heap-allocated Strings during event processing via 64-bit FNV-1a hashes and flattened 1D arrays.

pub struct OcelEvent {
    pub id_hash: u64,
    pub activity_hash: u64,
    pub timestamp: u64,
    pub omap_start: u32,
    pub omap_count: u32,
}

pub struct OcelObject {
    pub id_hash: u64,
    pub type_hash: u64,
}

pub struct OcelRelation {
    pub object_id_hash: u64,
    pub qualifier_hash: u64,
}

pub struct OcelO2O {
    pub source_id_hash: u64,
    pub target_id_hash: u64,
    pub qualifier_hash: u64,
}

pub struct OcelObjectChange {
    pub id_hash: u64,
    pub type_hash: u64,
    pub timestamp: u64,
    pub changed_field_hash: u64,
    pub value_hash: u64,
}

pub struct OcelLog {
    pub events: Vec<OcelEvent>,
    pub object_relations: Vec<OcelRelation>,
    pub objects: Vec<OcelObject>,
    pub o2o: Vec<OcelO2O>,
    pub object_changes: Vec<OcelObjectChange>,
}

impl Default for OcelLog {
    fn default() -> Self {
        Self::new()
    }
}

impl OcelLog {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            object_relations: Vec::new(),
            objects: Vec::new(),
            o2o: Vec::new(),
            object_changes: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn add_event_hashed(
        &mut self,
        id_hash: u64,
        activity_hash: u64,
        timestamp: u64,
        relations: &[OcelRelation],
    ) {
        let omap_start = self.object_relations.len() as u32;
        let omap_count = relations.len() as u32;

        for rel in relations {
            self.object_relations.push(OcelRelation {
                object_id_hash: rel.object_id_hash,
                qualifier_hash: rel.qualifier_hash,
            });
        }

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
/// Fully supports OCEL 2.0 Qualifiers, O2O relations, and Object Changes.
pub struct StreamingOcDfg<const OBJ_CACHE: usize, const EDGE_CACHE: usize> {
    pub last_activity_per_obj: [u64; OBJ_CACHE],
    pub edge_frequencies: [u32; EDGE_CACHE], // (A -> B, Type)
    pub binding_frequencies: [u32; EDGE_CACHE], // (Activity, Type, Qualifier)
    pub o2o_frequencies: [u32; EDGE_CACHE],  // (SourceType, TargetType, Qualifier)
    pub recent_attribute_changes: [u64; OBJ_CACHE], // Last changed field hash per object
}

impl<const OBJ_CACHE: usize, const EDGE_CACHE: usize> Default
    for StreamingOcDfg<OBJ_CACHE, EDGE_CACHE>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const OBJ_CACHE: usize, const EDGE_CACHE: usize> StreamingOcDfg<OBJ_CACHE, EDGE_CACHE> {
    pub fn new() -> Self {
        assert!(
            OBJ_CACHE.is_power_of_two(),
            "Object cache size must be power of two"
        );
        assert!(
            EDGE_CACHE.is_power_of_two(),
            "Edge cache size must be power of two"
        );
        Self {
            last_activity_per_obj: [0; OBJ_CACHE],
            edge_frequencies: [0; EDGE_CACHE],
            binding_frequencies: [0; EDGE_CACHE],
            o2o_frequencies: [0; EDGE_CACHE],
            recent_attribute_changes: [0; OBJ_CACHE],
        }
    }

    /// Observes a new event branchlessly and updates the OC-DFG with OCEL 2.0 semantics.
    #[inline(always)]
    pub fn observe_event(&mut self, activity_hash: u64, objects: &[(u64, u64, u64)]) {
        let obj_mask = OBJ_CACHE - 1;
        let edge_mask = EDGE_CACHE - 1;

        for &(obj_hash, type_hash, qualifier_hash) in objects {
            let obj_idx = (obj_hash as usize) & obj_mask;
            let prev_activity = self.last_activity_per_obj[obj_idx];

            let is_valid = prev_activity != 0;
            let valid_mask = 0u32.wrapping_sub(is_valid as u32);

            // FNV-1a mix of prev_activity, activity_hash, AND type_hash
            let edge_hash = prev_activity
                .wrapping_mul(0x9E3779B185EBCA87)
                .wrapping_add(activity_hash)
                .wrapping_mul(0x9E3779B185EBCA87)
                .wrapping_add(type_hash);

            let edge_idx = (edge_hash as usize) & edge_mask;
            self.edge_frequencies[edge_idx] =
                self.edge_frequencies[edge_idx].saturating_add(1 & valid_mask);

            // Update bindings: Activity + Type + Qualifier
            let binding_hash = activity_hash
                .wrapping_mul(0x9E3779B185EBCA87)
                .wrapping_add(type_hash)
                .wrapping_mul(0x9E3779B185EBCA87)
                .wrapping_add(qualifier_hash);
            let binding_idx = (binding_hash as usize) & edge_mask;
            self.binding_frequencies[binding_idx] =
                self.binding_frequencies[binding_idx].saturating_add(1);

            self.last_activity_per_obj[obj_idx] = activity_hash;
        }
    }

    /// Observes structural Object-to-Object (O2O) relations independently of events.
    #[inline(always)]
    pub fn observe_o2o(
        &mut self,
        source_type_hash: u64,
        target_type_hash: u64,
        qualifier_hash: u64,
    ) {
        let edge_mask = EDGE_CACHE - 1;
        let hash = source_type_hash
            .wrapping_mul(0x9E3779B185EBCA87)
            .wrapping_add(target_type_hash)
            .wrapping_mul(0x9E3779B185EBCA87)
            .wrapping_add(qualifier_hash);

        let idx = (hash as usize) & edge_mask;
        self.o2o_frequencies[idx] = self.o2o_frequencies[idx].saturating_add(1);
    }

    /// Observes continuous or categorical attribute evolution (Object Changes).
    #[inline(always)]
    pub fn observe_object_change(&mut self, obj_hash: u64, changed_field_hash: u64) {
        let obj_mask = OBJ_CACHE - 1;
        let obj_idx = (obj_hash as usize) & obj_mask;
        self.recent_attribute_changes[obj_idx] = changed_field_hash;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocel_log_add_event_hashed() {
        let mut log = OcelLog::new();
        let relations = vec![OcelRelation {
            object_id_hash: 123,
            qualifier_hash: 456,
        }];
        log.add_event_hashed(789, 111, 1000, &relations);
        assert_eq!(log.events.len(), 1);
        assert_eq!(log.events[0].id_hash, 789);
        assert_eq!(log.events[0].activity_hash, 111);
        assert_eq!(log.events[0].timestamp, 1000);
    }

    #[test]
    fn test_streaming_oc_dfg_observe_event() {
        let mut dfg: StreamingOcDfg<64, 256> = StreamingOcDfg::new();
        let objects = vec![(100u64, 200u64, 300u64)];
        dfg.observe_event(500, &objects);
        assert_eq!(dfg.last_activity_per_obj[100 & 63], 500);
    }

    #[test]
    fn test_streaming_oc_dfg_observe_o2o() {
        let mut dfg: StreamingOcDfg<64, 256> = StreamingOcDfg::new();
        dfg.observe_o2o(111, 222, 333);
        let mut found = false;
        for &freq in &dfg.o2o_frequencies {
            if freq > 0 {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_ocel_event_fields() {
        let event = OcelEvent {
            id_hash: 1,
            activity_hash: 2,
            timestamp: 3,
            omap_start: 0,
            omap_count: 1,
        };
        assert_eq!(event.id_hash, 1);
        assert_eq!(event.activity_hash, 2);
        assert_eq!(event.timestamp, 3);
    }
}
