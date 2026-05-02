import sys

with open("../insa/insa-kappa8/src/reduce_gap_gps.rs", "r") as f:
    content = f.read()

content = content.replace("GpsByte::GAP_CLOSED", "GpsByte::GAP_SMALL")
content = content.replace("GpsByte::SEARCH_EXHAUSTED", "GpsByte::NO_PROGRESS")
content = content.replace("GpsByte::OPERATOR_SELECTED", "GpsByte::OPERATOR_AVAILABLE")
content = content.replace("GpsByte::NO_OPERATOR_FOUND", "GpsByte::OPERATOR_BLOCKED")

with open("../insa/insa-kappa8/src/reduce_gap_gps.rs", "w") as f:
    f.write(content)
