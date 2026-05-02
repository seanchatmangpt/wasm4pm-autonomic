import os

# 1. Patch xtask/src/main.rs
xtask_main = "../insa/xtask/src/main.rs"
with open(xtask_main, "r") as f:
    xtask_code = f.read()

xtask_code = xtask_code.replace("golden(&action)?", "golden(&action)")
xtask_code = xtask_code.replace("replay(&action)?", "replay(&action)")
xtask_code = xtask_code.replace("truthforge()?", "truthforge()")
xtask_code = xtask_code.replace('format!("Unknown xtask: {}", unknown)', 'format!("Unknown xtask: {unknown}")')
xtask_code = xtask_code.replace('println!("Golden wire encoding action: {}", action);', 'println!("Golden wire encoding action: {action}");')
xtask_code = xtask_code.replace('println!("POWL64 Replay action: {}", action);', 'println!("POWL64 Replay action: {action}");')
xtask_code = xtask_code.replace('fn golden(action: &str) -> Result<(), String> {', 'fn golden(action: &str) {')
xtask_code = xtask_code.replace('fn replay(action: &str) -> Result<(), String> {', 'fn replay(action: &str) {')
xtask_code = xtask_code.replace('fn truthforge() -> Result<(), String> {', 'fn truthforge() {')
xtask_code = xtask_code.replace('    Ok(())\n}', '}')
xtask_code = xtask_code.replace('println!("Value: {:#010b} ({})", parsed_val, parsed_val);', 'println!("Value: {parsed_val:#010b} ({parsed_val})");')
xtask_code = xtask_code.replace('println!("  - Bit {}: {}", i, label);', 'println!("  - Bit {i}: {label}");')

with open(xtask_main, "w") as f:
    f.write(xtask_code)

# 2. Patch insa-types/src/powl8_op.rs
powl8_op = "../insa/insa-types/src/powl8_op.rs"
with open(powl8_op, "r") as f:
    powl_code = f.read()

error_def = """
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Powl8OpError {
    InvalidDiscriminant,
}

impl core::fmt::Display for Powl8OpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid Powl8Op discriminant")
    }
}

impl std::error::Error for Powl8OpError {}
"""
powl_code = powl_code.replace("type Error = &'static str;", error_def + "\n    type Error = Powl8OpError;")
powl_code = powl_code.replace("Ok(Powl8Op::", "Ok(Self::")
powl_code = powl_code.replace('Err("Invalid Powl8Op discriminant")', "Err(Powl8OpError::InvalidDiscriminant)")

with open(powl8_op, "w") as f:
    f.write(powl_code)

# 3. Patch insa-types/src/mask.rs
mask_rs = "../insa/insa-types/src/mask.rs"
with open(mask_rs, "r") as f:
    mask_code = f.read()

mask_error_def = """
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskError {
    OutOfRange,
}

impl core::fmt::Display for MaskError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutOfRange => write!(f, "FieldBit must be in range [0, 63]"),
        }
    }
}

impl std::error::Error for MaskError {}
"""

# Replace inline(always) with inline and add must_use
mask_code = mask_code.replace("#[inline(always)]\n    pub const fn empty", "#[inline]\n    #[must_use]\n    pub const fn empty")
mask_code = mask_code.replace("#[inline(always)]\n    pub const fn is_empty", "#[inline]\n    #[must_use]\n    pub const fn is_empty")
mask_code = mask_code.replace("#[inline(always)]\n    pub const fn with_bit", "#[inline]\n    #[must_use]\n    pub const fn with_bit")
mask_code = mask_code.replace("#[inline(always)]\n    pub const fn new_checked", "#[inline]\n    pub const fn new_checked")
mask_code = mask_code.replace("#[inline(always)]\n    pub const fn new_unchecked", "#[inline]\n    #[must_use]\n    pub const fn new_unchecked")
mask_code = mask_code.replace("#[inline(always)]\n    pub const fn get", "#[inline]\n    #[must_use]\n    pub const fn get")

mask_code = mask_code.replace("Result<Self, &'static str>", "Result<Self, MaskError>")
mask_code = mask_code.replace('Err("FieldBit must be in range [0, 63]")', 'Err(MaskError::OutOfRange)')

mask_code = mask_code + "\n" + mask_error_def

with open(mask_rs, "w") as f:
    f.write(mask_code)
