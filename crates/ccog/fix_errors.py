import os
import shutil

# 1. Clean up orphaned files in insa-types
orphans = [
    '../insa/insa-types/src/ids',
    '../insa/insa-types/src/ids.rs',
    '../insa/insa-types/src/instinct',
    '../insa/insa-types/src/instinct.rs',
    '../insa/insa-types/src/kappa',
    '../insa/insa-types/src/kappa.rs',
    '../insa/insa-types/src/masks',
    '../insa/insa-types/src/masks.rs',
]

for o in orphans:
    if os.path.exists(o):
        if os.path.isdir(o):
            shutil.rmtree(o)
        else:
            os.remove(o)

def replace_in_file(path, old, new):
    if os.path.exists(path):
        with open(path, 'r') as f:
            content = f.read()
        if old in content:
            content = content.replace(old, new)
            with open(path, 'w') as f:
                f.write(content)
            print(f"Updated {path}")
        else:
            print(f"Skipped {path} (target not found)")

# 2. Fix mask.rs
mask_old = """impl FieldBit {
    /// Creates a checked FieldBit.
    pub const fn new_checked(value: u8) -> Result<Self, &'static str> {
        if value < 64 {
            Ok(Self(value))
        } else {
            Err("FieldBit must be in range [0, 63]")
        }
    }
}"""
mask_new = """#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskError {
    OutOfBounds,
}

impl FieldBit {
    /// Creates a checked FieldBit.
    pub const fn new_checked(value: u8) -> Result<Self, MaskError> {
        if value < 64 {
            Ok(Self(value))
        } else {
            Err(MaskError::OutOfBounds)
        }
    }
}"""
replace_in_file('../insa/insa-types/src/mask.rs', mask_old, mask_new)

# 3. Fix construct8.rs
const8_old = """    /// Attempts to push a new mutation operation into the bounded delta.
    pub const fn push(mut self, op: Construct8Op) -> Result<Self, &'static str> {
        if self.len < 8 {
            self.ops[self.len as usize] = op;
            self.len += 1;
            Ok(self)
        } else {
            Err("CONSTRUCT8 violation: delta exceeded 8 mutations")
        }
    }"""
const8_new = """#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Construct8Error {
    CapacityExceeded,
}

    /// Attempts to push a new mutation operation into the bounded delta.
    pub const fn push(mut self, op: Construct8Op) -> Result<Self, Construct8Error> {
        if self.len < 8 {
            self.ops[self.len as usize] = op;
            self.len += 1;
            Ok(self)
        } else {
            Err(Construct8Error::CapacityExceeded)
        }
    }"""
replace_in_file('../insa/insa-hotpath/src/construct8.rs', const8_old, const8_new.replace("pub enum Construct8Error", "    /* temporary padding hack to fix indentation if needed */\n}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Construct8Error").replace("    /* temporary padding hack to fix indentation if needed */\n}\n\n", ""))
# A more robust replacement for construct8.rs:
with open('../insa/insa-hotpath/src/construct8.rs', 'r') as f:
    c8 = f.read()
if "pub enum Construct8Error" not in c8:
    c8 = c8.replace(
"""    pub const fn push(mut self, op: Construct8Op) -> Result<Self, &'static str> {
        if self.len < 8 {
            self.ops[self.len as usize] = op;
            self.len += 1;
            Ok(self)
        } else {
            Err("CONSTRUCT8 violation: delta exceeded 8 mutations")
        }
    }""",
"""    pub const fn push(mut self, op: Construct8Op) -> Result<Self, Construct8Error> {
        if self.len < 8 {
            self.ops[self.len as usize] = op;
            self.len += 1;
            Ok(self)
        } else {
            Err(Construct8Error::CapacityExceeded)
        }
    }""")
    c8 = c8.replace("impl Construct8Delta {", 
"""#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Construct8Error {
    CapacityExceeded,
}

impl Construct8Delta {""")
    with open('../insa/insa-hotpath/src/construct8.rs', 'w') as f:
        f.write(c8)
    print("Updated construct8.rs robustly")

# 4. Fix cog8.rs
with open('../insa/insa-hotpath/src/cog8.rs', 'r') as f:
    cg = f.read()
if "pub enum GraphError" not in cg:
    cg = cg.replace(
"""pub fn execute_cog8_graph(
    nodes: &[Cog8Row],
    present: u64,
    mut completed: u64,
) -> Result<Cog8Decision, &'static str> {""",
"""#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphError {
    NodeOutOfBounds,
}

#[inline(always)]
pub fn execute_cog8_graph(
    nodes: &[Cog8Row],
    present: u64,
    mut completed: u64,
) -> Result<Cog8Decision, GraphError> {""")
    cg = cg.replace('return Err("Node out of bounds");', 'return Err(GraphError::NodeOutOfBounds);')
    # Cleanup duplicate inline(always) if there are any
    cg = cg.replace("#[inline(always)]\n#[derive(Debug", "#[derive(Debug") 
    with open('../insa/insa-hotpath/src/cog8.rs', 'w') as f:
        f.write(cg)
    print("Updated cog8.rs robustly")

# 5. Fix blackboard.rs
bb_old = """    pub fn push(&mut self, slot: EvidenceSlot) -> Result<(), &'static str> {
        if self.len < 16 {
            self.slots[self.len as usize] = slot;
            self.len += 1;
            Ok(())
        } else {
            Err("Blackboard full")
        }
    }"""
bb_new = """    pub fn push(&mut self, slot: EvidenceSlot) -> Result<(), BlackboardError> {
        if self.len < 16 {
            self.slots[self.len as usize] = slot;
            self.len += 1;
            Ok(())
        } else {
            Err(BlackboardError::CapacityExceeded)
        }
    }"""
with open('../insa/insa-kappa8/src/fuse_hearsay/blackboard.rs', 'r') as f:
    bb = f.read()
if "pub enum BlackboardError" not in bb:
    bb = bb.replace(bb_old, bb_new)
    bb = bb.replace("impl Blackboard {", 
"""#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlackboardError {
    CapacityExceeded,
}

impl Blackboard {""")
    with open('../insa/insa-kappa8/src/fuse_hearsay/blackboard.rs', 'w') as f:
        f.write(bb)
    print("Updated blackboard.rs robustly")

# 6. Fix candidate.rs
c_path = '../insa/insa-kappa8/src/reconstruct_dendral/candidate.rs'
if os.path.exists(c_path):
    with open(c_path, 'r') as f:
        c_cand = f.read()
    if "pub enum CandidateError" not in c_cand:
        c_cand = c_cand.replace("Result<(), &'static str>", "Result<(), CandidateError>")
        c_cand = c_cand.replace('Err("Candidate list full")', "Err(CandidateError::CapacityExceeded)")
        c_cand = c_cand.replace("impl CandidateList {",
"""#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateError {
    CapacityExceeded,
}

impl CandidateList {""")
        with open(c_path, 'w') as f:
            f.write(c_cand)
        print("Updated candidate.rs robustly")

print("Done fixing errors.")
