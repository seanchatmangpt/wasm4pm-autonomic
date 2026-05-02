import sys

with open("../insa/insa-kappa8/src/reflect_eliza.rs", "r") as f:
    content = f.read()

content = content.replace("if let Some(_) = self.detect_slot_gap(ctx) {", "if self.detect_slot_gap(ctx).is_some() {")

with open("../insa/insa-kappa8/src/reflect_eliza.rs", "w") as f:
    f.write(content)
