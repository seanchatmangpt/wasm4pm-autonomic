#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct KappaByte(pub u8);

impl KappaByte {
    pub const REFLECT: Self = Self(1 << 0);
    pub const PRECONDITION: Self = Self(1 << 1);
    pub const GROUND: Self = Self(1 << 2);
    pub const PROVE: Self = Self(1 << 3);
    pub const RULE: Self = Self(1 << 4);
    pub const RECONSTRUCT: Self = Self(1 << 5);
    pub const FUSE: Self = Self(1 << 6);
    pub const REDUCE_GAP: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct InstinctByte(pub u8);

impl InstinctByte {
    pub const SETTLE: Self = Self(1 << 0);
    pub const RETRIEVE: Self = Self(1 << 1);
    pub const INSPECT: Self = Self(1 << 2);
    pub const ASK: Self = Self(1 << 3);
    pub const AWAIT: Self = Self(1 << 4);
    pub const REFUSE: Self = Self(1 << 5);
    pub const ESCALATE: Self = Self(1 << 6);
    pub const IGNORE: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SelectedInstinctByte(pub u8);

impl SelectedInstinctByte {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn onehot(bits: u8) -> Self {
        if bits == 0 || bits.count_ones() == 1 {
            Self(bits)
        } else {
            Self(0)
        }
    }
    #[inline(always)]
    pub const fn new_decode(bits: u8) -> Option<Self> {
        if bits == 0 || bits.count_ones() == 1 {
            Some(Self(bits))
        } else {
            None
        }
    }
    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

// --- Family8 Micro-Bytes ---

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct ElizaByte(pub u8);

impl ElizaByte {
    pub const MIRROR_INTENT: Self = Self(1 << 0);
    pub const RESTATE_CLAIM: Self = Self(1 << 1);
    pub const DETECT_AFFECT: Self = Self(1 << 2);
    pub const DETECT_AMBIGUITY: Self = Self(1 << 3);
    pub const DETECT_MISSING_SLOT: Self = Self(1 << 4);
    pub const ASK_CLARIFYING: Self = Self(1 << 5);
    pub const SLOW_PREMATURE_ACTION: Self = Self(1 << 6);
    pub const DEFER_TO_CLOSURE: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct StripsByte(pub u8);

impl StripsByte {
    pub const PRECONDITIONS_SATISFIED: Self = Self(1 << 0);
    pub const MISSING_REQUIRED: Self = Self(1 << 1);
    pub const FORBIDDEN_PRESENT: Self = Self(1 << 2);
    pub const EFFECTS_KNOWN: Self = Self(1 << 3);
    pub const EFFECTS_CONFLICT: Self = Self(1 << 4);
    pub const ACTION_ENABLED: Self = Self(1 << 5);
    pub const ACTION_BLOCKED: Self = Self(1 << 6);
    pub const REQUIRES_REPLAN: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct ShrdluByte(pub u8);

impl ShrdluByte {
    pub const SYMBOL_RESOLVED: Self = Self(1 << 0);
    pub const OBJECT_UNIQUE: Self = Self(1 << 1);
    pub const ALIAS_MATCHED: Self = Self(1 << 2);
    pub const CONTEXT_DISAMBIGUATED: Self = Self(1 << 3);
    pub const AMBIGUOUS_REFERENCE: Self = Self(1 << 4);
    pub const MISSING_OBJECT: Self = Self(1 << 5);
    pub const AUTHORITY_MISMATCH: Self = Self(1 << 6);
    pub const GROUNDING_FAILED: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct PrologByte(pub u8);

impl PrologByte {
    pub const GOAL_PROVED: Self = Self(1 << 0);
    pub const GOAL_FAILED: Self = Self(1 << 1);
    pub const FACT_MISSING: Self = Self(1 << 2);
    pub const RULE_MATCHED: Self = Self(1 << 3);
    pub const CONTRADICTION_FOUND: Self = Self(1 << 4);
    pub const DEPTH_EXHAUSTED: Self = Self(1 << 5);
    pub const CYCLE_DETECTED: Self = Self(1 << 6);
    pub const PROOF_REQUIRES_ESCALATION: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct MycinByte(pub u8);

impl MycinByte {
    pub const RULE_MATCHED: Self = Self(1 << 0);
    pub const RULE_FIRED: Self = Self(1 << 1);
    pub const RULE_CONFLICT: Self = Self(1 << 2);
    pub const CONFIDENCE_HIGH: Self = Self(1 << 3);
    pub const CONFIDENCE_LOW: Self = Self(1 << 4);
    pub const POLICY_EPOCH_VALID: Self = Self(1 << 5);
    pub const POLICY_EPOCH_STALE: Self = Self(1 << 6);
    pub const EXPERT_REVIEW_REQUIRED: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct DendralByte(pub u8);

impl DendralByte {
    pub const FRAGMENTS_SUFFICIENT: Self = Self(1 << 0);
    pub const CANDIDATE_GENERATED: Self = Self(1 << 1);
    pub const CANDIDATE_PRUNED: Self = Self(1 << 2);
    pub const UNIQUE_RECONSTRUCTION: Self = Self(1 << 3);
    pub const MULTIPLE_RECONSTRUCTIONS: Self = Self(1 << 4);
    pub const MISSING_FRAGMENT: Self = Self(1 << 5);
    pub const CONSTRAINT_VIOLATION: Self = Self(1 << 6);
    pub const RECONSTRUCTION_UNSTABLE: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct HearsayByte(pub u8);

impl HearsayByte {
    pub const SOURCE_AGREES: Self = Self(1 << 0);
    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const SOURCE_CONFLICTS: Self = Self(1 << 1);
    pub const SOURCE_MISSING: Self = Self(1 << 2);
    pub const SOURCE_STALE: Self = Self(1 << 3);
    pub const SOURCE_AUTHORITATIVE: Self = Self(1 << 4);
    pub const SOURCE_WEAK: Self = Self(1 << 5);
    pub const FUSION_COMPLETE: Self = Self(1 << 6);
    pub const FUSION_REQUIRES_INSPECTION: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct GpsByte(pub u8);

impl GpsByte {
    pub const GOAL_KNOWN: Self = Self(1 << 0);
    pub const GAP_DETECTED: Self = Self(1 << 1);
    pub const GAP_SMALL: Self = Self(1 << 2);
    pub const GAP_LARGE: Self = Self(1 << 3);
    pub const OPERATOR_AVAILABLE: Self = Self(1 << 4);
    pub const OPERATOR_BLOCKED: Self = Self(1 << 5);
    pub const PROGRESS_MADE: Self = Self(1 << 6);
    pub const NO_PROGRESS: Self = Self(1 << 7);

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct KappaDetail16 {
    pub kappa: KappaByte,
    pub eliza: ElizaByte,
    pub strips: StripsByte,
    pub shrdlu: ShrdluByte,
    pub prolog: PrologByte,
    pub mycin: MycinByte,
    pub dendral: DendralByte,
    pub hearsay: HearsayByte,
    pub gps: GpsByte,
    pub reserved: [u8; 7],
}

impl KappaDetail16 {
    pub const fn empty() -> Self {
        Self {
            kappa: KappaByte(0),
            eliza: ElizaByte(0),
            strips: StripsByte(0),
            shrdlu: ShrdluByte(0),
            prolog: PrologByte(0),
            mycin: MycinByte(0),
            dendral: DendralByte(0),
            hearsay: HearsayByte(0),
            gps: GpsByte(0),
            reserved: [0; 7],
        }
    }
}
