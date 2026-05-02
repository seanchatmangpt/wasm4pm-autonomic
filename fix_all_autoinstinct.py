import os
import glob
import re

def fix_file(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    original = content

    # Fix PackId and RuleId in LoadedFieldPack and LoadedPackRule
    content = re.sub(r'name:\s*"([^"]+)"\.to_string\(\),', r'name: ccog::packs::PackId("\1"),', content)
    content = re.sub(r'id:\s*"([^"]+)"\.to_string\(\),', r'id: ccog::packs::RuleId("\1"),', content)
    content = content.replace('id: rule_id.to_string(),', 'id: ccog::packs::RuleId(Box::leak(rule_id.to_string().into_boxed_str())),')

    # GroupId
    content = re.sub(r'decision\.matched_group_id\.as_deref\(\)', 'decision.matched_group_id.map(|g| g.0)', content)
    content = re.sub(r'([a-zA-Z0-9_]+)\.matched_group_id\.as_deref\(\)', r'\1.matched_group_id.map(|g| g.0)', content)
    content = re.sub(r'([a-zA-Z0-9_]+)\.matched_pack_id\.as_deref\(\)', r'\1.matched_pack_id.map(|p| p.0)', content)
    content = re.sub(r'([a-zA-Z0-9_]+)\.matched_rule_id\.as_deref\(\)', r'\1.matched_rule_id.map(|r| r.0)', content)

    # ---------------------------------------------------------
    # ClosedFieldContext replacements
    # ---------------------------------------------------------
    
    # 1. select_instinct_v0(&snap, &posture, &ctx) -> context bundle
    # Wait, variable names might differ!
    # Let's use a regex to find all select_instinct_v0 calls that take 3 args
    def repl_v0(match):
        snap_var, post_var, ctx_var = match.groups()
        snap_var = snap_var.replace('&', '')
        post_var = post_var.replace('&', '')
        ctx_var = ctx_var.replace('&', '')
        return f"""{{
            let context_bundle = ccog::runtime::ClosedFieldContext {{
                snapshot: std::sync::Arc::new({snap_var}.clone()),
                posture: {post_var}.clone(),
                context: {ctx_var}.clone(),
                tiers: ccog::packs::TierMasks::ZERO,
                human_burden: 0,
            }};
            select_instinct_v0(&context_bundle)
        }}"""
    
    content = re.sub(r'select_instinct_v0\s*\(\s*([^,]+),\s*([^,]+),\s*([^)]+)\)', repl_v0, content)

    # 2. select_instinct_with_pack(&snap, &posture, &ctx, &pack)
    def repl_pack(match):
        snap_var, post_var, ctx_var, pack_var = match.groups()
        snap_var = snap_var.replace('&', '')
        post_var = post_var.replace('&', '')
        ctx_var = ctx_var.replace('&', '')
        return f"""{{
            let context_bundle = ccog::runtime::ClosedFieldContext {{
                snapshot: std::sync::Arc::new({snap_var}.clone()),
                posture: {post_var}.clone(),
                context: {ctx_var}.clone(),
                tiers: ccog::packs::TierMasks::ZERO,
                human_burden: 0,
            }};
            select_instinct_with_pack(&context_bundle, {pack_var})
        }}"""
    content = re.sub(r'select_instinct_with_pack\s*\(\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^)]+)\)', repl_pack, content)

    # 3. select_instinct_with_pack_tiered(&snap, &posture, &ctx, &tiers, &pack)
    def repl_tiered(match):
        snap_var, post_var, ctx_var, tiers_var, pack_var = match.groups()
        snap_var = snap_var.replace('&', '')
        post_var = post_var.replace('&', '')
        ctx_var = ctx_var.replace('&', '')
        tiers_var = tiers_var.replace('&', '')
        return f"""{{
            let context_bundle = ccog::runtime::ClosedFieldContext {{
                snapshot: std::sync::Arc::new({snap_var}.clone()),
                posture: {post_var}.clone(),
                context: {ctx_var}.clone(),
                tiers: {tiers_var}.clone(),
                human_burden: 0,
            }};
            select_instinct_with_pack_tiered(&context_bundle, {pack_var})
        }}"""
    content = re.sub(r'select_instinct_with_pack_tiered\s*\(\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^)]+)\)', repl_tiered, content)

    # 4. ccog::bark_artifact::decide(&snap) or decide(&snap) in differential tests
    # wait, anti_fake_differential calls `decide(&snap)`. 
    # Let's replace `decide(&snap)` with the closed context
    def repl_decide(match):
        snap_var = match.group(1)
        snap_var = snap_var.replace('&', '')
        return f"""{{
            let context_bundle = ccog::runtime::ClosedFieldContext {{
                snapshot: std::sync::Arc::new({snap_var}.clone()),
                posture: ccog::multimodal::PostureBundle::default(),
                context: ccog::multimodal::ContextBundle::default(),
                tiers: ccog::packs::TierMasks::ZERO,
                human_burden: 0,
            }};
            decide(&context_bundle)
        }}"""
    content = re.sub(r'decide\s*\(\s*([^,]+)\s*\)', repl_decide, content)


    if content != original:
        print(f"Fixed {file_path}")
        with open(file_path, 'w') as f:
            f.write(content)


for file_path in glob.glob("crates/autoinstinct/tests/*.rs"):
    fix_file(file_path)

# Also fix the world_corpus.rs missing types
world_corpus_path = "crates/autoinstinct/src/llm/world_corpus.rs"
if os.path.exists(world_corpus_path):
    with open(world_corpus_path, 'r') as f:
        wc = f.read()
    
    if "std::collections::BTreeMap" not in wc:
        wc = wc.replace("use serde_json::json;", "use serde_json::json;\nuse std::collections::BTreeMap;\nuse crate::llm::{OcelObject, Counterfactual};")
        with open(world_corpus_path, 'w') as f:
            f.write(wc)

