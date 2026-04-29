# Model Card Template

## Purpose

Every system (5 symbolic + 5 learned) must have a completed model card. This template ensures regulatory-grade documentation of system behavior, training data, test metrics, and failure modes.

---

## Template: System Model Card

### 1. System Identity

| Field | Value |
|-------|-------|
| **System Name** | [Name, e.g., "ELIZA-keyword classifier"] |
| **System ID** | [Unique identifier, e.g., "sys-001"] |
| **System Type** | Symbolic / Learned (ML) |
| **Version** | [Semantic version, e.g., 1.2.3] |
| **Date Trained / Last Updated** | [ISO 8601 date] |
| **Maintainer** | [Team name, email] |
| **Responsible Party** | [Person accountable for performance] |

### 2. System Description

[Brief functional description: What does this system do? What decisions does it make? How does it integrate with the ensemble?]

### 3. Training Data

| Field | Value |
|-------|-------|
| **Data Source** | [Origin, e.g., "RevOps customer lifecycle logs, 2024 Q3-Q4"] |
| **Dataset Size** | [Examples: "50,000 traces", "1.2M events"] |
| **Data Characteristics** | [Domain, time period, sampling method, class balance] |
| **License** | [Data license, e.g., "Internal proprietary", "CC-BY-4.0"] |
| **Preprocessing** | [Transformations: normalization, feature engineering, outlier removal] |
| **Train/Validation/Test Split** | [Ratios, e.g., "70% / 10% / 20%"] |
| **Data Retention** | [Storage location, retention period, compliance (GDPR/CCPA)] |

### 4. Test Metrics

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| **Accuracy** | [%] | ≥95% | PASS/WARN/FAIL |
| **Precision** | [%] | ≥92% | PASS/WARN/FAIL |
| **Recall** | [%] | ≥92% | PASS/WARN/FAIL |
| **F1 Score** | [%] | ≥92% | PASS/WARN/FAIL |
| **ROC-AUC** | [0.00–1.00] | ≥0.95 | PASS/WARN/FAIL |
| **False Positive Rate** | [%] | ≤5% | PASS/WARN/FAIL |
| **False Negative Rate** | [%] | ≤5% | PASS/WARN/FAIL |

### 5. Confidence Calibration

| Aspect | Finding |
|--------|---------|
| **Calibration Method** | Ensemble variance: σ(system outputs) |
| **Expected Confidence** | (agreement_count / 10) for deterministic systems |
| **Calibration Error** | [Expected/Max Expected Calibration Error] |
| **Confidence vs. Accuracy** | High-confidence predictions should have error rate <2%; low-confidence (<0.7) should be flagged for review |

### 6. Failure Modes

| Failure Mode | Trigger Condition | Severity | Mitigation |
|--------------|-------------------|----------|-----------|
| [Name, e.g., "Class imbalance misclassification"] | [When does it occur?] | Low / Medium / High | [How do we prevent / respond?] |
| | | | |
| | | | |

**Example Failure Modes for Learned Models**:
- *Concept drift*: Accuracy drop >5% on rolling 30-day validation. Trigger retraining.
- *Adversarial input*: OOD score > 3σ. Trigger alert, reject prediction, escalate.
- *Class imbalance*: Minority class F1 < 0.85. Add sampling weights, retrain.
- *Data poisoning*: Audit trail log shows system output jumped without corresponding input change. Isolate, investigate, revert.

**Example Failure Modes for Symbolic Systems**:
- *Rule coverage gap*: Input matches no rules. Flag as OOD, reject.
- *Rule conflict*: Multiple rules fire, outputs diverge. Escalate to human review.
- *Threshold drift*: Regulatory boundary shift (e.g., credit limit rules change). Retrain/recalibrate within 48 hours.

### 7. Out-of-Distribution (OOD) Behavior

| Aspect | Method |
|--------|--------|
| **OOD Detection** | Input divergence score = max(L2 distance to training clusters) / σ_train |
| **Rejection Threshold** | Divergence > 3σ → Reject, escalate to human, flag in audit trail |
| **Fallback** | Default action (e.g., "admit with human review") |
| **Logging** | Every OOD rejection logged with input hash, divergence score, timestamp |

### 8. Approved Use Cases

| Use Case | Approval Status | Notes |
|----------|-----------------|-------|
| [e.g., "Customer eligibility classification"] | Approved | [Conditions: "Insurance, ≥8/10 agreement required"] |
| | | |
| | | |

### 9. Forbidden Use Cases

- [Criminal justice decision-making]
- [Autonomous lethal actions]
- [Medical diagnosis without human review]
- [Discriminatory decisions without fairness audit]

### 10. Dependencies and Integration

| Dependency | Type | Version | Status |
|------------|------|---------|--------|
| [Other system, library, data source] | [System / Library / Data] | [Version] | [Active / Deprecated] |
| | | | |

### 11. Audit Trail Integration

Every decision made by this system **MUST** produce an entry in the immutable audit trail:

```json
{
  "timestamp": "2026-04-28T14:32:15.123Z",
  "system_id": "sys-001",
  "input_hash": "a1b2c3d4e5f6...",
  "output": "approved",
  "confidence": 0.94,
  "rule_fired": "eligibility_rule_001",
  "weight_hash": "w1w2w3w4...",
  "tier_executed": "tier_1"
}
```

### 12. Performance Monitoring

- **Refresh Cadence**: 60 seconds (real-time dashboard)
- **Alarm Threshold**: Accuracy < 85% (warning), < 80% (critical)
- **Drift Detection**: Quarterly validation on held-out test set
- **Incident Escalation**: If accuracy drop > 10% in rolling 30 days, halt retraining until root cause found

### 13. Versioning and Change Log

| Version | Date | Change | Approval | Notes |
|---------|------|--------|----------|-------|
| 1.0 | 2026-02-15 | Initial release | [Approver] | Production deployment |
| 1.1 | 2026-03-20 | Precision calibration | [Approver] | Canary test 7 days, no incidents |
| | | | | |

### 14. Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Model Developer | | | |
| Data Governance Committee | | | |
| Chief Data Officer | | | |
| Compliance Officer | | | |

---

## Example: ELIZA-Keyword Classifier (Completed Model Card)

### 1. System Identity

| Field | Value |
|-------|-------|
| **System Name** | ELIZA-keyword classifier |
| **System ID** | sys-001 |
| **System Type** | Symbolic |
| **Version** | 2.1.0 |
| **Date Trained / Last Updated** | 2026-02-15 |
| **Maintainer** | ML Engineering Team, ml-team@company.com |
| **Responsible Party** | Alice Chen, Senior ML Engineer |

### 2. System Description

ELIZA-keyword classifier is a rule-based symbolic system that routes customer support requests to the appropriate team (billing, technical, general inquiry) by matching text patterns against a curated set of linguistic markers. The system is fully deterministic and produces no randomness.

### 3. Training Data

| Field | Value |
|-------|-------|
| **Data Source** | RevOps customer support logs, 2024 Q3-Q4; 8,000 hand-annotated support tickets |
| **Dataset Size** | 8,000 support requests; 45,000 total words |
| **Data Characteristics** | Customer support domain; real production queries; 70% English, 20% Spanish, 10% mixed; class distribution: 45% billing, 35% technical, 20% general |
| **License** | Internal proprietary; GDPR-compliant anonymization applied (removed customer names, account IDs) |
| **Preprocessing** | Lowercasing, punctuation removal, stop-word filtering (standard NLTK list) |
| **Train/Validation/Test Split** | 70% train (5,600 tickets), 10% validation (800), 20% test (1,600) |
| **Data Retention** | Stored in S3 (s3://company-ml/datasets/support-tickets-v2.1/); 2-year retention; GDPR right-to-deletion enabled |

### 4. Test Metrics

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| **Accuracy** | 96.3% | ≥95% | PASS |
| **Precision (Billing)** | 97.1% | ≥92% | PASS |
| **Precision (Technical)** | 95.8% | ≥92% | PASS |
| **Precision (General)** | 94.2% | ≥92% | PASS |
| **Recall (Billing)** | 96.0% | ≥92% | PASS |
| **Recall (Technical)** | 95.2% | ≥92% | PASS |
| **Recall (General)** | 97.1% | ≥92% | PASS |
| **F1 Score (macro)** | 95.8% | ≥92% | PASS |
| **ROC-AUC (weighted)** | 0.987 | ≥0.95 | PASS |
| **False Positive Rate** | 3.2% | ≤5% | PASS |
| **False Negative Rate** | 2.8% | ≤5% | PASS |

### 5. Confidence Calibration

| Aspect | Finding |
|--------|---------|
| **Calibration Method** | For symbolic rules: confidence = 1.0 if rule fires; 0.0 if no rule fires. For ensemble: agreement across 10 systems provides variance estimate. |
| **Expected Confidence** | Average system agreement in test set: 9.4 / 10 (94% agreement) |
| **Calibration Error** | MCE (max calibration error): 2.1% |
| **Confidence vs. Accuracy** | Predictions with confidence ≥0.9 have error rate 1.8%; predictions with confidence <0.7 have error rate 6.2% (escalated for manual review) |

### 6. Failure Modes

| Failure Mode | Trigger Condition | Severity | Mitigation |
|--------------|-------------------|----------|-----------|
| Slang/dialect mismatch | New customer cohort uses unseen slang (e.g., "my app is sus") | Low | OOD detection flags; routed to general category with manual review flag |
| Language-specific keywords | Spanish tickets misrouted due to keyword list imbalance | Medium | Quarterly audit of Spanish subset (n=1,600 historical); add keywords every Q2 |
| Rule conflict | Two rules fire simultaneously (e.g., "billing" + "refund fraud") | Medium | Log both matches; ensemble agreement vote breaks tie |
| Typos in routing keywords | "biiling" instead of "billing" not caught | Low | Phonetic fuzzy matching (Levenshtein distance ≤1); triggers OOD if distance > 2 |

### 7. Out-of-Distribution (OOD) Behavior

| Aspect | Method |
|--------|--------|
| **OOD Detection** | If no rule matches AND input divergence > 3σ from training vocabulary, flag as OOD |
| **Rejection Threshold** | OOD score > 0.75 → Reject routing decision, escalate to human (no automatic action) |
| **Fallback** | Route to "general" queue with HIGH_PRIORITY_REVIEW flag |
| **Logging** | Every OOD rejection logged: {timestamp, input_hash, divergence_score, rejected_routing, routed_to: "human_review"} |

### 8. Approved Use Cases

| Use Case | Approval Status | Notes |
|----------|-----------------|-------|
| Customer support ticket routing | Approved | Insurance domain; accuracy 96.3%; requires ≥8/10 system agreement before automated action |
| SaaS billing inquiry triage | Approved | E-commerce; 3/5 systems minimum agreement |
| General inquiry classification | Approved | No additional human review required if confidence ≥0.95 |

### 9. Forbidden Use Cases

- Criminal background/fraud detection without follow-up human review
- Automated account closure based solely on keyword classification
- Discriminatory routing (e.g., preferential treatment by language/region)

### 10. Dependencies and Integration

| Dependency | Type | Version | Status |
|------------|------|---------|--------|
| NLTK (stop-word list) | Library | 3.8.1 | Active |
| RevOps event log | Data | 2024 Q3-Q4 | Active |
| Ensemble: Bayesian classifier (sys-002) | System | 1.0.0 | Active |
| Ensemble: Logistic regression (sys-003) | System | 1.5.2 | Active |

### 11. Audit Trail Integration

Every routing decision produces:

```json
{
  "timestamp": "2026-04-28T14:32:15.123Z",
  "system_id": "sys-001",
  "input_hash": "5a7f8c2e9b1d...",
  "output": "technical",
  "confidence": 0.98,
  "rule_fired": "rule_technical_001_error_keywords",
  "weight_hash": "w1a2b3c4d5e6f...",
  "tier_executed": "tier_1",
  "ood_score": 0.12,
  "agreement_count": 9,
  "ensemble_confidence": 0.90
}
```

### 12. Performance Monitoring

- **Refresh Cadence**: Real-time on every decision; aggregate metrics every 60 seconds
- **Alarm Threshold**: Accuracy < 85% (warning), < 80% (critical); OOD detection rate > 10% (warning)
- **Drift Detection**: Monthly validation on 500-ticket rolling sample; quarterly deep validation (full test set)
- **Incident Escalation**: If accuracy drops >10% in rolling 30 days, pause automated routing and investigate

### 13. Versioning and Change Log

| Version | Date | Change | Approval | Notes |
|---------|------|--------|----------|-------|
| 1.0 | 2025-11-10 | Initial ruleset (50 rules) | Jane Smith (CDO) | Production v1 |
| 1.1 | 2025-12-05 | Added Spanish language rules (+15 rules) | Jane Smith (CDO) | Canary 7 days, no incidents, accuracy 95.8% |
| 2.0 | 2026-01-15 | Refactored rule conflict resolution, added fuzzy matching | Jane Smith + Mark Wu (Compliance) | Full test suite passed; F1 95.1% → 95.8% |
| 2.1 | 2026-02-15 | Phonetic matching (Levenshtein ≤1) for typo robustness | Jane Smith (CDO) | Canary showed 0.3% accuracy uplift; all thresholds met |

### 14. Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Model Developer | Alice Chen | 2026-02-15 | A.Chen (signed) |
| Data Governance Committee | Jane Smith (Chair) | 2026-02-15 | J.Smith (approved) |
| Chief Data Officer | Jane Smith | 2026-02-15 | J.Smith (CDO) |
| Compliance Officer | Mark Wu | 2026-02-15 | M.Wu (signed) |

---

## Remaining Systems (9 more to be completed)

- **sys-002**: Bayesian classifier
- **sys-003**: Logistic regression model
- **sys-004**: Random Forest ensemble
- **sys-005**: XGBoost model
- **sys-006**: Rule-based threshold system
- **sys-007**: Time-series anomaly detector
- **sys-008**: NLP transformer model
- **sys-009**: Symbolic symbolic expert system
- **sys-010**: Hybrid neuro-symbolic model

Each system MUST have a completed model card before deployment to production. Target completion: 2026-05-31.
