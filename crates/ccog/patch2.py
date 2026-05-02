import sys

with open("../insa/insa-hotpath/src/powl8.rs", "r") as f:
    content = f.read()

content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\n#[repr(u8)]\npub enum Powl8Op {\n    NoOp = 0,", "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]\n#[repr(u8)]\npub enum Powl8Op {\n    #[default]\n    NoOp = 0,")

content = content.replace("""impl Default for Powl8Op {
    fn default() -> Self {
        Self::NoOp
    }
}
""", "")

with open("../insa/insa-hotpath/src/powl8.rs", "w") as f:
    f.write(content)
