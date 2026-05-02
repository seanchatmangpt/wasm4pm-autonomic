import sys

with open("../insa/insa-kappa8/src/reconstruct_dendral/engine.rs", "r") as f:
    lines = f.readlines()

new_lines = []
skip = False
for i, line in enumerate(lines):
    if "if (cand.fragments_used & (1 << b) != 0) && (cand.fragments_used & (1 << a) != 0) {" in line:
        new_lines.append("                            if (cand.fragments_used & (1 << b) != 0)\n")
        new_lines.append("                                && (cand.fragments_used & (1 << a) != 0)\n")
        new_lines.append("                                && self.fragments[b].time.start > self.fragments[a].time.start\n")
        new_lines.append("                            {\n")
        skip = True
    elif "if self.fragments[b].time.start > self.fragments[a].time.start {" in line and skip:
        continue # skip inner if
    elif "if (cand.fragments_used & (1 << ai) != 0) && (cand.fragments_used & (1 << bi) != 0) {" in line:
        new_lines.append("                            if (cand.fragments_used & (1 << ai) != 0)\n")
        new_lines.append("                                && (cand.fragments_used & (1 << bi) != 0)\n")
        new_lines.append("                                && self.fragments[ai].object != self.fragments[bi].object\n")
        new_lines.append("                            {\n")
        skip = True
    elif "if self.fragments[ai].object != self.fragments[bi].object {" in line and skip:
        continue # skip inner if
    elif "                            }" in line and skip and "}" in lines[i-1]:
        # outer closing bracket
        skip = False
        # Do not append this closing bracket because we merged them
    else:
        new_lines.append(line)

with open("../insa/insa-kappa8/src/reconstruct_dendral/engine.rs", "w") as f:
    f.writelines(new_lines)

