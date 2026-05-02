import os

path = '../insa/insa-instinct/src/byte.rs'
with open(path, 'r') as f:
    content = f.read()

# We need to find the StripsByte impl and insert the contains method
impl_str = "impl StripsByte {"
contains_str = """
    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }"""

if "pub const fn contains" not in content.split("impl StripsByte {")[1].split("impl ShrdluByte {")[0]:
    # Replace the empty() block with empty() and contains()
    content = content.replace(
"""    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }""", 
"""    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }""" + contains_str)

with open(path, 'w') as f:
    f.write(content)
print("Patched StripsByte to include contains method.")
