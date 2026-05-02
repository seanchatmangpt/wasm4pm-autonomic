import sys

# 1. fuse_hearsay/result.rs
with open("../insa/insa-kappa8/src/fuse_hearsay/result.rs", "r") as f:
    content = f.read()
content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum FusionStatus {\n    Complete = 0,\n    Incomplete = 1,\n}", "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\npub enum FusionStatus {\n    Complete = 0,\n    #[default]\n    Incomplete = 1,\n}")
content = content.replace("impl Default for FusionStatus {\n    fn default() -> Self {\n        Self::Incomplete\n    }\n}\n", "")
with open("../insa/insa-kappa8/src/fuse_hearsay/result.rs", "w") as f:
    f.write(content)

# 2. ground_shrdlu/result.rs
with open("../insa/insa-kappa8/src/ground_shrdlu/result.rs", "r") as f:
    content = f.read()
content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum GroundingStatus {\n    Resolved = 0,\n    Ambiguous = 1,\n    Missing = 2,\n}", "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\npub enum GroundingStatus {\n    Resolved = 0,\n    Ambiguous = 1,\n    #[default]\n    Missing = 2,\n}")
content = content.replace("impl Default for GroundingStatus {\n    fn default() -> Self {\n        Self::Missing\n    }\n}\n", "")
with open("../insa/insa-kappa8/src/ground_shrdlu/result.rs", "w") as f:
    f.write(content)

# 3. prove_prolog/result.rs
with open("../insa/insa-kappa8/src/prove_prolog/result.rs", "r") as f:
    content = f.read()
content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum ProofStatus {\n    Proved = 0,\n    Failed = 1,\n}", "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\npub enum ProofStatus {\n    Proved = 0,\n    #[default]\n    Failed = 1,\n}")
content = content.replace("impl Default for ProofStatus {\n    fn default() -> Self {\n        Self::Failed\n    }\n}\n", "")
with open("../insa/insa-kappa8/src/prove_prolog/result.rs", "w") as f:
    f.write(content)

# 4. reconstruct_dendral/candidate.rs
with open("../insa/insa-kappa8/src/reconstruct_dendral/candidate.rs", "r") as f:
    content = f.read()
if "impl Default for CandidateArena" not in content:
    content += """
impl Default for CandidateArena {
    fn default() -> Self {
        Self::new()
    }
}
"""
with open("../insa/insa-kappa8/src/reconstruct_dendral/candidate.rs", "w") as f:
    f.write(content)

# 5 & 6. reconstruct_dendral/engine.rs
with open("../insa/insa-kappa8/src/reconstruct_dendral/engine.rs", "r") as f:
    content = f.read()
content = content.replace("""                            if (cand.fragments_used & (1 << b) != 0) && (cand.fragments_used & (1 << a) != 0) {
                                if self.fragments[b].time.start > self.fragments[a].time.start {
                                    violated = true;""", """                            if (cand.fragments_used & (1 << b) != 0) && (cand.fragments_used & (1 << a) != 0) && self.fragments[b].time.start > self.fragments[a].time.start {
                                violated = true;""")
content = content.replace("""                            if (cand.fragments_used & (1 << ai) != 0) && (cand.fragments_used & (1 << bi) != 0) {
                                if self.fragments[ai].object != self.fragments[bi].object {
                                    violated = true;""", """                            if (cand.fragments_used & (1 << ai) != 0) && (cand.fragments_used & (1 << bi) != 0) && self.fragments[ai].object != self.fragments[bi].object {
                                violated = true;""")
with open("../insa/insa-kappa8/src/reconstruct_dendral/engine.rs", "w") as f:
    f.write(content)

# 7. reflect_eliza/result.rs
with open("../insa/insa-kappa8/src/reflect_eliza/result.rs", "r") as f:
    content = f.read()
content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum ReflectStatus {\n    Matched = 0,\n    Incomplete = 1,\n    NoMatch = 2,\n}", "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\npub enum ReflectStatus {\n    Matched = 0,\n    Incomplete = 1,\n    #[default]\n    NoMatch = 2,\n}")
content = content.replace("impl Default for ReflectStatus {\n    fn default() -> Self {\n        Self::NoMatch\n    }\n}\n", "")
with open("../insa/insa-kappa8/src/reflect_eliza/result.rs", "w") as f:
    f.write(content)

# 8. rule_mycin/result.rs
with open("../insa/insa-kappa8/src/rule_mycin/result.rs", "r") as f:
    content = f.read()
content = content.replace("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum MycinStatus {\n    Fired = 0,\n    Conflict = 1,\n    NoMatch = 2,\n}", "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\npub enum MycinStatus {\n    Fired = 0,\n    Conflict = 1,\n    #[default]\n    NoMatch = 2,\n}")
content = content.replace("impl Default for MycinStatus {\n    fn default() -> Self {\n        Self::NoMatch\n    }\n}\n", "")
with open("../insa/insa-kappa8/src/rule_mycin/result.rs", "w") as f:
    f.write(content)

