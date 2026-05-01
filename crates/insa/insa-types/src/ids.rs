//! ID definitions.

/// Identifies a specific node in a topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NodeId(pub u16);

/// Indicates an empty node reference.
pub const NODE_NONE: NodeId = NodeId(u16::MAX);

/// Identifies an edge in a topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EdgeId(pub u16);

/// Identifies a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PackId(pub u16);

/// Identifies a group within a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GroupId(pub u16);

/// Identifies a rule within a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RuleId(pub u16);

/// Identifies a specific breed execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BreedId(pub u16);

/// Identifies a specific proof route segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RouteId(pub u64);
