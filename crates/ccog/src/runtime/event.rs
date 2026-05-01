//! Formal Event & Case Model for L3 process mining and traceability.

use crate::utils::dense::PackedKeyTable;
use serde::{Deserialize, Serialize};

/// Formalized Case ID for L3 event correlation.
#[repr(transparent)]
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Serialize, Deserialize,
)]
pub struct CaseId(pub u64);

impl From<u64> for CaseId {
    #[inline]
    fn from(v: u64) -> Self {
        Self(v)
    }
}

/// Lifecycle transitions for formal process events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Lifecycle {
    /// Activity has started.
    Start = 0,
    /// Activity has completed successfully.
    Complete = 1,
    /// Activity has been scheduled for future execution.
    Schedule = 2,
    /// Activity execution has been suspended.
    Suspend = 3,
    /// Activity execution has been resumed.
    Resume = 4,
    /// Activity has been aborted or failed.
    Abort = 5,
}

/// Formal Event Model for Process Mining and Traceability.
///
/// Designed to be zero-allocation on the hot path.
/// Identifiers (activity, resource, attributes, provenance) are represented
/// as hashed URNs (u64) derived via `fnv1a_64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Case identifier for the event stream.
    pub case: CaseId,
    /// Hashed URN of the activity being performed.
    pub activity: u64,
    /// Timestamp (usually microseconds since epoch).
    pub timestamp: u64,
    /// Lifecycle state of the activity.
    pub lifecycle: Lifecycle,
    /// Hashed URN of the resource performing the activity.
    pub resource: u64,
    /// Hashed ID for the attribute set.
    pub attributes: u64,
    /// Hashed ID for the provenance link.
    pub provenance: u64,
}

impl Event {
    /// Create a new event with explicit hashed values.
    pub fn new(
        case: CaseId,
        activity: u64,
        timestamp: u64,
        lifecycle: Lifecycle,
        resource: u64,
        attributes: u64,
        provenance: u64,
    ) -> Self {
        Self {
            case,
            activity,
            timestamp,
            lifecycle,
            resource,
            attributes,
            provenance,
        }
    }
}

/// A zero-allocation (pre-allocated) table for storing event attributes or metadata.
///
/// Uses `PackedKeyTable` from `utils::dense` to provide deterministic,
/// alloc-free lookups after initial capacity is established.
pub type EventAttributeTable<V> = PackedKeyTable<u64, V>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::dense::fnv1a_64;

    #[test]
    fn event_basic_construction() {
        let case = CaseId(12345);
        let activity = fnv1a_64(b"urn:ccog:activity:test");
        let timestamp = 1625097600000000; // Example timestamp
        let resource = fnv1a_64(b"urn:ccog:resource:agent-1");

        let event = Event::new(case, activity, timestamp, Lifecycle::Start, resource, 0, 0);

        assert_eq!(event.case, CaseId(12345));
        assert_eq!(event.activity, activity);
        assert_eq!(event.lifecycle, Lifecycle::Start);
    }

    #[test]
    fn event_attribute_table_usage() {
        let mut table: EventAttributeTable<&'static str> = EventAttributeTable::with_capacity(16);
        let key = fnv1a_64(b"custom_attr");
        table.insert(key, key, "value");

        assert_eq!(table.get(key), Some(&"value"));
    }
}
