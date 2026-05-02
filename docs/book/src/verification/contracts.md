# Contract Testing

We employ a strict contract-testing framework where both the *Producer* and the *Consumer* of a cognitive surface must prove the input/output contract.

## The Admissibility Gate
Every `ainst` action is gated by an admissibility contract. Before a policy is deployed:
1. **Schema Check**: The OCEL structure is validated for completeness.
2. **Gauntlet Admission**: The policy is challenged against generated scenarios.
3. **Artifact Manifest**: The pack is hashed and verified.

If any contract is breached, the admission is revoked.
