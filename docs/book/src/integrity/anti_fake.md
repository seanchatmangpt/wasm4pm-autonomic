# Anti-Fake Gauntlet

The Anti-Fake Gauntlet is a suite of automated Kill Zones that physically prove the system is not hardcoded, shallow, or fake.

## Kill Zones

| Zone | Invariant | Proof |
|------|-----------|-------|
| KZ1 | Doctrine Drift | Semantic lattice consistency |
| KZ2 | Causal Dependence | Perturbation-based causal testing |
| KZ6 | Performance Honesty | Zero-allocation hot path verification |
| KZ7 | Runtime Reality | E2E supply chain replay |

## Evidence
Every run generates a signed `ANTI_FAKE_EVIDENCE.log`, which captures the git commit hash, toolchain state, and raw cargo test outputs. This serves as the definitive audit trail for system admissibility.
