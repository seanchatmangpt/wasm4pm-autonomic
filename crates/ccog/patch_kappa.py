import os
import glob

# 1. Allow some overly pedantic lints at the lib level
lib_rs = "../insa/insa-kappa8/src/lib.rs"
with open(lib_rs, "r") as f:
    lib_code = f.read()

allow_lints = """#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::suspicious_operation_groupings)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::pub_underscore_fields)]
"""

if not lib_code.startswith("#![allow"):
    lib_code = allow_lints + "\n" + lib_code
    with open(lib_rs, "w") as f:
        f.write(lib_code)

