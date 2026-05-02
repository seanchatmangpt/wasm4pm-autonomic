import re

file_path = "crates/ccog-bridge/src/lib.rs"
with open(file_path, 'r') as f:
    content = f.read()

# Fix BarkNodeTrace missing fields
extra_fields = """
            args_digest: [0; 32],
            collapse_fn: ccog::trace::CollapseFn::ExpertRule,
            input_digest: [0; 32],
            pack_id: None,
            group_id: None,
            rule_id: None,
            response: None,
            kappa: None,
"""
content = re.sub(
    r'(trace\.nodes\.push\(ccog::trace::BarkNodeTrace \{[^}]+?)(\s*\})',
    r'\1' + extra_fields + r'\2',
    content
)

# Fix Receipt::new args
content = content.replace(
    'Receipt::new(\n            "urn:blake3:deadbeef".to_string(),',
    'Receipt::new(\n            ccog::graph::GraphIri(oxigraph::model::NamedNode::new_unchecked("urn:blake3:deadbeef")),\n'
)

# Fix chrono
content = content.replace('chrono::Utc::now()', 'ccog::chrono::Utc::now()')

with open(file_path, 'w') as f:
    f.write(content)
