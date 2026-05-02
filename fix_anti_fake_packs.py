import re

file_path = "crates/autoinstinct/tests/anti_fake_packs.rs"
with open(file_path, 'r') as f:
    content = f.read()

# Replace all occurrences of load_compiled(...) to compute expected hash dynamically
# We look for `let (mut)? loaded = load_compiled(` up to `.expect(` or `)` 
# Since we just want to override `&artifact.digest_urn` or `&pack.digest_urn`, we can find it.

lines = content.split('\n')
for i, line in enumerate(lines):
    if 'load_compiled(' in line:
        # We need to insert the expected_urn logic BEFORE this line.
        # But this is a bit tricky if it's multiline. Let's just use regex.
        pass

# A simpler regex that replaces the last argument of load_compiled with a freshly computed one.
# It's multiline, so:
#         &artifact.digest_urn,
#     )
pattern1 = r'load_compiled\(\s*&([a-zA-Z0-9_]+)\.name,\s*&\1\.ontology_profile,\s*&\1\s*\.rules\s*\.iter\(\)\s*\.map\(\|\(k, v\)\| \(k\.clone\(\), format!\("\{\:\?\}", v\)\)\)\s*\.collect::<Vec<_>>\(\),\s*&format!\("\{\:\?\}", \1\.default_response\),\s*&\1\.digest_urn,\s*\)'

replacement1 = r"""{
        let rule_strs: Vec<_> = \1.rules.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect();
        let expected_hash = ccog::packs::compute_manifest_digest(&\1.name, &\1.ontology_profile, &rule_strs);
        let expected_urn = format!("urn:blake3:{}", expected_hash.to_hex());
        load_compiled(&\1.name, &\1.ontology_profile, &rule_strs, &format!("{:?}", \1.default_response), &expected_urn)
    }"""

content = re.sub(pattern1, replacement1, content)

# Another variant:
pattern2 = r'load_compiled\(\s*&([a-zA-Z0-9_]+)\.name,\s*&\1\.ontology_profile,\s*&([a-zA-Z0-9_]+),\s*&format!\("\{\:\?\}", \1\.default_response\),\s*&\1\.digest_urn,\s*\)'

replacement2 = r"""{
        let expected_hash = ccog::packs::compute_manifest_digest(&\1.name, &\1.ontology_profile, &\2);
        let expected_urn = format!("urn:blake3:{}", expected_hash.to_hex());
        load_compiled(&\1.name, &\1.ontology_profile, &\2, &format!("{:?}", \1.default_response), &expected_urn)
    }"""

content = re.sub(pattern2, replacement2, content)

with open(file_path, 'w') as f:
    f.write(content)
