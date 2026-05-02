import os

def fix_clippy(path, enum_name, default_variant, start_replace, end_replace):
    with open(path, 'r') as f:
        content = f.read()

    # Find the enum and add the derive
    enum_decl = f"pub enum {enum_name} {{"
    derived_enum_decl = f"#[derive(Default)]\n{enum_decl}"
    
    if "#[derive(Default)]" not in content.split(enum_decl)[0].split("#[derive(Debug")[1]:
        content = content.replace(enum_decl, derived_enum_decl)
    
    # Add #[default] to the correct variant
    variant_decl = f"    {default_variant} ="
    if f"#[default]\n{variant_decl}" not in content:
        content = content.replace(variant_decl, f"    #[default]\n{variant_decl}")
        
    # Remove the manual impl
    manual_impl = f"""impl Default for {enum_name} {{
    fn default() -> Self {{
        Self::{default_variant}
    }}
}}
"""
    content = content.replace(manual_impl, "")
    
    # Optional specific removals
    if start_replace and end_replace:
        content = content.replace(start_replace, end_replace)

    with open(path, 'w') as f:
        f.write(content)

# 1. fuse_hearsay
fix_clippy(
    '../insa/insa-kappa8/src/fuse_hearsay/result.rs', 
    'FusionStatus', 'Incomplete', None, None)

# 2. ground_shrdlu
fix_clippy(
    '../insa/insa-kappa8/src/ground_shrdlu/result.rs', 
    'GroundingStatus', 'Missing', None, None)

# 3. prove_prolog
fix_clippy(
    '../insa/insa-kappa8/src/prove_prolog/result.rs', 
    'ProofStatus', 'Failed', None, None)

# fix unused import in prove_prolog fixtures
prove_fix_path = '../insa/insa-kappa8/src/prove_prolog/fixtures.rs'
with open(prove_fix_path, 'r') as f:
    p_content = f.read()
p_content = p_content.replace("ProofGoal", "")
p_content = p_content.replace("TermId, }", "TermId}")
p_content = p_content.replace(", }", "}")
with open(prove_fix_path, 'w') as f:
    f.write(p_content)

# 4. reconstruct_dendral
fix_clippy(
    '../insa/insa-kappa8/src/reconstruct_dendral/result.rs', 
    'DendralStatus', 'Failed', None, None)

# 5. reflect_eliza
fix_clippy(
    '../insa/insa-kappa8/src/reflect_eliza/result.rs', 
    'ReflectStatus', 'NoMatch', None, None)

# fix unused import in reflect_eliza pattern
eliza_fix_path = '../insa/insa-kappa8/src/reflect_eliza/pattern.rs'
with open(eliza_fix_path, 'r') as f:
    e_content = f.read()
e_content = e_content.replace("CompletedMask, ", "")
with open(eliza_fix_path, 'w') as f:
    f.write(e_content)

# 6. rule_mycin
fix_clippy(
    '../insa/insa-kappa8/src/rule_mycin/result.rs', 
    'MycinStatus', 'NoMatch', None, None)

print("Applied Clippy fixes across insa-kappa8.")
