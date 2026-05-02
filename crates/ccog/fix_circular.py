import os

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

# We are going to move Powl8Op to insa-types to break the cycle.

powl8_op_content = """//! POWL8 Operation primitive.

/// The operator for a process motion edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum Powl8Op {
    #[default]
    NoOp = 0,
    Act = 1,
    Choice = 2,
    Parallel = 3,
    Join = 4,
    Loop = 5,
    Block = 6,
    Silent = 7,
}

impl TryFrom<u8> for Powl8Op {
    type Error = &'static str;
    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(Powl8Op::NoOp),
            1 => Ok(Powl8Op::Act),
            2 => Ok(Powl8Op::Choice),
            3 => Ok(Powl8Op::Parallel),
            4 => Ok(Powl8Op::Join),
            5 => Ok(Powl8Op::Loop),
            6 => Ok(Powl8Op::Block),
            7 => Ok(Powl8Op::Silent),
            _ => Err("Invalid Powl8Op discriminant"),
        }
    }
}
"""

write_file('../insa/insa-types/src/powl8_op.rs', powl8_op_content)

# Patch insa-types/src/lib.rs
types_lib = '../insa/insa-types/src/lib.rs'
with open(types_lib, 'r') as f:
    content = f.read()

if "pub mod powl8_op;" not in content:
    content += "\npub mod powl8_op;\npub use powl8_op::*;\n"
    with open(types_lib, 'w') as f:
        f.write(content)

# Patch insa-hotpath/src/powl8.rs to use it from insa-types
hotpath_powl8 = '../insa/insa-hotpath/src/powl8.rs'
with open(hotpath_powl8, 'r') as f:
    content = f.read()

content = content.replace("pub enum Powl8Op", "/* pub enum Powl8Op moved */")
content = content.replace("impl TryFrom<u8> for Powl8Op", "/* impl TryFrom<u8> for Powl8Op moved */")

# Add the import
if "use insa_types::Powl8Op;" not in content:
    content = content.replace("use insa_types::{CompletedMask, EdgeId, NodeId};", "use insa_types::{CompletedMask, EdgeId, NodeId, Powl8Op};")

with open(hotpath_powl8, 'w') as f:
    f.write(content)


# Patch insa-hotpath/src/cog8.rs
cog8_path = '../insa/insa-hotpath/src/cog8.rs'
with open(cog8_path, 'r') as f:
    content = f.read()

if "use insa_types::{NodeId, EdgeId};" in content:
    content = content.replace("use insa_types::{NodeId, EdgeId};", "use insa_types::{NodeId, EdgeId, Powl8Op};")
    # removing the original definition
    content = content.replace(
"""/// The operator for a process motion edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Powl8Op {
    /// Terminal state reached.
    NoOp = 0,
    /// Execute the connected closure row.
    Act = 1,
}

impl Default for Powl8Op {
    fn default() -> Self {
        Self::NoOp
    }
}""", "/* Powl8Op definition moved to insa-types */")
    with open(cog8_path, 'w') as f:
        f.write(content)


# Update the operator.rs and fixtures.rs in reduce_gap_gps to point to insa-types instead of insa-hotpath
operator_path = '../insa/insa-kappa8/src/reduce_gap_gps/operator.rs'
with open(operator_path, 'r') as f:
    content = f.read()
content = content.replace("use insa_hotpath::powl8::Powl8Op;", "use insa_types::Powl8Op;")
with open(operator_path, 'w') as f:
    f.write(content)

fixtures_path = '../insa/insa-kappa8/src/reduce_gap_gps/fixtures.rs'
with open(fixtures_path, 'r') as f:
    content = f.read()
content = content.replace("use insa_hotpath::powl8::Powl8Op;", "use insa_types::Powl8Op;")
with open(fixtures_path, 'w') as f:
    f.write(content)

print("Powl8Op extracted to insa-types and paths updated.")
