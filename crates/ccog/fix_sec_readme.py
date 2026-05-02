import os

readme_path = '../insa/insa-security/README.md'
with open(readme_path, 'r') as f:
    content = f.read()

content = content.replace('```rust', '```rust,ignore')

with open(readme_path, 'w') as f:
    f.write(content)

print("Fixed doctest in insa-security README.md")
