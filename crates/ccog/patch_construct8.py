import sys

with open("../insa/insa-hotpath/src/construct8.rs", "r") as f:
    content = f.read()

content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n#[repr(u8)]\npub enum Construct8OpKind {\n    /// No operation / empty slot.\n    None = 0,", "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\n#[repr(u8)]\npub enum Construct8OpKind {\n    /// No operation / empty slot.\n    #[default]\n    None = 0,")

with open("../insa/insa-hotpath/src/construct8.rs", "w") as f:
    f.write(content)
