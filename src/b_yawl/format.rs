//! Binary YAWL (bYAWL) Format for Nanosecond Execution.
//! Replaces XML with a cache-friendly, 64-bit aligned binary packed structure.

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JoinType {
    XOR = 0,
    AND = 1,
    OR = 2,
    Complex = 3,     // N-out-of-M, Discriminator, Partial Joins
    ThreadMerge = 4, // WCP-41
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SplitType {
    XOR = 0,
    AND = 1,
    OR = 2,
    MultiInstance = 3,
    DynamicMultiInstance = 4, // WCP-15: Spawn instances dynamically based on runtime trigger
    DeferredChoice = 9,
    InterleavedRouting = 5,
    ThreadSplit = 6,
    ImplicitTermination = 7,
    ExplicitTermination = 8,
}

/// A 64-byte cache-aligned Binary YAWL Task representation.
/// Represents all 43 Workflow Patterns without XML parsing overhead.
#[derive(Clone, Copy, Debug)]
#[repr(C, align(64))]
pub struct BYawlTask {
    pub id: u16,
    pub join_type: JoinType,
    pub split_type: SplitType,
    pub min_instances: u8,
    pub max_instances: u8,
    pub threshold_instances: u8,
    pub join_state_bit: u8, // Tracks if a complex join has fired (Discriminators)
    pub flags: u8,          // Custom pattern flags (e.g. Transient vs Persistent Trigger)

    pub consume_mask: u64,
    pub produce_mask: u64,
    pub cancellation_mask: u64,
    pub condition_mask: u64, // Used for Milestone (WCP-18) and Critical Section (WCP-39)
    pub reset_mask: u64,     // Used for Cancelling Discriminators (WCP-29, 32, 35)

    /// Upstream places that can reach this task. Essential for O(1) Synchronizing Merge (OR-Join).
    pub reachability_mask: u64,

    /// Mutex mask for Interleaved Parallel Routing (WCP-17, WCP-40)
    pub interleaved_lock_mask: u64,
}
