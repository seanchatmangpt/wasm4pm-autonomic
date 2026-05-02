import os

# 1. cog8.rs
cog8_rs = "../insa/insa-hotpath/src/cog8.rs"
with open(cog8_rs, "r") as f:
    code = f.read()

# Change inline(always) to inline
code = code.replace("#[inline(always)]\npub fn execute_cog8_graph", "#[inline]\npub fn execute_cog8_graph")
# Remove Result
code = code.replace("Result<Cog8Decision, &'static str>", "Cog8Decision")
code = code.replace("Ok(best)", "best")
with open(cog8_rs, "w") as f:
    f.write(code)

# 2. jtbd_access_drift.rs
access_drift = "../insa/insa-truthforge/tests/jtbd_access_drift.rs"
with open(access_drift, "r") as f:
    code = f.read()
code = code.replace('.expect("Graph execution failed")', "")
with open(access_drift, "w") as f:
    f.write(code)

# 3. execute_gates.rs
exec_gates = "../insa/insa-truthforge/tests/execute_gates.rs"
with open(exec_gates, "r") as f:
    code = f.read()
code = code.replace(".unwrap()", "")
with open(exec_gates, "w") as f:
    f.write(code)

# 4. kappa_shrdlu.rs
shrdlu = "../insa/insa-truthforge/tests/kappa_shrdlu.rs"
with open(shrdlu, "r") as f:
    code = f.read()
code = code.replace("assert_eq!(res.resolved_object.unwrap().0, 100);", "assert_eq!(res.resolved_object.map(|o| o.0), Some(100));")
with open(shrdlu, "w") as f:
    f.write(code)

# 5. kappa_eliza.rs
eliza = "../insa/insa-truthforge/tests/kappa_eliza.rs"
with open(eliza, "r") as f:
    code = f.read()
code = code.replace("assert_eq!(res.selected_pattern.unwrap().0, 1);", "assert_eq!(res.selected_pattern.map(|p| p.0), Some(1));")
with open(eliza, "w") as f:
    f.write(code)

