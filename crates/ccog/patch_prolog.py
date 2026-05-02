import sys

with open("../insa/insa-kappa8/src/prove_prolog.rs", "r") as f:
    lines = f.readlines()

new_lines = []
skip = False
for i, line in enumerate(lines):
    if "if fact.relation == goal.relation && fact.subject == goal.subject && fact.object == goal.object {" in line:
        new_lines.append("            if fact.relation == goal.relation && fact.subject == goal.subject && fact.object == goal.object && fact.policy_epoch.0 <= ctx.policy.0 && fact.validity.0 > 0 {\n")
        skip = True
    elif skip and "if fact.policy_epoch.0 <= ctx.policy.0 && fact.validity.0 > 0 {" in line:
        continue # skipped the inner if
    elif skip and "            }" in line and "}" in lines[i-1]:
        # we reached the end of the outer if block which had double }}
        new_lines.pop() # remove the previous line's newline if needed, or just don't append
        skip = False
    else:
        new_lines.append(line)

with open("../insa/insa-kappa8/src/prove_prolog.rs", "w") as f:
    f.writelines(new_lines)
