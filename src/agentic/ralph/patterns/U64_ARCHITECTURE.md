# UniverseOS Architecture — Deterministic Dual-Plane L1 Execution

## Scope
UniverseOS is the deterministic operating environment for **Universe64**. It governs **lawful state motion** ($U_t \rightarrow U_{t+1}$) inside a 64 KiB L1D-class envelope.

## The Dual-Plane Model
- **Plane D (Data Plane):** 32 KiB resident state (`UniverseBlock = [u64; 4096]`).
- **Plane S (Scratch Plane):** 32 KiB workspace for operands, masks, deltas, and receipts.

## Operating Primitives
| Concept | Definition | Law |
| :--- | :--- | :--- |
| **UInstruction** | Tiny operator ID | Moves to the resident state; data does not move. |
| **UTransition** | Admissible move | $U_{t+1} = (U_t \land \neg I) \lor O$ iff $(U_t \land I) == I$. |
| **UDelta** | Event unit | $\Delta U = U_t \oplus U_{t+1}$. Exact change, no interpretation. |
| **UReceipt** | Memory unit | $R_{t+1} = mix(R_t, instruction, \Delta U)$. Proof of motion. |
| **UProjection** | UI/View unit | $View_i = \pi_i(U_t)$. Disposable derived state. |

## The Timing Constitution
- **T0 (Atom):** $\le 2\text{ns}$. Bit-level truth algebra.
- **T1 (Microkernel):** $\le 200\text{ns}$. Scoped transitions and delta building.
- **T2 (Orchestration):** $\le 5\mu\text{s}$. Full-universe scans (Hamming, logic checks).
- **T3 (Epoch):** $\le 100\mu\text{s}$. System synthesis and cryptographic provenance.

## Architectural Rules
1. **Data Plane is Resident:** The 32 KiB universe is never copied in the hot path.
2. **Scratch Plane is Bounded:** All staging and temporary motion must fit in 32 KiB.
3. **No Branches (CC=1):** Use mask-based selection for all T1/T2 admissibility.
4. **No Heap:** Zero allocations in the hot path.
5. **Delta-Driven:** Subscribers consume $\Delta U$, not universe snapshots.
6. **Receipted Motion:** Every state change must contribute to a deterministic proof chain.
