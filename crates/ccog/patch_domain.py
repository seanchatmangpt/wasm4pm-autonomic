import sys

with open("../insa/insa-types/src/domain.rs", "r") as f:
    content = f.read()

content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub struct DictionaryDigest(pub [u8; 32]);", "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]\npub struct DictionaryDigest(pub [u8; 32]);")

content = content.replace("""impl Default for DictionaryDigest {
    fn default() -> Self {
        Self([0; 32])
    }
}
""", "")

with open("../insa/insa-types/src/domain.rs", "w") as f:
    f.write(content)
