# HDIT Analysis: HDC and AutoML Results

**Date:** 2026-04-22  
**Experiment:** Test whether HDC (hyperdimensional trace encoding) or AutoML (greedy orthogonal signal selection) breaks the 67.78% accuracy ceiling on PDC 2025.

---

## Hypothesis

The 67.78% ceiling is due to **information loss in the approximate Petri net** — not weak algorithms.

- **HDC tests:** Can we encode traces directly without the net? (hyperdimensional projection)
- **AutoML tests:** Can we break signal correlation by greedy orthogonal selection? (fusion strategy)

---

## Results

### HDC (Hyperdimensional Trace Encoding)

| Metric | Value |
|--------|-------|
| Accuracy | **??%** |
| vs. Baseline | **??** |
| Key Insight | |

### AutoML (HDIT Orthogonal Selection)

| Metric | Value |
|--------|-------|
| Accuracy | **??%** |
| Signals Selected | **?** |
| Fusion Operator | **?** |
| vs. Baseline | **??** |
| Key Insight | |

---

## Analysis

### HDC Findings

**If HDC > 67.78%:**
- The approximate net WAS the bottleneck
- Temporal trace structure is discriminative
- Hypervector encoding preserves ~XXX% more signal than language membership

**If HDC ≈ 67.78%:**
- The data itself has limited discrimination
- Both projections (net + hypervector) hit the same ceiling
- Confirms HDIT thesis: information is the limit, not computation

### AutoML Findings

**If AutoML > 67.78%:**
- Correlation between signals IS a real constraint
- Greedy orthogonal selection finds better combinations than brute-force fusion
- Signals F/G/H are too similar; E/HDC add orthogonal information

**If AutoML ≈ 67.78%:**
- The selected signals are still bound by same information source
- Ceiling is structural, not about signal overlap

---

## Conclusion

TBD — waiting for binary results...
