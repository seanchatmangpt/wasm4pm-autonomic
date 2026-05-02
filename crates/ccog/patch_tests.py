import sys

with open("../insa/insa-truthforge/tests/kappa8_engines.rs", "r") as f:
    content = f.read()

content = content.replace("max_depth: 5", "")
content = content.replace("operators: OPS1, ", "operators: OPS1")
content = content.replace("operators: OPS2, ", "operators: OPS2")

with open("../insa/insa-truthforge/tests/kappa8_engines.rs", "w") as f:
    f.write(content)
