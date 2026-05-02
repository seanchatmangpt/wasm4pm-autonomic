import sys

files = [
    ("../insa/insa-kappa8/src/fuse_hearsay/result.rs", "FusionStatus", "Incomplete", "Complete, Incomplete"),
    ("../insa/insa-kappa8/src/ground_shrdlu/result.rs", "GroundingStatus", "Missing", "Resolved, Ambiguous, Missing"),
    ("../insa/insa-kappa8/src/prove_prolog/result.rs", "ProofStatus", "Failed", "Proved, Failed, FactMissing, Contradiction, DepthExhausted, CycleDetected, RequiresEscalation"),
    ("../insa/insa-kappa8/src/reflect_eliza/result.rs", "ReflectStatus", "NoMatch", "Matched, Incomplete, NoMatch"),
    ("../insa/insa-kappa8/src/rule_mycin/result.rs", "MycinStatus", "NoMatch", "Fired, Conflict, NoMatch"),
]

for filepath, enum_name, def_var, all_vars in files:
    with open(filepath, "r") as f:
        content = f.read()
    
    # We will just manually reconstruct the enum definitions
    # Strip everything between pub enum { ... }
    import re
    
    variants = [v.strip() for v in all_vars.split(",")]
    enum_body = ""
    for i, v in enumerate(variants):
        if v == def_var:
            enum_body += f"    #[default]\n    {v} = {i},\n"
        else:
            enum_body += f"    {v} = {i},\n"
            
    pattern = r'(#\[derive\([^\]]+\)\]\s*(?:#\[repr\(u8\)\]\s*)?pub enum ' + enum_name + r'\s*\{)(.*?)(\})'
    
    def replacer(match):
        return match.group(1) + "\n" + enum_body + match.group(3)
        
    content = re.sub(pattern, replacer, content, flags=re.DOTALL)
    
    with open(filepath, "w") as f:
        f.write(content)

