import sys

with open("../insa/insa-kappa8/src/prove_prolog.rs", "r") as f:
    content = f.read()

content = content.replace("""                return ProofResult {
                    status: ProofStatus::Proved,
                    prolog: PrologByte::empty().union(PrologByte::GOAL_PROVED),
                    kappa: KappaByte::PROVE,
                    emits: InstinctByte::SETTLE,
                    support: FieldMask::empty(), // Simulated: record support
                };""", """                return ProofResult {
                    status: ProofStatus::Proved,
                    prolog: PrologByte::empty().union(PrologByte::GOAL_PROVED),
                    kappa: KappaByte::PROVE,
                    emits: InstinctByte::SETTLE,
                    support: FieldMask::empty(), // Simulated: record support
                    witness: ProofWitness { steps_recorded: current_depth + 1 },
                };""")

with open("../insa/insa-kappa8/src/prove_prolog.rs", "w") as f:
    f.write(content)
