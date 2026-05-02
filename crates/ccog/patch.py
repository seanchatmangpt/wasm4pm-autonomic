import sys

with open("../insa/insa-kappa8/src/rule_mycin.rs", "r") as f:
    content = f.read()

content = content.replace("""            if req_met && forb_met {
                if rule.certainty.0 > max_certainty.0 {
                    max_certainty = rule.certainty;
                    best_rule = Some(rule);
                }
            }""", """            if req_met && forb_met && rule.certainty.0 > max_certainty.0 {
                max_certainty = rule.certainty;
                best_rule = Some(rule);
            }""")

with open("../insa/insa-kappa8/src/rule_mycin.rs", "w") as f:
    f.write(content)
