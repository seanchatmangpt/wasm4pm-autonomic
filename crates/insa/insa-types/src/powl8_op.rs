//! POWL8 Operation primitive.

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
