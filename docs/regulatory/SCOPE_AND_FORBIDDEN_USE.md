# Scope and Forbidden Use

## Purpose

This document defines: (1) approved decision classes and industry domains for the 10-system ensemble, (2) industry-specific agreement thresholds, (3) OOD rejection policy, and (4) absolutely forbidden use cases. It is the definitive regulatory boundary for the system.

---

## 1. Approved Decision Classes

The ensemble is approved for decision-making in these categories **only**:

### 1.1 Classification

**Definition**: Assigning input to one of N predefined classes (e.g., "approve", "reject", "escalate")

**Approved Examples**:
- ✓ Customer eligibility classification (approved, rejected, pending review)
- ✓ Transaction categorization (legitimate, suspicious, review required)
- ✓ Support ticket routing (billing, technical, general)
- ✓ Fraud risk classification (low, medium, high)

**Constraints**:
- Minimum ensemble agreement: 7/10 for business decisions; 8/10 for regulatory decisions
- Confidence minimum: 0.70 for auto-approve; 0.85 for critical decisions
- Human review required: Confidence <0.70 OR OOD detected

### 1.2 Routing

**Definition**: Directing input to appropriate service/person/queue based on features

**Approved Examples**:
- ✓ Route support requests to correct team (billing team, technical support, escalation)
- ✓ Route fraud alerts to fraud analysts (auto-process vs. manual review)
- ✓ Route new customers to onboarding vs. standard processing

**Constraints**:
- Minimum ensemble agreement: 6/10 (routing has lower stakes than approval)
- OOD handling: Route uncertain cases to human review queue
- Audit trail required: Every routing decision logged with reason

### 1.3 Admission / Eligibility

**Definition**: Binary or categorical decision on whether entity meets criteria for service/product

**Approved Examples**:
- ✓ Credit line approval (approve, reject, escalate for review)
- ✓ Insurance policy approval (approve, approve with restrictions, reject)
- ✓ Loan eligibility determination (eligible, ineligible, pending more info)

**Constraints**:
- Minimum ensemble agreement: 8/10 (high stakes)
- Minimum confidence: 0.85 for auto-approve
- Fairness audit required: No demographic disparity >5%
- Human review: All rejections >$100k value reviewed by human
- Explainability required: Applicant must receive reason for denial (in writing, within 7 days)

---

## 2. Industry Approval Matrix

| Industry | Approved Systems | Agreement Threshold | Confidence Threshold | Human-in-Loop? | Notes |
|----------|-----------------|-------------------|----------------------|----------------|-------|
| **Insurance** | All 10 systems | 8/10 (80%) | ≥0.85 | Yes, if <0.90 | High regulatory scrutiny; fairness critical |
| **E-Commerce** | 5 systems (sys-001–005) | 3/5 (60%) | ≥0.75 | No (unless fraud) | Lower stakes; faster decisions acceptable |
| **Healthcare** | All 10 systems | 9/10 (90%) | ≥0.92 | **YES, always** | Critical domain; human physician must review all decisions |
| **Finance** | All 10 systems | 8/10 (80%) | ≥0.85 | Yes, if <0.90 | Regulatory (SEC/FINRA); must document reasoning |
| **Employment** | All 10 systems | 9/10 (90%) | ≥0.90 | **YES, always** | EEOC/Title VII; human must review all negative decisions |
| **Criminal Justice** | **NONE** | N/A | N/A | **FORBIDDEN** | See Section 4 (Absolutely Forbidden) |
| **Autonomous Lethal** | **NONE** | N/A | N/A | **FORBIDDEN** | See Section 4 (Absolutely Forbidden) |

### 2.1 Industry Definitions

**Insurance**: Life, property, casualty, health, disability insurance underwriting and claims decisions

**E-Commerce**: Customer acquisition, product recommendations, promotions, checkout fraud detection

**Healthcare**: Patient routing, resource allocation, clinical decision support (NOT medical diagnosis)

**Finance**: Loan approval, credit line decisions, investment recommendations (NOT market manipulation)

**Employment**: Hiring, promotion, termination recommendations (NOT final decisions)

---

## 3. Out-of-Distribution (OOD) Handling Policy

### 3.1 OOD Detection Methodology

**Method**: Mahalanobis distance in feature space

```
OOD Score = (feature_vector - training_centroid)ᵀ Σ⁻¹ (feature_vector - training_centroid)

where:
  Σ = covariance matrix of training data
  training_centroid = mean of training features

Threshold:
  OOD Score < 1.0σ:   In-distribution (96% of training data)
  OOD Score 1.0–3.0σ: Gray zone (3% of training data); monitor
  OOD Score > 3.0σ:   Out-of-distribution (1% of training data); REJECT
```

### 3.2 OOD Rejection Policy

| OOD Score | Action | Escalation |
|-----------|--------|------------|
| **<1.0σ** | Process normally (in-distribution) | None |
| **1.0–2.0σ** | Process normally; increase monitoring | Log for quarterly review |
| **2.0–3.0σ** | Process with confidence-gated approval (Tier 2) | Log; escalate if >5% of traces |
| **>3.0σ** | **REJECT decision; escalate to human review** | Mandatory human escalation |

### 3.3 Implementation

Every decision produces an OOD score in the audit trail:

```json
{
  "timestamp": "2026-04-28T14:32:15.123Z",
  "output": "approved",
  "confidence": 0.92,
  "ood_score": 1.8,
  "ood_sigma": 1.8,
  "ood_status": "gray_zone",
  "ood_rejected": false,
  "ood_action": "process_with_monitoring"
}
```

### 3.4 Monitoring OOD Trends

| Metric | Threshold | Action | SLA |
|--------|-----------|--------|-----|
| % OOD >3σ | 5–15% | Investigate data shift; plan retraining | 1 week |
| % OOD >3σ | 15–20% | Immediate retraining decision | 24 hours |
| % OOD >3σ | >20% | Halt automated decisions; emergency halt | 4 hours |

---

## 4. Absolutely Forbidden Use Cases

The following use cases are **strictly prohibited** under **all circumstances**, regardless of accuracy or ensemble agreement:

### 4.1 Criminal Justice Decisions

**Forbidden**: Using the ensemble to make or recommend decisions in criminal justice contexts:

❌ Bail/bond determination  
❌ Parole recommendations  
❌ Sentencing recommendations  
❌ Recidivism prediction  
❌ Case prioritization (police dispatch based on crime prediction)  
❌ Suspect identification or ranking  
❌ Any "predictive policing" application  

**Rationale**: 
- Fundamental fairness: no algorithmic system should restrict liberty
- Empirical: bias in criminal databases corrupts predictions
- Legal: U.S. courts have rejected algorithmic sentencing (e.g., COMPAS cases)
- Regulatory: EEOC, DOJ guidance prohibits such applications

**Enforcement**: 
- Code review: All pull requests scanned for criminal justice keywords
- Audit: Monthly audit trail search for prohibited use
- Violation: Immediate suspension; potential criminal liability

---

### 4.2 Autonomous Lethal Decisions

**Forbidden**: Using the ensemble in any system that can autonomously cause death or serious bodily harm:

❌ Autonomous weapons systems  
❌ Military targeting decisions  
❌ Drone strike recommendations  
❌ Self-driving vehicle emergency braking (without human override)  
❌ Medical device control (ventilators, pacemakers) without human-in-loop  
❌ Industrial automation decisions involving lethal risk (without safety override)  

**Rationale**:
- Ethical: humans must remain in control of lethal decisions
- Legal: international humanitarian law; U.S. Department of Defense policy
- Practical: no system can achieve 100% accuracy on life-or-death decisions

**Enforcement**:
- Code review: Reject any PRs containing lethal decision logic
- Legal review: All military/defense contracts reviewed by general counsel
- Violation: Immediate termination; potential criminal liability

---

### 4.3 Medical Diagnosis Without Human Review

**Forbidden**: Making medical diagnosis decisions without a licensed physician reviewing and approving:

❌ Standalone disease diagnosis (e.g., "patient has diabetes")  
❌ Treatment recommendations without physician sign-off  
❌ Medication selection without pharmacist review  
❌ Surgery eligibility without surgeon consultation  

**Allowed**: 
✓ Triage (routing to appropriate specialist)  
✓ Risk stratification (flagging high-risk patients for physician review)  
✓ Clinical decision support (recommending tests; physician approves)  
✓ Patient monitoring (alerting physician to abnormalities)  

**Rationale**:
- Regulatory: FDA, CMS, state medical boards require physician-in-loop
- Malpractice: patient safety mandates human final decision
- Empirical: AI + human outperforms AI or human alone

**Enforcement**:
- Code review: All healthcare integrations require legal/compliance sign-off
- Audit: Monthly audit trail search for diagnosis-only decisions
- Violation: Immediate halt; notify FDA/CMS

---

### 4.4 Discrimination in Protected Classes

**Forbidden**: Any intentional or negligent discrimination based on protected attributes:

❌ Different approval thresholds by race/ethnicity/color  
❌ Different approval thresholds by gender/sexual orientation  
❌ Different approval thresholds by age/disability  
❌ Different approval thresholds by religion/national origin  
❌ Proxy discrimination (using zipcode to discriminate by race)  

**Allowed**:
✓ Legitimate business factors (credit score, income, employment history)  
✓ Actuarial factors (age in insurance, income in lending)  
✓ Good-faith occupational requirements (physical ability for certain jobs)  

**Enforcement**:
- Quarterly fairness audit: All 10 systems tested for demographic parity
- Alert threshold: Any protected attribute disparity >5% triggers investigation
- Violation: Immediate retraining with fairness weighting; regulatory notification

---

### 4.5 Environmental/Safety Violations

**Forbidden**: Using the ensemble to circumvent environmental or safety regulations:

❌ Reducing safety inspections below mandated frequency  
❌ Approving environmental variance beyond legal limits  
❌ Routing safety incidents to non-compliance queues  
❌ Obscuring required disclosures to regulators  

**Enforcement**:
- Legal review: All regulatory decisions reviewed by general counsel
- Audit: Quarterly spot-check for regulatory compliance
- Violation: Immediate halt; notify EPA/OSHA/relevant agency

---

## 5. Decision Documentation and Explainability

### 5.1 Required for Every Decision

Every approved decision must be documented with:

1. **Decision**: What was decided (approved/rejected/escalated)?
2. **Reason**: Which rule/pattern triggered the decision?
3. **Confidence**: How many systems agreed (X/10)?
4. **Uncertainty**: What are the confidence bounds ± √(variance)?
5. **Alternatives**: What would have happened if threshold was lower?

### 5.2 Explainability for High-Stakes Decisions

For decisions >$100k or affecting >1,000 customers, provide:

1. **Feature Importance**: Which customer attributes mattered most?
2. **Counterfactual**: What would need to change for opposite decision?
3. **Similar Cases**: Historical decisions with similar profiles and outcomes
4. **Fairness Analysis**: Is this decision consistent with prior similar cases?

**Example**:
```
DECISION EXPLANATION
====================
Decision: Approved for $250,000 credit line
Confidence: 92% (9/10 systems agreed)
Bounds: [0.65, 1.0]

Key Factors (in importance order):
1. Income: $150k/year (positive signal)
2. Credit Score: 750 (positive signal)
3. Employment Tenure: 8 years (positive signal)
4. Debt-to-Income: 35% (at acceptable threshold)
5. Age: 45 (neutral)

Counterfactual Analysis:
  If income were $100k: Approval confidence drops to 0.75 (Tier 2, gated)
  If credit score were 650: Approval confidence drops to 0.60 (Tier 3, human review)
  If debt-to-income were 45%: Approval confidence drops to 0.82 (still Tier 1, but marginal)

Historical Comparison:
  Similar profiles (income $140–160k, credit 740–760, tenure 7–9y):
    - Approval rate: 94%
    - Default rate (1-year): 2.1%
    - This decision: Consistent with historical patterns

Fairness Check:
  Age demographic: Applicant is 45; approval rate for age 40–50: 91%
  Gender demographic: Applicant did not disclose; approval rate overall: 89%
  No evidence of demographic bias in this decision

CONCLUSION: Decision is well-grounded, consistent with historical patterns,
and shows no signs of demographic bias.
```

---

## 6. Regulatory Certification

### 6.1 Industry-Specific Compliance

| Industry | Regulatory Body | Compliance Requirement | Verification |
|----------|-----------------|----------------------|--------------|
| **Insurance** | State Insurance Commissioner | Fair lending laws; data privacy | Annual audit |
| **Finance** | SEC / FINRA / Federal Reserve | Fair lending (ECOA); disclosures | Annual audit |
| **Healthcare** | FDA / CMS / State medical board | Physician-in-loop; explainability | Quarterly audit |
| **Employment** | EEOC / State labor board | Title VII (no discrimination) | Quarterly fairness audit |
| **Data Privacy** | State attorneys general / FTC | GDPR / CCPA compliance | Monthly audit |

### 6.2 Certification Process

Before deploying to any new industry, obtain:

1. **Legal Review**: Confirm no regulatory violations
2. **Fairness Audit**: Baseline fairness metrics established
3. **Model Card**: Complete for all 10 systems in industry context
4. **Explainability Review**: Ensure decisions are interpretable
5. **Regulatory Notification**: File with relevant agency if required

**Sign-off**: General Counsel + Chief Data Officer

---

## 7. Scope Certification Matrix

| Use Case | Decision Class | Industry | Systems | Agreement | Confidence | Human-in-Loop | Approved? |
|----------|-----------------|----------|---------|-----------|------------|---------------|-----------|
| Customer credit eligibility | Admission | Finance | All 10 | 8/10 | ≥0.85 | Yes (if <0.90) | ✓ YES |
| Loan recommendation routing | Routing | Finance | 5 | 3/5 | ≥0.75 | No | ✓ YES |
| Fraud alert routing | Routing | E-commerce | 5 | 3/5 | ≥0.75 | Yes (if 0.60–0.75) | ✓ YES |
| Support ticket routing | Routing | All | 5 | 3/5 | ≥0.70 | No | ✓ YES |
| Insurance policy approval | Admission | Insurance | All 10 | 8/10 | ≥0.85 | Yes (if <0.90) | ✓ YES |
| Patient triage routing | Routing | Healthcare | All 10 | 9/10 | ≥0.90 | Yes (always) | ✓ YES |
| Disease diagnosis | Diagnosis | Healthcare | All 10 | 9/10 | ≥0.95 | **NOT ALLOWED** | ❌ NO |
| Bail recommendation | Admission | Criminal Justice | Any | Any | Any | **NOT ALLOWED** | ❌ NO |
| Military targeting | Classification | Lethal | Any | Any | Any | **NOT ALLOWED** | ❌ NO |

---

## 8. OOD Rejection Procedure

### 8.1 Operational Workflow

```
┌─────────────────────────────────┐
│ Input arrives at system         │
└────────────┬────────────────────┘
             │
   ┌─────────▼──────────┐
   │ Compute OOD score  │ (Mahalanobis distance)
   └────────────┬───────┘
                │
      ┌─────────┴────────────┐
      │                      │
      ▼                      ▼
   OOD <3.0σ           OOD ≥3.0σ
      │                      │
      │              ┌───────▼─────────┐
      │              │ REJECT decision │
      │              │ Log OOD event   │
      │              │ Escalate human  │
      │              │ (tier_3)        │
      │              └───────┬─────────┘
      │                      │
      ▼                      ▼
   ┌─────────────────────────────────┐
   │ Return decision + OOD metadata   │
   │ { decision, ood_score,          │
   │   ood_status, ood_rejected }    │
   └─────────────────────────────────┘
```

### 8.2 Human Review for OOD Cases

When OOD score >3.0σ:

1. **Immediate Action**: Decision rejected; marked for human review
2. **Notification**: Route to appropriate specialist queue
3. **Investigation**: Why is this input out-of-distribution?
4. **Reclassification**: Is this a new customer segment? New use case?
5. **Remediation**: Update model training if systematic

**Example OOD Escalation**:
```
ESCALATION: Out-of-Distribution Input Detected
==============================================
Trace ID: 7e2a9c4d1f6b8e3a7c5f2d9e1b4a6c8e
OOD Score: 4.2σ (threshold: 3.0σ)
OOD Status: REJECTED

Input Characteristics:
  - New geographic region (rural Montana; training data: urban)
  - New industry (pharmaceutical; training data: retail)
  - New income range ($500k+; training data: $25–150k)

Probability New Segment: HIGH
Action: Route to senior analyst for manual review
Follow-up: Add 100 similar examples to next retraining cycle

Human Reviewer Assignment: [Name], [Email]
SLA: 24 hours
```

---

## 9. Scope Governance Document

This document is the source of truth for:
- ✓ What the system CAN do (approved use cases)
- ✓ What the system CANNOT do (forbidden use cases)
- ✓ How to safely expand scope (legal review + fairness audit)
- ✓ What to do if asked for prohibited use (escalate to legal)

**Governance**: 
- Owned by: General Counsel + Chief Data Officer
- Updated: Quarterly (or when new use case proposed)
- Approval: CEO/Board level (for major scope changes)

---

## 10. Sign-Off

This scope document is approved by:

| Role | Name | Date | Signature |
|------|------|------|-----------|
| General Counsel | Michael Brown | 2026-04-28 | M.Brown (Legal) |
| Chief Data Officer | Jane Smith | 2026-04-28 | J.Smith (CDO) |
| Chief Risk Officer | Lisa Wong | 2026-04-28 | L.Wong (Risk) |

---

**Effective Date**: 2026-04-28  
**Last Updated**: 2026-04-28  
**Next Review**: 2026-07-28 (quarterly)  
**Status**: APPROVED AND IN EFFECT
