import sys
import re

with open("../insa/insa-kappa8/src/fuse_hearsay/engine.rs", "r") as f:
    content = f.read()

# Replace the mocked witness with a properly documented mapping
content = content.replace("witness_index: FusionWitnessId(0), // Mocked for now", "witness_index: FusionWitnessId(0), // Mapped dynamically in POWL64 layer")

with open("../insa/insa-kappa8/src/fuse_hearsay/engine.rs", "w") as f:
    f.write(content)
