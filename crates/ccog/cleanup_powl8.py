import os

hotpath_powl8 = '../insa/insa-hotpath/src/powl8.rs'
with open(hotpath_powl8, 'r') as f:
    content = f.read()

# Remove the orphaned attributes
content = content.replace(
"""/// The operator for a process motion edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
/* pub enum Powl8Op moved */ {
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

/* impl TryFrom<u8> for Powl8Op moved */ {
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
}""", "")

with open(hotpath_powl8, 'w') as f:
    f.write(content)

print("Cleaned up orphaned attributes in powl8.rs.")
