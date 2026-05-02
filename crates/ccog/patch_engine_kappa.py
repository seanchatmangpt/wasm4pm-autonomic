import sys
import re

with open("../insa/insa-kappa8/src/reconstruct_dendral/engine.rs", "r") as f:
    content = f.read()

# Fix the broken dendral: dendral\n.union
content = re.sub(r'dendral: dendral\s+\.union', 'detail: dendral\n                    .union', content)
content = re.sub(r'dendral: dendral', 'detail: dendral', content) # catch-all just in case

# Add kappa: KappaByte::RECONSTRUCT to ReconstructionResult struct instantiations
# We can just do a regex replace on 'detail:' -> 'kappa: KappaByte::RECONSTRUCT, detail:'
content = content.replace("detail:", "kappa: KappaByte::RECONSTRUCT,\n                detail:")

with open("../insa/insa-kappa8/src/reconstruct_dendral/engine.rs", "w") as f:
    f.write(content)
