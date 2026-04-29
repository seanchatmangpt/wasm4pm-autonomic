# Adversarial Response: Structural Mitigations by System

**Date:** April 2026  
**Responders:** dteam Engineering Team  
**Status:** We accept the adversarial reviewers' critiques and propose targeted structural mitigations that restore feedback loops and drift detection while preserving determinism and auditability.

---

## Executive Summary

The adversarial panel identified a genuine vulnerability: compile-time reasoning cannot adapt to runtime divergence between model and reality. Rather than dispute this, we propose five focused mitigations—one per system—that add feedback instrumentation, confidence intervals, and retraining schedules without sacrificing determinism or introducing runtime variance.

The key insight: **determinism and learning are orthogonal**. A deterministic system at T=0 can emit instrumentation that feeds learning loops at T=1, without corrupting the immutable audit trail at T=0.

---

## I. ELIZA: Intent Confidence + Deprecation Horizon

### The Critique

Weizenbaum identified: keyword matching is deterministic but produces no measure of correctness. Compile-time freezing means the system cannot adapt when language evolves, demographics shift, or new emotional contexts emerge. Determinism proves auditability, not validity.

### Structural Mitigation

Replace binary intent classification with **confidence-instrumented keywords**:

1. **At compile time**, embed not just intent class but a _confidence band_:
   - Intent match occurs when keyword similarity > threshold
   - Confidence = (1.0 - edit_distance / max_distance)
   - If confidence < 0.6, emit `UNCERTAIN_INTENT`
   
2. **Ship with temporal boundaries**:
   - Embed a "validity horizon" in the binary (e.g., "expires 2027-04-28")
   - System warns: "This ELIZA binary is deprecated as of [DATE]. Retrain required."
   - Clinches: user cannot ignore expiration without explicit override

3. **Emit structured feedback logs**:
   - Every exchange produces a timestamped JSON record: `{intent, confidence, text, timestamp, user_feedback}`
   - Aggregated logs drive retraining at T=1
   - Zero runtime variance, full auditability

### Implementation Sketch

```rust
// File: src/classical_ai/eliza.rs

#[derive(Clone, Copy)]
pub struct IntentClassification {
    pub intent: ElizaIntent,
    pub confidence: f32,  // [0.0, 1.0]
    pub keyword_match: &'static str,
    pub validity_horizon: UnixTimestamp,  // Compile-time constant
}

pub struct ELIZADeterministic {
    keywords: &'static [(KeywordPattern, ElizaIntent, f32)],  // pattern, intent, base_confidence
    validity_horizon: UnixTimestamp,
}

impl ELIZADeterministic {
    pub fn classify(&self, text: &str) -> (IntentClassification, Option<&'static str>) {
        if SystemTime::now().unix_timestamp() > self.validity_horizon {
            return (
                IntentClassification { 
                    intent: UNCERTAIN,
                    confidence: 0.0,
                    keyword_match: "EXPIRED_MODEL",
                    validity_horizon: self.validity_horizon,
                },
                Some("ELIZA binary deprecated. Retrain required.")
            );
        }
        
        // Find best keyword match using Levenshtein distance
        let (best_keyword, best_intent, base_conf) = self.keywords
            .iter()
            .max_by_key(|(pattern, _, _)| text_similarity_score(text, pattern))
            .unwrap_or((&UNKNOWN_PATTERN, UNCERTAIN, 0.0));
        
        let confidence = if best_keyword.matches_exactly(text) {
            base_conf
        } else {
            base_conf * text_similarity_score(text, best_keyword)
        };
        
        (IntentClassification {
            intent: *best_intent,
            confidence,
            keyword_match: best_keyword.as_str(),
            validity_horizon: self.validity_horizon,
        }, None)
    }
}

// Instrumentation: every classification attempt is logged
pub fn classify_with_feedback(
    eliza: &ELIZADeterministic,
    text: &str,
) -> (IntentClassification, FeedbackRecord) {
    let (classification, warning) = eliza.classify(text);
    
    let record = FeedbackRecord {
        timestamp: SystemTime::now().unix_timestamp(),
        input_text: text.to_string(),
        predicted_intent: format!("{:?}", classification.intent),
        confidence: classification.confidence,
        validity_horizon: classification.validity_horizon,
        deprecation_warning: warning.map(|s| s.to_string()),
    };
    
    (classification, record)
}
```

### Code Locations

- **Intent matching logic**: `src/classical_ai/eliza.rs`
- **Confidence instrumentation**: Add `confidence: f32` field to existing `IntentClassification` struct
- **Validity horizon**: `src/config.rs` — add `[eliza] expiration_date = "2027-04-28"` to `dteam.toml`
- **Feedback emission**: `src/agentic/feedback_emitter.rs` (new module, append-only JSON log)

### Trade-Off Analysis

- **Determinism cost**: None. Confidence calculation is pure arithmetic (edit distance + multiplication).
- **Latency cost**: Minimal. Levenshtein is O(|keyword| × |text|); precomputed for fixed keywords.
- **Auditability gain**: Inspectors can now see confidence bands and expiration dates in compiled constants.
- **Feedback loop restored**: Logs enable retraining; expiration prevents indefinite staleness.

---

## II. MYCIN: Confidence Intervals + Quarterly Retraining Gate

### The Critique

Feigenbaum identified: compile-time training on static data means organisms unseen at training time are classified with false confidence. Novel pathogens, resistance evolution, and covariate shift degrade accuracy silently. No mechanism detects this drift.

### Structural Mitigation

1. **Replace binary output with (class, confidence_interval)**:
   - MYCIN predicts not just "infected: E. coli" but "(E. coli, [0.68, 0.78])"
   - Interval reflects training set uncertainty, not runtime uncertainty
   - Clinician sees: "Likely organism: E. coli. Confidence: 68–78%. Verify with culture."

2. **Periodic retraining gate**:
   - Embed minimum retraining date in binary
   - After that date, system enters "advisory-only" mode
   - Requires human confirmation before acting

3. **Silent degradation detection**:
   - Ship with a small test set (10 representative cases)
   - Append a "diagnostic accuracy check" that runs at startup
   - If test accuracy drops below 85%, log ALERT

### Implementation Sketch

```rust
// File: src/classical_ai/mycin.rs

#[derive(Clone, Copy)]
pub struct ConfidenceInterval {
    pub lower: f32,
    pub upper: f32,
}

pub struct MYCINClassification {
    pub predicted_organism: MYCINOrganism,
    pub confidence_interval: ConfidenceInterval,
    pub last_retrain_date: UnixTimestamp,
    pub next_retrain_required: UnixTimestamp,
}

pub struct MYCINDeterministic {
    // Compiled Naive Bayes tables
    feature_weights: &'static [[f32; NUM_FEATURES]],  // Per organism
    organism_priors: &'static [f32],
    
    // Compiled test set for diagnostic checks
    diagnostic_cases: &'static [(DiagnosticInput, DiagnosticGoldStandard)],
    min_acceptable_accuracy: f32,  // 0.85
    
    last_retrain: UnixTimestamp,
    next_retrain_required: UnixTimestamp,
}

impl MYCINDeterministic {
    pub fn classify(&self, features: &[f32; NUM_FEATURES]) -> MYCINClassification {
        let now = SystemTime::now().unix_timestamp();
        
        // Check if past retraining deadline
        let in_advisory_mode = now > self.next_retrain_required;
        
        // Compute Naive Bayes scores
        let mut scores = [0.0; NUM_ORGANISMS];
        for (org_idx, weights) in self.feature_weights.iter().enumerate() {
            let mut score = self.organism_priors[org_idx].ln();
            for (feat_idx, &feat_val) in features.iter().enumerate() {
                score += (feat_val * weights[feat_idx]).ln();
            }
            scores[org_idx] = score.exp();
        }
        
        // Normalize to probabilities
        let sum: f32 = scores.iter().sum();
        for score in &mut scores {
            *score /= sum;
        }
        
        let (best_org_idx, &best_prob) = scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        
        // Confidence interval: use training-set variance as proxy
        let confidence_interval = self.compute_confidence_interval(best_org_idx, best_prob);
        
        MYCINClassification {
            predicted_organism: ORGANISMS[best_org_idx],
            confidence_interval,
            last_retrain_date: self.last_retrain,
            next_retrain_required: self.next_retrain_required,
        }
    }
    
    // Run diagnostic test suite at startup
    pub fn diagnostic_accuracy_check(&self) -> DiagnosticResult {
        let mut correct = 0;
        for (input, gold_std) in self.diagnostic_cases.iter() {
            let classification = self.classify(&input.features);
            if classification.predicted_organism == gold_std.expected_organism {
                correct += 1;
            }
        }
        
        let accuracy = correct as f32 / self.diagnostic_cases.len() as f32;
        
        if accuracy < self.min_acceptable_accuracy {
            DiagnosticResult::DegradationDetected {
                accuracy,
                min_required: self.min_acceptable_accuracy,
                alert: format!(
                    "MYCIN accuracy has degraded to {:.2}%. Next retraining required by {}.",
                    accuracy, self.next_retrain_required
                ),
            }
        } else {
            DiagnosticResult::Healthy { accuracy }
        }
    }
    
    fn compute_confidence_interval(&self, org_idx: usize, prob: f32) -> ConfidenceInterval {
        // Using binomial proportion confidence interval (Wilson score)
        let n = TRAINING_SET_SIZE as f32;
        let z = 1.96;  // 95% confidence
        let denominator = 1.0 + (z * z) / n;
        let numerator_center = prob + (z * z) / (2.0 * n);
        let margin = z * (prob * (1.0 - prob) / n).sqrt() / denominator;
        
        ConfidenceInterval {
            lower: (numerator_center - margin).max(0.0),
            upper: (numerator_center + margin).min(1.0),
        }
    }
}
```

### Code Locations

- **Classification logic**: `src/classical_ai/mycin.rs`
- **Confidence interval computation**: Add to same file, method `compute_confidence_interval`
- **Diagnostic test suite**: Embed in binary as `&'static [(DiagnosticInput, DiagnosticGoldStandard)]`
- **Retraining gate**: `src/config.rs` — add `[mycin] next_retrain_required = "2026-07-28"`
- **Startup health check**: `src/bin/ralph.rs` — call `mycin.diagnostic_accuracy_check()` before accepting requests

### Trade-Off Analysis

- **Determinism cost**: None. Confidence interval is pure arithmetic (binomial bounds).
- **Latency cost**: Diagnostic check runs once at startup (one-time cost, not per-request).
- **Auditability gain**: Every output now includes explicit uncertainty range, enabling clinician override.
- **Feedback loop restored**: Diagnostic cases + retraining schedule enable continuous validation; silent drift is detected.

---

## III. STRIPS: Explainable Reachability + Replanning Trigger

### The Critique

Fikes & Nilsson identified: gradient-boosted reachability prediction on 16-bit state space is overfitting to training data. Real planning requires runtime contingency handling. Compile-time precomputation has no explanation, no plan, no adaptability when preconditions fail.

### Structural Mitigation

1. **Replace binary reachability prediction with (reachable: bool, confidence: f32, failed_precondition: Option<Precondition>)**:
   - System predicts not just "reachable" but "reachable with 0.72 confidence, assuming [Precondition X]"
   - If preconditions are violated at runtime, system flags the state change

2. **Explainable decision trees**:
   - Gradient boosted trees compiled as if-then-else statements (not opaque decision matrices)
   - Inspector can trace: "State bits [3,7,12] match training region. Predicted reachable."

3. **Replanning trigger**:
   - Embed a lightweight backup planner for contingency cases
   - If compiled prediction fails (operator reports goal not reached), invoke replanner
   - Planner is not compiled; it runs at runtime but only on failure

### Implementation Sketch

```rust
// File: src/classical_ai/strips.rs

#[derive(Clone, Copy)]
pub struct ReachabilityPrediction {
    pub reachable: bool,
    pub confidence: f32,
    pub required_preconditions: &'static [Precondition],
    pub explanation: &'static str,  // Human-readable decision path
}

pub struct STRIPSDeterministic {
    // Compiled decision tree: sequence of if-then-else rules
    decision_rules: &'static [DecisionRule],
    
    // For each prediction region, store the learned preconditions
    precondition_map: &'static [(StateRegion, &'static [Precondition])],
    
    // Backup runtime planner (light-weight, only on failure)
    fallback_planner: Option<RuntimePlanner>,
}

pub struct DecisionRule {
    pub state_bit_mask: u16,
    pub state_bit_value: u16,
    pub then_reachable: bool,
    pub confidence: f32,
    pub explanation: &'static str,
}

impl STRIPSDeterministic {
    pub fn predict_reachability(&self, state: u16) -> ReachabilityPrediction {
        // Execute compiled decision tree
        for rule in self.decision_rules {
            if (state & rule.state_bit_mask) == rule.state_bit_value {
                let preconditions = self.precondition_map
                    .iter()
                    .find(|(region, _)| region.contains(state))
                    .map(|(_, prec)| *prec)
                    .unwrap_or(&[]);
                
                return ReachabilityPrediction {
                    reachable: rule.then_reachable,
                    confidence: rule.confidence,
                    required_preconditions: preconditions,
                    explanation: rule.explanation,
                };
            }
        }
        
        // Default: unrecognized region, low confidence
        ReachabilityPrediction {
            reachable: false,
            confidence: 0.0,
            required_preconditions: &[],
            explanation: "State outside training distribution",
        }
    }
    
    // Runtime replanning: triggered only if prediction fails
    pub fn replan_on_failure(
        &self,
        initial_state: u16,
        goal_state: u16,
        failed_precondition: Option<Precondition>,
    ) -> Result<Vec<Action>, PlanningError> {
        if let Some(planner) = &self.fallback_planner {
            // Log the failure for future retraining
            log_replan_trigger(initial_state, goal_state, failed_precondition);
            
            // Run lightweight planner (BFS, bounded depth)
            planner.find_plan(initial_state, goal_state, depth_limit=8)
        } else {
            Err(PlanningError::NoFallbackPlanner)
        }
    }
}

// Instrumentation: user feedback on whether goal was achieved
pub fn record_reachability_outcome(
    prediction: &ReachabilityPrediction,
    state: u16,
    goal_achieved: bool,
) {
    let record = OutcomeRecord {
        timestamp: SystemTime::now().unix_timestamp(),
        initial_state: state,
        predicted_reachable: prediction.reachable,
        predicted_confidence: prediction.confidence,
        actual_outcome: goal_achieved,
        match_prediction: prediction.reachable == goal_achieved,
    };
    
    append_outcome_log(record);
}
```

### Code Locations

- **Decision tree evaluation**: `src/classical_ai/strips.rs`
- **Precondition metadata**: Compile into `decision_rules` as `explanation: &'static str`
- **Replanning trigger**: `src/classical_ai/strips.rs` method `replan_on_failure`
- **Fallback planner**: Either stub (returns error) or lightweight BFS in `src/classical_ai/strips_fallback_planner.rs`
- **Outcome instrumentation**: `src/agentic/feedback_emitter.rs`

### Trade-Off Analysis

- **Determinism cost**: Minimal. Decision tree execution is deterministic. Fallback planner runs only on reported failures (off-critical-path).
- **Latency cost**: Decision tree is O(NUM_RULES); typical 50–100 rules = ~1µs. Fallback planner not on hot path.
- **Auditability gain**: Every prediction now includes explicit explanation and preconditions. Inspector can verify decision rule logic.
- **Feedback loop restored**: Outcome logs enable retraining; replanning on failure demonstrates system has learned failure modes.

---

## IV. SHRDLU: Feature Vector Versioning + Feedback-Driven Ontology Evolution

### The Critique

Winograd identified: 30-feature vector is a closed abstraction of a microworld. The moment new object types, relations, or constraints appear, the system fails silently. Logistic regression on fixed features cannot adapt to new ontologies.

### Structural Mitigation

1. **Embed feature vector versioning**:
   - Each prediction includes feature vector version (e.g., "v2.1")
   - If user describes a feature not in v2.1, system flags: "ONTOLOGY_MISMATCH"
   - Old binaries can be retired; new binaries have expanded feature sets

2. **Feasibility confidence + context qualification**:
   - Output: "(feasible: 0.67, context: [block_world], out_of_distribution: false)"
   - If input description uses unknown predicates, set `out_of_distribution: true`
   - User sees: "Feasible [0.67 confidence], but command uses unknown predicates. Consult human."

3. **Feedback-driven ontology evolution**:
   - When user reports "system said feasible but it wasn't," log the misclassified case + any new predicates
   - Retraining incorporates new features + retrained logistic regression
   - Next binary has expanded feature set

### Implementation Sketch

```rust
// File: src/classical_ai/shrdlu.rs

pub const SHRDLU_FEATURE_VECTOR_VERSION: &str = "2.1";

#[derive(Clone, Copy)]
pub struct FeasibilityPrediction {
    pub feasible: bool,
    pub confidence: f32,
    pub feature_vector_version: &'static str,
    pub known_features: u32,  // Bitmask: which features were recognized
    pub unknown_predicates: &'static [&'static str],  // Predicates not in feature set
    pub out_of_distribution: bool,
}

pub struct SHRDLUDeterministic {
    // Logistic regression model: learned once, compiled
    feature_weights: &'static [f32],  // Length NUM_FEATURES
    intercept: f32,
    feature_vector_version: &'static str,
    known_predicate_names: &'static [&'static str],
}

impl SHRDLUDeterministic {
    pub fn predict_feasibility(&self, command: &str) -> FeasibilityPrediction {
        // Parse command to extract features
        let (features, known_features, unknown_predicates) = 
            self.extract_features(command);
        
        // Logistic regression: sum weights, apply sigmoid
        let mut logit = self.intercept;
        for (i, &weight) in self.feature_weights.iter().enumerate() {
            logit += weight * features[i];
        }
        
        let confidence = 1.0 / (1.0 + (-logit).exp());  // Sigmoid
        let feasible = confidence > 0.5;
        
        let out_of_distribution = !unknown_predicates.is_empty();
        
        FeasibilityPrediction {
            feasible,
            confidence,
            feature_vector_version: self.feature_vector_version,
            known_features,
            unknown_predicates,
            out_of_distribution,
        }
    }
    
    fn extract_features(&self, command: &str) -> (Vec<f32>, u32, Vec<&str>) {
        let mut features = vec![0.0; self.feature_weights.len()];
        let mut known_features: u32 = 0;
        let mut unknown_predicates = Vec::new();
        
        // Tokenize command; match against known predicates
        for token in command.split_whitespace() {
            if let Some((idx, _)) = self.known_predicate_names
                .iter()
                .enumerate()
                .find(|(_, name)| **name == token)
            {
                features[idx] = 1.0;
                known_features |= 1 << idx;
            } else if is_predicate_like(token) {
                // Detected predicate syntax but not in known set
                unknown_predicates.push(token);
            }
        }
        
        (features, known_features, unknown_predicates)
    }
}

// Instrumentation: user feedback on whether command succeeded
pub fn record_feasibility_outcome(
    prediction: &FeasibilityPrediction,
    command: &str,
    success: bool,
    new_predicates_discovered: Vec<String>,
) {
    let record = FeasibilityOutcomeRecord {
        timestamp: SystemTime::now().unix_timestamp(),
        command: command.to_string(),
        predicted_feasible: prediction.feasible,
        predicted_confidence: prediction.confidence,
        actual_outcome: success,
        feature_vector_version: prediction.feature_vector_version.to_string(),
        new_predicates: new_predicates_discovered,
        out_of_distribution: prediction.out_of_distribution,
    };
    
    append_feasibility_outcome_log(record);
}
```

### Code Locations

- **Feature extraction & logistic regression**: `src/classical_ai/shrdlu.rs`
- **Feature vector versioning**: Constant `SHRDLU_FEATURE_VECTOR_VERSION: &str`
- **Unknown predicate detection**: Method `extract_features`, check against `known_predicate_names`
- **Outcome logging**: `src/agentic/feedback_emitter.rs`
- **Retraining procedure**: Document in `docs/RETRAINING.md` — rebuild binary with new features + retrain logistic regression

### Trade-Off Analysis

- **Determinism cost**: None. Feature extraction and logistic regression are pure arithmetic.
- **Latency cost**: Feature extraction is O(|command|) tokenization; regression is O(NUM_FEATURES) dot product.
- **Auditability gain**: Every prediction includes feature set version and flags unknown predicates. Inspector can verify regression coefficients.
- **Feedback loop restored**: Outcome logs capture new predicates + misclassifications, enabling ontology evolution in next release.

---

## V. HEARSAY-II: Hierarchical Confidence Propagation + Speaker-Adaptive Confidence Bounds

### The Critique

Reddy & Erman identified: Borda count treats hierarchical levels (Acoustic → Phoneme → Syllable → Word) as independent sources, but they are causally dependent. Double-counting evidence. Confidence factors are discarded, replaced by binary decisions.

### Structural Mitigation

1. **Replace Borda count with hierarchical confidence propagation**:
   - Each level emits (hypothesis, confidence_factor, supporting_evidence)
   - Word-level CF = min(Syllable_CF, Phoneme_CF, Acoustic_CF)
   - Conservative fusion: if any level is uncertain, the result is uncertain

2. **Speaker-adaptive confidence bounds**:
   - Compile reference acoustic models for "canonical" speakers (male, female, elderly, etc.)
   - At runtime, measure acoustic deviation from reference
   - Confidence degrades if input is far from training distribution

3. **Dependency-aware fusion**:
   - Emit not just confidence factors but explicit causal chains
   - Word recognized as [cat] because Phoneme-level CF=0.9 (driven by Acoustic CF=0.85)
   - User sees: "[cat] (0.85 confidence, limited by acoustic signal clarity)"

### Implementation Sketch

```rust
// File: src/classical_ai/hearsay.rs

#[derive(Clone, Copy)]
pub struct ConfidenceFactor {
    pub value: f32,  // [0.0, 1.0]
    pub limited_by: &'static str,  // Which level is the bottleneck
}

pub struct LevelHypothesis {
    pub hypothesis: &'static str,
    pub confidence_factor: ConfidenceFactor,
    pub supporting_evidence: &'static str,  // For auditability
}

pub struct HearsayIIWord {
    pub word: &'static str,
    pub acoustic_cf: ConfidenceFactor,
    pub phoneme_cf: ConfidenceFactor,
    pub syllable_cf: ConfidenceFactor,
    pub final_cf: ConfidenceFactor,
    pub speaker_deviation: f32,  // Distance from canonical speaker model
}

pub struct HearsayIIDeterministic {
    // Compiled blackboard with per-level CFs
    acoustic_models: &'static [AcousticFeatures],
    phoneme_models: &'static [PhonemeHypothesis],
    syllable_models: &'static [SyllableHypothesis],
    word_hypotheses: &'static [WordHypothesis],
    
    // Reference speaker models for adaptive confidence
    canonical_speakers: &'static [CanonicalSpeaker],
}

impl HearsayIIDeterministic {
    pub fn recognize(&self, audio_features: &AudioSignal) -> HearsayIIWord {
        // Level 1: Acoustic
        let acoustic_hyp = self.match_acoustic_features(audio_features);
        let acoustic_cf = ConfidenceFactor {
            value: acoustic_hyp.confidence,
            limited_by: "acoustic_models",
        };
        
        // Level 2: Phoneme (depends on Acoustic)
        let phoneme_hyp = self.match_phoneme(audio_features, &acoustic_hyp);
        let phoneme_cf = ConfidenceFactor {
            value: phoneme_hyp.confidence,
            limited_by: if phoneme_hyp.confidence < acoustic_cf.value {
                "phoneme_models"
            } else {
                "acoustic_models"
            },
        };
        
        // Level 3: Syllable (depends on Phoneme)
        let syllable_hyp = self.match_syllable(audio_features, &phoneme_hyp);
        let syllable_cf = ConfidenceFactor {
            value: syllable_hyp.confidence,
            limited_by: if syllable_hyp.confidence < phoneme_cf.value {
                "syllable_models"
            } else {
                phoneme_cf.limited_by
            },
        };
        
        // Level 4: Word (depends on Syllable)
        let word_hyp = self.match_word(audio_features, &syllable_hyp);
        let word_cf = ConfidenceFactor {
            value: word_hyp.confidence,
            limited_by: if word_hyp.confidence < syllable_cf.value {
                "word_hypotheses"
            } else {
                syllable_cf.limited_by
            },
        };
        
        // Speaker deviation: measure distance from canonical speakers
        let speaker_deviation = self.compute_speaker_deviation(audio_features);
        
        // Degrade confidence if speaker is unusual
        let adaptive_final_cf = ConfidenceFactor {
            value: word_cf.value * (1.0 - speaker_deviation / 2.0),  // Max 50% reduction
            limited_by: if speaker_deviation > 0.2 { "speaker_mismatch" } else { word_cf.limited_by },
        };
        
        HearsayIIWord {
            word: word_hyp.word,
            acoustic_cf,
            phoneme_cf,
            syllable_cf,
            final_cf: adaptive_final_cf,
            speaker_deviation,
        }
    }
    
    fn compute_speaker_deviation(&self, audio_features: &AudioSignal) -> f32 {
        let mut min_distance = f32::INFINITY;
        
        for canonical in self.canonical_speakers {
            let distance = self.acoustic_distance(audio_features, &canonical.reference_features);
            min_distance = min_distance.min(distance);
        }
        
        // Normalize: 0.0 = matches canonical, 1.0 = completely unknown
        (min_distance / ACOUSTIC_DISTANCE_SCALE).min(1.0)
    }
}

// Instrumentation: user correction on recognized word
pub fn record_recognition_outcome(
    result: &HearsayIIWord,
    user_correction: Option<&str>,
) {
    let correct = user_correction.is_none() || user_correction == Some(result.word);
    
    let record = RecognitionOutcomeRecord {
        timestamp: SystemTime::now().unix_timestamp(),
        recognized_word: result.word.to_string(),
        final_confidence: result.final_cf.value,
        limited_by: result.final_cf.limited_by.to_string(),
        acoustic_cf: result.acoustic_cf.value,
        phoneme_cf: result.phoneme_cf.value,
        syllable_cf: result.syllable_cf.value,
        speaker_deviation: result.speaker_deviation,
        correct: correct,
        user_correction: user_correction.map(|s| s.to_string()),
    };
    
    append_recognition_outcome_log(record);
}
```

### Code Locations

- **Hierarchical blackboard**: `src/classical_ai/hearsay.rs`
- **Confidence propagation**: Method `recognize`, explicit CF chaining
- **Speaker-adaptive confidence**: Method `compute_speaker_deviation`
- **Canonical speaker models**: Embed as `&'static [CanonicalSpeaker]`
- **Outcome logging**: `src/agentic/feedback_emitter.rs`

### Trade-Off Analysis

- **Determinism cost**: None. Blackboard matching and distance metrics are deterministic.
- **Latency cost**: Four levels of model matching (~1–10ms per level depending on feature extraction). Speaker deviation is O(NUM_CANONICAL_SPEAKERS), typically 4–8.
- **Auditability gain**: Every output includes explicit CF chain and speaker-adaptation factor. Inspector can trace why confidence was degraded.
- **Feedback loop restored**: Outcome logs enable retraining on new speakers + confidence calibration; speaker deviation flags out-of-distribution inputs.

---

## Conclusion: Determinism + Feedback Are Orthogonal

The adversarial reviewers' central claim was correct: compile-time reasoning cannot adapt on its own. **However**, determinism and learning are orthogonal properties.

A system can be:
1. **Deterministic at T=0** (binary is immutable; outputs are audit-traceable)
2. **Instrumented for feedback** (every decision emits confidence, preconditions, unexplained features)
3. **Scheduled for retraining** (mandatory expiration dates; periodic diagnostic checks)
4. **Learning-enabled at T=1** (feedback logs drive next-generation binary)

With these mitigations, Compiled Cognition adds feedback loops and drift detection while preserving:
- **Auditability**: Every output is deterministic, traceable to compiled rules
- **Immutability**: The running binary never changes; learning happens at compile time
- **Safety**: Confidence bounds and expiration dates prevent silent failures
- **Honesty**: The system explicitly states when it is out of distribution or past retraining deadline

The reviewers asked: *How do you ensure embedded reasoning is correct?* Our answer: You don't—not alone. But you can make the system transparent about its uncertainty, attach confidence bounds, and schedule mandatory retraining so that *humans + learning loops* validate and evolve it over time.

---

## Appendix: Integration Checklist

For each system, implement in order:

1. **Confidence/explanation output** (deterministic, affects API contract)
2. **Diagnostic checks or deprecation horizon** (startup / request-time gate)
3. **Feedback instrumentation** (append-only logging)
4. **Retraining schedule** (external process, not in binary)
5. **Outcome validation** (optional: A/B test new binary before full deployment)

**Files to create/modify**:
- `src/classical_ai/*.rs` — Prediction logic + confidence calculations
- `src/agentic/feedback_emitter.rs` — Unified instrumentation
- `src/config.rs` — Deprecation dates, retraining schedules
- `src/bin/ralph.rs` — Startup health checks
- `docs/RETRAINING.md` — Procedure for rebuilding binaries (e.g., "rerun training with new logs")

**Testing**:
- Unit tests: Verify confidence bounds are computed correctly
- Integration tests: Verify feedback logs are well-formed JSON
- Regression tests: Ensure determinism (same input = same output every time)
- Adversarial tests: Inject out-of-distribution inputs; verify `out_of_distribution` flag is set correctly

---

**Status**: Ready for implementation. Each mitigation is low-risk and preserves the determinism guarantee while restoring the human-in-the-loop feedback that the original systems required.
