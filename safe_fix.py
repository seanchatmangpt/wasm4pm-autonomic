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
    content = re.sub(r'([a-zA-Z0-9_]+)\.matched_group_id\.as_deref\(\)', r'\1.matched_group_id.map(|g| g.0)', content)
    content = re.sub(r'([a-zA-Z0-9_]+)\.matched_pack_id\.as_deref\(\)', r'\1.matched_pack_id.map(|p| p.0)', content)
    content = re.sub(r'([a-zA-Z0-9_]+)\.matched_rule_id\.as_deref\(\)', r'\1.matched_rule_id.map(|r| r.0)', content)

    lines = content.split('\n')
    for i, line in enumerate(lines):
        if line.strip().startswith('//') or line.strip().startswith('///') or line.strip().startswith('//!'):
            continue
        if '`' in line:
            continue
        
        line = re.sub(r'decide\(&([a-zA-Z0-9_]+)\)', 
                      r'decide(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(\1.clone()), posture: ccog::multimodal::PostureBundle::default(), context: ccog::multimodal::ContextBundle::default(), tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 })', 
                      line)
        
        line = re.sub(r'select_instinct_v0\(&([a-zA-Z0-9_]+),\s*&([a-zA-Z0-9_]+),\s*&([a-zA-Z0-9_]+)\)',
                      r'select_instinct_v0(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(\1.clone()), posture: \2.clone(), context: \3.clone(), tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 })',
                      line)

        line = re.sub(r'select_instinct_with_pack\(&([a-zA-Z0-9_]+),\s*&([a-zA-Z0-9_]+),\s*&([a-zA-Z0-9_]+),\s*&([a-zA-Z0-9_]+)\)',
                      r'select_instinct_with_pack(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(\1.clone()), posture: \2.clone(), context: \3.clone(), tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 }, &\4)',
                      line)

        line = re.sub(r'select_instinct_with_pack_tiered\(&([a-zA-Z0-9_]+),\s*&([a-zA-Z0-9_]+),\s*&([a-zA-Z0-9_]+),\s*&([a-zA-Z0-9_:]+),\s*&([a-zA-Z0-9_\(\)]+)\)',
                      r'select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(\1.clone()), posture: \2.clone(), context: \3.clone(), tiers: \4.clone(), human_burden: 0 }, &\5)',
                      line)

        lines[i] = line

    content = '\n'.join(lines)

    if content != original:
        with open(file_path, 'w') as f:
            f.write(content)

for file_path in glob.glob("crates/autoinstinct/tests/*.rs"):
    fix_file(file_path)

world_corpus_path = "crates/autoinstinct/src/llm/world_corpus.rs"
if os.path.exists(world_corpus_path):
    with open(world_corpus_path, 'r') as f:
        wc = f.read()
    
    if "std::collections::BTreeMap" not in wc:
        wc = wc.replace("use serde_json::json;", "use serde_json::json;\nuse std::collections::BTreeMap;\nuse crate::llm::schema::{OcelObject, Counterfactual};")
        with open(world_corpus_path, 'w') as f:
            f.write(wc)
