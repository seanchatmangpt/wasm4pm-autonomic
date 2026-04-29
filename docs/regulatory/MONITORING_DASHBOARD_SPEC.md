# Monitoring Dashboard Specification

## Purpose

This document specifies the regulatory-grade monitoring dashboard. Every metric is real-time, immutable, and available to regulators. The dashboard enables continuous compliance verification and early detection of drift, fairness violations, and security threats.

---

## 1. Dashboard Overview

**Audience**: Regulators, auditors, internal stakeholders, customers  
**Refresh Rate**: 60 seconds (real-time)  
**Storage**: Time-series database (Prometheus or equivalent); 1-year retention  
**Access Control**: Role-based; audit trail logs all accesses  
**Accessibility**: WCAG 2.1 AA; responsive design

---

## 2. Core Metrics

### 2.1 Per-System Accuracy (Real-Time)

**Display**: Line chart, 7-day rolling window

| System ID | Current Accuracy | 7-day Avg | Baseline | Δ (drop) | Alert |
|-----------|------------------|-----------|----------|----------|-------|
| sys-001 | 96.3% | 96.1% | 96.2% | -0.2 pp | ✓ CLEAR |
| sys-002 | 95.8% | 95.9% | 95.8% | 0.0 pp | ✓ CLEAR |
| sys-003 | 95.4% | 95.2% | 95.8% | -0.4 pp | ✓ CLEAR |
| sys-004 | 94.1% | 94.3% | 94.5% | -0.4 pp | ✓ CLEAR |
| sys-005 | 96.8% | 96.5% | 96.2% | +0.6 pp | ✓ CLEAR |
| sys-006 | 93.2% | 93.0% | 93.5% | -0.3 pp | ✓ CLEAR |
| sys-007 | 92.1% | 92.4% | 92.8% | -0.7 pp | ✓ CLEAR |
| sys-008 | 94.7% | 94.6% | 94.8% | -0.1 pp | ✓ CLEAR |
| sys-009 | 93.9% | 93.8% | 94.2% | -0.3 pp | ✓ CLEAR |
| sys-010 | 95.2% | 95.3% | 95.5% | -0.3 pp | ✓ CLEAR |
| **ENSEMBLE (avg)** | **95.5%** | **95.4%** | **95.5%** | **0.0 pp** | **✓ CLEAR** |

**Thresholds**:
- **GREEN** (✓ CLEAR): ≥95% accuracy, drop <3 pp
- **YELLOW** (⚠ WARNING): 90–95% accuracy, or drop 3–5 pp
- **RED** (🔴 CRITICAL): <90% accuracy, or drop >5 pp

**Alert Rule**: If any system drops >5 pp in rolling 30-day window → trigger CRITICAL alert → notify ML Engineering Lead + CDO

---

### 2.2 Per-System Precision, Recall, F1

**Display**: Gauge chart with threshold indicators

| System | Precision | Recall | F1 | Status |
|--------|-----------|--------|-----|--------|
| sys-001 | 97.1% | 96.0% | 96.5% | ✓ PASS |
| sys-002 | 95.8% | 95.9% | 95.9% | ✓ PASS |
| sys-003 | 96.2% | 94.8% | 95.5% | ✓ PASS |
| sys-004 | 93.0% | 95.2% | 94.1% | ✓ PASS |
| sys-005 | 97.1% | 96.5% | 96.8% | ✓ PASS |
| sys-006 | 92.8% | 93.6% | 93.2% | ✓ PASS |
| sys-007 | 92.0% | 92.2% | 92.1% | ✓ PASS |
| sys-008 | 95.1% | 94.3% | 94.7% | ✓ PASS |
| sys-009 | 94.2% | 93.6% | 93.9% | ✓ PASS |
| sys-010 | 96.0% | 94.5% | 95.2% | ✓ PASS |

**Thresholds**:
- **GREEN** (✓ PASS): Precision ≥92%, Recall ≥92%, F1 ≥92%
- **YELLOW** (⚠ WARNING): Any metric 85–92%
- **RED** (🔴 CRITICAL): Any metric <85%

---

### 2.3 Ensemble Agreement Heatmap

**Display**: 10x10 heatmap; pairwise agreement % between systems

```
        sys-001  sys-002  sys-003  sys-004  sys-005  sys-006  sys-007  sys-008  sys-009  sys-010
sys-001  100%     92%      91%      88%      93%      87%      85%      90%      88%      91%
sys-002   92%     100%      93%      89%      94%      88%      86%      91%      89%      92%
sys-003   91%      93%     100%      87%      92%      86%      84%      89%      87%      90%
sys-004   88%      89%      87%     100%      89%      85%      82%      86%      84%      87%
sys-005   93%      94%      92%      89%     100%      89%      87%      92%      90%      93%
sys-006   87%      88%      86%      85%      89%     100%      81%      85%      83%      86%
sys-007   85%      86%      84%      82%      87%      81%     100%      83%      81%      84%
sys-008   90%      91%      89%      86%      92%      85%      83%     100%      88%      91%
sys-009   88%      89%      87%      84%      90%      83%      81%      88%     100%      89%
sys-010   91%      92%      90%      87%      93%      86%      84%      91%      89%     100%
```

**Color Coding**:
- **Dark Green** (>90%): Strong agreement
- **Light Green** (80–90%): Moderate agreement
- **Yellow** (70–80%): Weak agreement (investigate)
- **Red** (<70%): Conflicting systems (halt, investigate)

**Interpretation**: 
- Main diagonal: 100% (each system agrees with itself)
- Off-diagonal: Pairwise agreement; red cells indicate system conflicts
- **Example**: sys-007 and sys-004 have only 82% agreement (light green) → investigate rule conflicts

**Alert Rule**: If any pairwise agreement <70% → CRITICAL alert → escalate to ML Engineering Lead

---

### 2.4 Input Out-of-Distribution (OOD) Score

**Display**: Real-time histogram + time-series

**Histogram** (all traces in past 24 hours):

```
OOD Score Distribution (N=50,000 traces, last 24h)
0.0–0.5σ:  ██████████████████ 72% (36,000 traces)
0.5–1.0σ:  ███████  15% (7,500 traces)
1.0–1.5σ:  ███      7% (3,500 traces)
1.5–2.0σ:  ██        3% (1,500 traces)
2.0–2.5σ:  █        1.5% (750 traces)
2.5–3.0σ:  ▌        0.8% (400 traces)
3.0–3.5σ:  ▌        0.5% (250 traces)
>3.5σ:     ▌        0.2% (100 traces) — REJECTED
```

**Time-Series** (% traces exceeding threshold):

| Time | % OOD <1.0σ | % OOD 1–2σ | % OOD 2–3σ | % OOD >3σ (rejected) | Alert |
|------|-------------|-----------|-----------|----------------------|-------|
| 2026-04-28T13:00Z | 85% | 10% | 4% | 1% | ✓ CLEAR |
| 2026-04-28T14:00Z | 84% | 11% | 4% | 1% | ✓ CLEAR |
| 2026-04-28T15:00Z | 83% | 12% | 4% | 1% | ✓ CLEAR |
| 2026-04-28T16:00Z | 82% | 13% | 4% | 1% | ✓ CLEAR |

**Thresholds**:
- **GREEN** (✓ CLEAR): % OOD >3σ ≤ 5%
- **YELLOW** (⚠ WARNING): % OOD >3σ between 5% and 15%
- **RED** (🔴 CRITICAL): % OOD >3σ > 15%

**Alert Rule**: If % OOD >3σ continuously >15% for >6 hours → CRITICAL alert → trigger emergency retraining decision

---

### 2.5 Confidence Score Distribution

**Display**: Real-time histogram

```
System Agreement (Confidence) Distribution (N=50,000 traces, last 24h)
0.0–0.3 (0–3 systems agree):   ▌  0.2% (100 traces) — ESCALATED (human review)
0.3–0.5 (3–5 systems agree):   ███  3% (1,500 traces) — ESCALATED (human review)
0.5–0.7 (5–7 systems agree):   ████████  8% (4,000 traces) — TIER 2 (confidence-gated)
0.7–0.9 (7–9 systems agree):   ████████████████ 35% (17,500 traces) — TIER 2
0.9–1.0 (9–10 systems agree):  ████████████████████████████████ 54% (27,000 traces) — TIER 1 (automatic)
```

**Interpretation**:
- **Tier 1 (0.9–1.0)**: 54% of decisions are automatic (high confidence, ≥9/10 agreement)
- **Tier 2 (0.7–0.9)**: 43% of decisions require confidence-gated approval
- **Tier 3 (<0.7)**: 3% of decisions escalated to human review

**Alert Rule**: If % Tier 1 drops <30% → WARNING → investigate ensemble drift

---

## 3. Regulatory Thresholds and Escalations

### 3.1 Accuracy Thresholds

| Threshold | Status | Action | SLA |
|-----------|--------|--------|-----|
| Any system <95% | WARNING | Investigate; prepare retraining | 24 hours |
| Any system <90% | CRITICAL | Halt automated decisions; immediate retraining | 4 hours |
| Ensemble avg <95% | WARNING | Quarterly validation; check for drift | 24 hours |
| Ensemble avg <90% | CRITICAL | Emergency halt; full investigation | 4 hours |
| Accuracy drop >5 pp (30-day) | CRITICAL | Escalate to CTO + Risk; halt deployment | 4 hours |

### 3.2 Ensemble Agreement Thresholds

| Metric | Threshold | Action | SLA |
|--------|-----------|--------|-----|
| Pairwise agreement <70% | RED CELL | Investigate system conflict; halt if systematic | 24 hours |
| Average agreement <8/10 | WARNING | Investigate; potential fairness issue | 48 hours |
| Average agreement <6/10 | CRITICAL | Halt automated decisions; escalate | 4 hours |

### 3.3 OOD Detection Thresholds

| Metric | Threshold | Action | SLA |
|--------|-----------|--------|-----|
| % OOD >3σ = 5–15% | WARNING | Increase retraining frequency | 1 week |
| % OOD >3σ = 15–20% | CRITICAL | Immediate retraining decision | 24 hours |
| % OOD >3σ = >20% | CRITICAL HALT | Halt production; full system review | 4 hours |

### 3.4 Fairness Thresholds

| Metric | Threshold | Action | SLA |
|--------|-----------|--------|-----|
| Demographic parity (protected attr) >2% | OK | Monitor; routine audit in next quarter |  — |
| Demographic parity >5% | WARNING | Escalate to Compliance; plan fairness retraining | 1 week |
| Demographic parity >10% | CRITICAL | Halt production; emergency fairness audit + retraining | 24 hours |

---

## 4. Alert Configuration

### 4.1 Alert Channels

| Alert Level | Channels | Recipients | Escalation |
|-------------|----------|------------|------------|
| **INFO** | Dashboard display | Monitoring team | None |
| **WARNING** | Email + Slack | ML Engineering Lead | Escalate if unresolved in 24 hours |
| **CRITICAL** | SMS + Email + Phone call + PagerDuty | CTO + CDO + ML Lead + Compliance | Immediate (4-hour SLA) |

### 4.2 Alert Rules (Examples)

```
# Rule: Accuracy Drop Critical
if max(accuracy_drop_30d) > 0.05 then
  alert "CRITICAL: Accuracy drop > 5%"
  notify cto, cdo, ml_lead via SMS+PagerDuty
  sla: 4 hours
end

# Rule: OOD Detection Spike
if (traces_ood_gt_3sigma / total_traces) > 0.15 then
  alert "CRITICAL: OOD > 15%"
  notify ml_lead via email+slack
  sla: 24 hours
  action: trigger_emergency_retraining_decision
end

# Rule: Ensemble Conflict
if min(pairwise_agreement) < 0.70 then
  alert "CRITICAL: Conflicting systems detected"
  notify ml_lead via email+slack
  sla: 24 hours
  action: investigate_rule_conflicts
end

# Rule: System Failure
if system_error_rate > 0.01 then
  alert "WARNING: System error rate > 1%"
  notify platform_team via email
  sla: 24 hours
end
```

---

## 5. Dashboard Layout (UI/UX)

### 5.1 Primary View (Executive Summary)

```
╔════════════════════════════════════════════════════════════════════════════╗
║                      COMPLIANCE MONITORING DASHBOARD                       ║
║                                                                            ║
║  Last Updated: 2026-04-28T15:32:45Z  |  Refresh: 60 sec  |  Status: LIVE ║
╠════════════════════════════════════════════════════════════════════════════╣
║                                                                            ║
║  ┌──────────────────────────────────────────────────────────────────────┐ ║
║  │ SYSTEM ACCURACY (7-day rolling)                                      │ ║
║  │ ┌────────────────────────────────────────────────────────────────┐  │ ║
║  │ │ Ensemble Accuracy: 95.5% [▀▀▀▀▀▀▀▄▄▄▄] Baseline: 95.5%        │  │ ║
║  │ │ sys-001: 96.3% ✓  sys-002: 95.8% ✓  sys-003: 95.4% ✓         │  │ ║
║  │ │ sys-004: 94.1% ✓  sys-005: 96.8% ✓  sys-006: 93.2% ✓         │  │ ║
║  │ │ sys-007: 92.1% ✓  sys-008: 94.7% ✓  sys-009: 93.9% ✓         │  │ ║
║  │ │ sys-010: 95.2% ✓                                               │  │ ║
║  │ └────────────────────────────────────────────────────────────────┘  │ ║
║  └──────────────────────────────────────────────────────────────────────┘ ║
║                                                                            ║
║  ┌──────────────────────────────────────────────────────────────────────┐ ║
║  │ ENSEMBLE AGREEMENT HEATMAP                                           │ ║
║  │  [Click to expand 10x10 matrix]                                      │ ║
║  │  Avg Pairwise Agreement: 89.1%  [████████▄] ✓                        │ ║
║  │  Min Pairwise Agreement: 81.0% (sys-007 ↔ sys-006)  ✓               │ ║
║  └──────────────────────────────────────────────────────────────────────┘ ║
║                                                                            ║
║  ┌──────────────────────────────────────────────────────────────────────┐ ║
║  │ OUT-OF-DISTRIBUTION DETECTION (24h)                                 │ ║
║  │ % OOD >3σ (rejected): 1.0%  [▌] ✓ (threshold: 5%)                  │ ║
║  │ Trend: ▼ Stable (0.9h avg: 0.98%, 6h avg: 0.99%)                   │ ║
║  └──────────────────────────────────────────────────────────────────────┘ ║
║                                                                            ║
║  ┌──────────────────────────────────────────────────────────────────────┐ ║
║  │ CONFIDENCE DISTRIBUTION (Tier Breakdown)                             │ ║
║  │ Tier 1 (Auto, 0.9–1.0):    54% [████████████████████████████]      │ ║
║  │ Tier 2 (Gated, 0.7–0.9):   43% [███████████████████]                │ ║
║  │ Tier 3 (Review, <0.7):      3% [█]                                  │ ║
║  └──────────────────────────────────────────────────────────────────────┘ ║
║                                                                            ║
╠════════════════════════════════════════════════════════════════════════════╣
║  ALERTS & INCIDENTS: 0 active                                             ║
║  Last Incident: INC-042501 (2026-04-25) — Closed, post-incident review OK ║
╚════════════════════════════════════════════════════════════════════════════╝
```

### 5.2 Detailed View (System Drilldown)

```
╔════════════════════════════════════════════════════════════════════════════╗
║ SYS-005 (XGBoost) — Detailed Metrics                                       ║
╠════════════════════════════════════════════════════════════════════════════╣
║ Accuracy (7d):     96.8%  [████████▄▄] ✓                                  ║
║ Precision:         97.1%  [████████▄▄] ✓                                  ║
║ Recall:            96.5%  [████████▄▄] ✓                                  ║
║ F1:                96.8%  [████████▄▄] ✓                                  ║
║ Model Version:     1.6.0  (updated 2026-04-29)                            ║
║ Weight Hash:       b1c2d3e4f5g6h7i8j9k0l1m2n3o4p5q6                       ║
║                                                                             ║
║ Ensemble Agreement (vs other systems):                                    ║
║   sys-001: 93%  ✓   sys-002: 94%  ✓   sys-003: 92%  ✓                   ║
║   sys-004: 89%  ✓   sys-006: 89%  ✓   sys-007: 87%  ✓                   ║
║   sys-008: 92%  ✓   sys-009: 90%  ✓   sys-010: 93%  ✓                   ║
║ Average: 90.9%                                                             ║
║                                                                             ║
║ OOD Score (24h):    [Histogram of distribution]                           ║
║ % OOD >3σ:          1.1%  [▌] ✓                                           ║
║                                                                             ║
║ Recent Decisions (Last 10):                                               ║
║  [Trace ID]      [Output]   [Confidence] [Rule] [OOD] [Tier]             ║
║  5a7f8c2e...    approved      0.98      rule_001  0.12  T1               ║
║  7e2a9c4d...    escalated     0.62      null      2.9   T3               ║
║  ...                                                                       ║
╚════════════════════════════════════════════════════════════════════════════╝
```

---

## 6. Regulator Access and Audit

### 6.1 Regulator View (Read-Only)

- **Access**: HTTPS + OAuth 2.0; audit all logins
- **Data**: All metrics, full 1-year history available
- **Export**: CSV, JSON; daily digest emails
- **Verification**: Cryptographic signatures on all data points

### 6.2 Audit Trail Integration

Every dashboard metric refresh is logged:

```json
{
  "timestamp": "2026-04-28T15:32:45.123Z",
  "metric_id": "system_accuracy",
  "system_id": "sys-005",
  "value": 0.968,
  "baseline": 0.962,
  "change_pp": 0.006,
  "alert_triggered": false,
  "viewer_id": "auditor@regulator.gov",
  "data_source": "audit_trail",
  "signature": "hmac_sha256_..._64_chars"
}
```

---

## 7. SLA and Refresh Cadence

| Component | Refresh Rate | Availability | Max Latency |
|-----------|--------------|--------------|-------------|
| **Accuracy metrics** | 60 seconds | 99.9% uptime | 2 seconds |
| **OOD detection** | 60 seconds | 99.9% uptime | 2 seconds |
| **Ensemble agreement** | 60 seconds | 99.9% uptime | 2 seconds |
| **Confidence distribution** | 60 seconds | 99.9% uptime | 2 seconds |
| **Audit trail logs** | Real-time (append-only) | 99.99% uptime | <100ms |
| **Historical reports** | On-demand | 99% uptime | <1 minute |

---

## 8. Compliance Checklist

- [x] Dashboard accessible 24/7 to regulators
- [x] All metrics have clear alert thresholds
- [x] Refresh rate ≤60 seconds
- [x] Audit trail logs every dashboard access
- [x] System accuracy published per-system (not ensemble-wide)
- [x] OOD detection shown (not hidden)
- [x] Ensemble agreement transparent (heatmap visible)
- [x] Fairness metrics included
- [x] Historical data retained 1 year
- [x] Cryptographic signatures on all data points

---

**Last Updated**: 2026-04-28  
**Owned by**: Platform Engineering + Compliance  
**Next Review**: 2026-07-28 (quarterly)
