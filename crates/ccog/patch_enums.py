import sys
import re

files = [
    ("../insa/insa-kappa8/src/fuse_hearsay/result.rs", "FusionStatus", "Incomplete = 1"),
    ("../insa/insa-kappa8/src/ground_shrdlu/result.rs", "GroundingStatus", "Missing = 2"),
    ("../insa/insa-kappa8/src/prove_prolog/result.rs", "ProofStatus", "Failed = 1"),
    ("../insa/insa-kappa8/src/reflect_eliza/result.rs", "ReflectStatus", "NoMatch = 2"),
    ("../insa/insa-kappa8/src/rule_mycin/result.rs", "MycinStatus", "NoMatch = 2"),
]

for filepath, enum_name, default_var in files:
    with open(filepath, "r") as f:
        content = f.read()
    
    # 1. Add Default to derive
    content = re.sub(r'#\[derive\(([^\]]+)\)\]\s*#\[repr\(u8\)\]\s*pub enum ' + enum_name, r'#[derive(\1, Default)]\n#[repr(u8)]\npub enum ' + enum_name, content)
    
    # 2. Add #[default] to the default variant
    content = content.replace(default_var, "#[default]\n    " + default_var)
    
    with open(filepath, "w") as f:
        f.write(content)

