import sys

with open("../insa/xtask/Cargo.toml", "r") as f:
    content = f.read()

if "insa-instinct" not in content:
    content += """
insa-instinct = { path = "../insa-instinct" }
insa-types = { path = "../insa-types" }
insa-kappa8 = { path = "../insa-kappa8" }
"""

with open("../insa/xtask/Cargo.toml", "w") as f:
    f.write(content)
