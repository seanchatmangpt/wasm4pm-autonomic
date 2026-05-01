//! Object-Centric Process Mining (OCEL) Extension.
//!
//! This module expands the event model to support Object-Centric Process Mining,
//! allowing a single event to bind to multiple objects (e.g., Order + Invoice).
//! It implements the "Object-Centric Field Closure" logic where closure is
//! computed per-object lifecycle.

use crate::construct8::ObjectId;
use crate::ids::NodeId;
use crate::powl64::Powl64RouteCell;

/// Maximum number of objects that can be bound to a single OCEL event.
pub const MAX_OCEL_OBJECTS: usize = 8;

/// Object-Centric Process Mining (OCEL) event.
///
/// Unlike XES events which have a single case ID, OCEL events can bind to
/// multiple objects of different types, enabling many-to-many relationship
/// discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OcelEvent {
    /// The activity name (as a NodeId).
    pub activity: NodeId,
    /// Up to 8 associated objects (Order, Invoice, Customer, etc.).
    pub objects: [Option<ObjectId>; MAX_OCEL_OBJECTS],
    /// Cryptographic chain head (pseudo-timestamp).
    pub chain_head: u64,
}

impl OcelEvent {
    /// Create a new OcelEvent with a single object.
    pub const fn new(activity: NodeId, object: ObjectId, chain_head: u64) -> Self {
        let mut objects = [None; MAX_OCEL_OBJECTS];
        objects[0] = Some(object);
        Self {
            activity,
            objects,
            chain_head,
        }
    }

    /// Create an empty OcelEvent.
    pub const fn empty(activity: NodeId, chain_head: u64) -> Self {
        Self {
            activity,
            objects: [None; MAX_OCEL_OBJECTS],
            chain_head,
        }
    }

    /// Create an OcelEvent from a POWL64 route cell and a set of objects.
    pub fn from_powl64(
        cell: &Powl64RouteCell,
        objects: [Option<ObjectId>; MAX_OCEL_OBJECTS],
    ) -> Self {
        Self {
            activity: cell.to_node,
            objects,
            chain_head: cell.chain_head,
        }
    }

    /// Add an object to the event. Returns false if the object list is full.
    pub fn bind_object(&mut self, object: ObjectId) -> bool {
        for slot in self.objects.iter_mut() {
            if slot.is_none() {
                *slot = Some(object);
                return true;
            }
            if *slot == Some(object) {
                return true; // Already bound
            }
        }
        false
    }

    /// Check if the event is bound to a specific object.
    pub fn has_object(&self, object_id: ObjectId) -> bool {
        let mut i = 0;
        while i < MAX_OCEL_OBJECTS {
            if self.objects[i] == Some(object_id) {
                return true;
            }
            i += 1;
        }
        false
    }
}

/// Object-Centric Field Closure.
///
/// Encapsulates the semantic closure of a specific object within the event log.
/// This provides a view of the object's lifecycle across multiple collaborative
/// events.
pub struct ObjectCentricFieldClosure<'a> {
    /// The object for which closure is computed.
    pub object_id: ObjectId,
    /// The log of events.
    pub log: &'a [OcelEvent],
}

impl<'a> ObjectCentricFieldClosure<'a> {
    /// Creates a new closure view for a specific object.
    pub const fn new(object_id: ObjectId, log: &'a [OcelEvent]) -> Self {
        Self { object_id, log }
    }

    /// Returns an iterator over events that belong to this object's lifecycle.
    pub fn events(&self) -> impl Iterator<Item = &'a OcelEvent> + 'a {
        let oid = self.object_id;
        self.log.iter().filter(move |event| event.has_object(oid))
    }

    /// Computes the "Object-Centric Closure Mask" — a bitmask of activities
    /// that have occurred for this object.
    ///
    /// This is used for COG8 matching where the requirements are based on
    /// object-specific history.
    pub fn activity_mask(&self) -> u64 {
        let mut mask = 0u64;
        for event in self.events() {
            let bit = (event.activity.0 % 64) as u64;
            mask |= 1 << bit;
        }
        mask
    }
}

/// Zero-allocation workspace for OCEL discovery.
pub struct OcelMiningWorkspace {
    /// Frequency of each activity (NodeId).
    pub frequencies: [u32; 64],
    /// Object count per activity.
    pub object_counts: [u32; 64],
}

impl Default for OcelMiningWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl OcelMiningWorkspace {
    /// Create a new OCEL mining workspace.
    pub const fn new() -> Self {
        Self {
            frequencies: [0; 64],
            object_counts: [0; 64],
        }
    }

    /// Reset the workspace.
    pub fn reset(&mut self) {
        self.frequencies = [0; 64];
        self.object_counts = [0; 64];
    }

    /// Mine a log of OCEL events.
    pub fn mine(&mut self, log: &[OcelEvent]) {
        for event in log {
            let idx = (event.activity.0 % 64) as usize;
            self.frequencies[idx] += 1;

            let mut count = 0;
            for obj in event.objects {
                if obj.is_some() {
                    count += 1;
                }
            }
            self.object_counts[idx] += count;
        }
    }
}

/// Helper to mine OCEL events from a log.
pub fn mine_ocel(log: &[OcelEvent], workspace: &mut OcelMiningWorkspace) {
    workspace.mine(log);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construct8::ObjectId;
    use crate::ids::EdgeId;
    use crate::ids::NodeId;
    use crate::powl64::{PartnerId, Polarity, Powl64RouteCell, ProjectionTarget};
    use crate::runtime::cog8::EdgeKind;

    #[test]
    fn test_ocel_event_binding() {
        let mut event = OcelEvent::empty(NodeId(1), 100);
        assert!(event.bind_object(ObjectId(10)));
        assert!(event.bind_object(ObjectId(20)));
        assert!(event.bind_object(ObjectId(30)));

        assert!(event.has_object(ObjectId(10)));
        assert!(event.has_object(ObjectId(20)));
        assert!(event.has_object(ObjectId(30)));
        assert!(!event.has_object(ObjectId(40)));
    }

    #[test]
    fn test_object_centric_closure() {
        let obj1 = ObjectId(1);
        let obj2 = ObjectId(2);

        let log = [
            OcelEvent::new(NodeId(10), obj1, 100),
            OcelEvent::new(NodeId(20), obj2, 200),
            {
                let mut e = OcelEvent::empty(NodeId(30), 300);
                e.bind_object(obj1);
                e.bind_object(obj2);
                e
            },
        ];

        let closure1 = ObjectCentricFieldClosure::new(obj1, &log);
        let events1: Vec<_> = closure1.events().collect();
        assert_eq!(events1.len(), 2);
        assert_eq!(events1[0].activity, NodeId(10));
        assert_eq!(events1[1].activity, NodeId(30));

        let mask1 = closure1.activity_mask();
        assert_eq!(mask1, (1 << 10) | (1 << 30));
    }

    #[test]
    fn test_ocel_from_powl64() {
        let cell = Powl64RouteCell {
            graph_id: 1,
            from_node: NodeId(1),
            to_node: NodeId(2),
            edge_id: EdgeId(1),
            edge_kind: EdgeKind::Choice,
            collapse_fn: crate::ids::CollapseFn::ExpertRule,
            polarity: Polarity::Positive,
            projection_target: ProjectionTarget::NoOp,
            partner_id: PartnerId::NONE,
            input_digest: 0,
            args_digest: 0,
            result_digest: 0,
            prior_chain: 0,
            chain_head: 999,
        };

        let mut objects = [None; 8];
        objects[0] = Some(ObjectId(500));

        let event = OcelEvent::from_powl64(&cell, objects);
        assert_eq!(event.activity, NodeId(2));
        assert_eq!(event.chain_head, 999);
        assert!(event.has_object(ObjectId(500)));
    }
}
