# Adversarial Review: Compiled Cognition
## A Technical Critique by the Original AI System Designers

**Date:** April 2026  
**Reviewers:** Weizenbaum (ELIZA), Feigenbaum & Lederberg (MYCIN), Fikes & Nilsson (STRIPS), Winograd (SHRDLU), Reddy & Erman (Hearsay-II)  
**Verdict:** Intellectually coherent but systemically overconfident.

---

## Executive Adversarial Summary

The dteam authors have achieved something genuine: embedding symbolic reasoning at nanosecond latency. But they have conflated *technical achievement* with *semantic correctness*, and claimed *structural safety* for systems that are still subject to the same validity crises their originals faced—only faster, and now immutable.

The claim "A = μ(O*)" — that action is lawful iff projected from a compile-time closed set O* — is not wrong. It is *incomplete*. It says nothing about whether O* itself is correct, whether the rules that generate O* survive contact with the real world, or what happens when reality deviates from the assumptions baked into the binary.

We review each system and the paradigm as a whole.

---

## I. ELIZA: "Determinism Proves Nothing"

**Reviewer: Joseph Weizenbaum (MIT, 1966)**

> "The computer programmer is a creator of universes for which he alone is responsible."

### The Compiled Cognition Claim

dteam claims:
- ELIZA intent classification via keyword matching is deterministic (✓)
- Embedded as const, auditable, zero runtime variance (✓)
- Therefore, safe for deployment (✗)

### The Adversarial Critique

**1. Determinism ≠ Correctness**

A system can be perfectly deterministic and profoundly wrong. ELIZA's keyword matching is deterministic. Given "I'm sad," it will always return the same template response. This proves nothing about whether that response is *appropriate* for the human receiving it.

When I created ELIZA in 1966, the revelation was unexpected: people *believed* the system understood them, projecting understanding onto patterns that had no semantic content. Today, your "deterministic" ELIZA still has no understanding. But because it's embedded in a binary, shipped to production, and immutable, the illusion is now *structural*. 

The responsibility transfers from "we know this is a parlor trick" to "this is what the user will see, forever."

**2. Compile-Time Brittleness**

Your ELIZA has 42 hand-coded keywords and 15 template responses. These were chosen once, at compile time. They are now frozen. What happens when:

- A user type-aliases emotional language unseen in your training set?
- Domain shifts (e.g., customer support moving from product issues to refund requests)?
- The keyword mapping proves biased toward one demographic?

You cannot retrain. You cannot adapt. You ship another binary. This is not safety; this is immutability masquerading as rigor.

**3. The "Auditability" False Comfort**

You claim every output is audit-traceable because the rules are in git. But auditability is not causation. Showing that line 42 of `eliza.rs` says "respond with [template_X]" does not explain *why* that is the right response for a human in distress. It only documents the frozen choice.

I was humbled when psychiatrists began using ELIZA to supplement patient interviews. I had created an illusion of therapeutic competence. Today, your compiled ELIZA is the same illusion, but faster, and now guaranteed to persist.

### Recommendation

Remove the claim that determinism implies safety. Replace it with: "Deterministic ≠ correct; determinism is a prerequisite for auditability, not a proof of validity."

The binary should ship with a human-readable explanation of its intent classification rules, and a *mandatory* deprecation timeline (e.g., "expires in 24 months; retrain required").

---

## II. MYCIN: "Compile-Time Data Starvation"

**Reviewers: Edward Feigenbaum & Joshua Lederberg (Stanford, 1970s)**

> "The expert system knows only what we taught it. When reality exceeds that, the system collapses silently."

### The Compiled Cognition Claim

dteam claims:
- MYCIN reasoning chains are deterministic and auditable (✓)
- Training on domain data is frozen at compile time (✓)
- Therefore, organisms detected at runtime are safely classified (✗)

### The Adversarial Critique

**1. Generalization Cannot Be Compiled**

MYCIN was trained on ~450 cases of bacterial infections. We spent years understanding which features (Gram stain, culture site, patient risk factors) matter for diagnosis. But training data is *always* incomplete.

Your version uses a Naive Bayes classifier on 16 binary features, trained once and compiled as const lookup tables. This is the epitome of compile-time brittleness:

- **Novel organisms emerge.** A new pathogen, unseen in training, arrives. Your system classifies it as one of the 8 trained classes with high false confidence. Why? Because the feature space is closed. The organism does not exist in O*.

- **Feature-value distributions shift.** In 1976, Staphylococcus aureus was sensitive to certain antibiotics. By 2026, resistance patterns have evolved dramatically. Your compile-time probability table is obsolete but immutable.

- **Covariate shift.** The patient population changes (e.g., immunocompromised COVID survivors have different infection patterns). Your training data did not include these cases. The model drifts without ever signaling degradation.

**2. Confidence Without Calibration**

Your Naive Bayes returns a boolean (infected / not infected). But reality is probabilistic. MYCIN at least returned a *confidence factor* (CF), which we learned to distrust when the model was uncertain.

Compiling a binary classifier removes the confidence interval entirely. You either predict "infected" or not. A clinician has no way to know if the system is 55% confident or 99% confident. Determinism has hidden uncertainty, not eliminated it.

**3. Silent Failure is the Greatest Risk**

The worst diagnosis is the confident wrong diagnosis. An ELIZA user knows they're talking to a chatbot. A MYCIN user (the clinician) knows they're consulting a tool. But when MYCIN is embedded as const in a clinical workflow, when its output is auto-actioned, when it runs at "nanosecond latency" like a direct hardware computation—the user may forget it is a statistical model trained on 50-year-old data.

We learned this the hard way. Expert systems failed spectacularly not when they were wrong, but when they were confidently wrong and no one caught it.

### Recommendation

1. Do not claim compile-time training guarantees runtime correctness.
2. Ship with a *required* periodic retraining (minimum every 12 months).
3. Add a "confidence range" output even if binary, so users know the model's uncertainty.
4. Log every prediction; compare against ground truth; alert if accuracy drops below a threshold.

---

## III. STRIPS: "Plan Materialization Breaks on Contact"

**Reviewers: Richard Fikes & Nils Nilsson (Stanford, 1971)**

> "A plan is not the world. The moment you execute it, the world changes in ways you did not anticipate."

### The Compiled Cognition Claim

dteam claims:
- STRIPS goal reachability is deterministic search over a state space (✓)
- Reachability predictions are pre-computed and compiled as const (✓)
- Therefore, goals are reliably achieved at runtime (✗)

### The Adversarial Critique

**1. Pre-Materialized Plans Are Unicorn Plans**

Your STRIPS system trains on ~100 goal-reachability labels, then uses gradient boosting to predict reachability from initial state features. This is not planning; this is *memorized outcome probability*.

We designed STRIPS to *search* for plans because:
- Not every sequence of actions is previsible.
- Contingencies arise: preconditions fail, side effects interact, resource constraints tighten.
- A planner must reason about the interaction of sub-goals.

Your gradient-boosted predictor says "yes, this goal is reachable" or "no, it is not." But it has no plan. It has no sequence of actions. If the user asks "how do I reach this goal?" your system cannot explain. It only predicts.

Real planning requires generating the plan *at runtime*, when you know the actual state.

**2. The State-Space Explosion You've Ignored**

You represent STRIPS state as a 16-bit bitmask. This is 65,536 possible states. Your training set has maybe 64 examples. You have seen ~0.1% of the state space.

The gradient boosting model fits a decision tree. That tree generalizes *between* the examples it has seen. But generalization in a 16-bit space is not generalization in the real world. The real world has continuous state, partial observability, stochastic actions, and second-order effects.

Compiling that model as const does not make it correct. It only makes it consistently wrong.

**3. Why Compile-Time Reachability Analysis Fails**

In real manufacturing (which your "Manufacturing Workflow" profile claims to model):

- A machine breaks down (a state change not in your training set).
- A part arrives late (concurrency).
- A worker forgets a step (nondeterminism).
- Scheduling constraints tighten (resources are now scarcer).

Your compiled reachability predictor says "HOLDING_A is reachable from INITIAL_STATE" based on 50-year-old logic and a 2026 training set. Then the world changes. The goal is no longer reachable. The system has no mechanism to detect this, no way to adapt, and no explanation for why the prediction failed.

We learned this the hard way. STRIPS plans that worked in simulation failed in the real world 30% of the time.

**4. The Claim That Determinism Solves This**

You claim the conformance trace (process mining) proves the system worked. But this is backwards:

- Process mining shows *what happened*.
- It does not prove *what should have happened was achievable*.
- It does not explain *why the goal failed if the prediction was wrong*.

Auditing a deterministic failure is not the same as preventing it.

### Recommendation

1. Stop claiming "goal reachability" is precomputable. It is not.
2. If you must use a learned model, make it a *guide* for runtime planning, not a replacement.
3. Add a "replanning" mechanism: at runtime, if the plan fails, replan using a real planner.
4. Ship with a confidence interval, not a boolean. "Reachable [0.72 confidence]" is more honest than "Reachable: true."

---

## IV. SHRDLU: "Brittle Domain Closure"

**Reviewer: Terry Winograd (Stanford, 1968–1970)**

> "I came to believe that the fundamental assumptions we made about language and reasoning were wrong. The system was brittle because the world is not a block."

### The Compiled Cognition Claim

dteam claims:
- SHRDLU spatial reasoning is logistic regression on 30-dimensional state features (✓)
- Learned once, compiled as const, deterministic (✓)
- Therefore, feasibility of commands is reliably predicted (✗)

### The Adversarial Critique

**1. The Block World Was Always a Lie**

I designed SHRDLU in a microworld: a table, blocks, an arm. Clear predicates. ON(A, B) is true or false. The world was closed. But I knew even then: real language understanding is not manipulation of a 5×5 block world. Real understanding is open-ended, grounded in human experience, sensitive to context, rich in ambiguity.

Your SHRDLU uses a 30-feature vector derived from a 64-bit bitmask. This is still a block world. It is never the real world.

**2. Feasibility Is Context-Dependent**

In a block world, "PickUp(A)" is feasible iff CLEAR(A) ∧ ON_TABLE(A) ∧ ARM_EMPTY. Your logistic regression learns this pattern from training data, then compiles it.

But in the real world, "Pick up the coffee cup" is feasible only if:
- The cup is not hot (tactile feedback).
- The user is not injured (biomechanics).
- The cup is not valuable (ownership/social context).
- The user wants to pick it up (intent).

None of these are in your feature vector. Your system will confidently predict feasibility in a world it has never modeled.

**3. Compile-Time Assumes Static Ontology**

You baked 30 features into const arrays. Those features are the *complete* abstraction of world state. But abstractions are domain-specific and brittle.

If the real world introduces:
- A new object type (not A, B, or C)?
- A new relation (ATTACHED, not just ON)?
- A new constraint (weight limits, thermal limits)?

Your system has no way to represent it. The feature vector is closed. You cannot adapt. You ship a new binary.

This is why SHRDLU failed in the real world. It was too faithful to its microworld.

**4. Why Logistic Regression on Fixed Features Fails**

Your classifier learned the decision boundary in 2026 training data. That boundary is optimal *for that data*. But the real world is non-stationary:

- As operators gain experience, they find new ways to make commands feasible (e.g., creative gripper designs).
- As hardware changes, feasibility changes (a faster arm can pick up more delicate objects).
- As goals shift, the interpretation of feasibility changes.

Your logistic regression is frozen. It cannot learn. It cannot adapt. It will gradually fail.

### Recommendation

1. Acknowledge that symbolic reasoning in a microworld is not reasoning about the real world.
2. Do not claim compile-time feasibility predicts runtime feasibility.
3. If you must use this system, treat it as a *suggestion*, not a guarantee.
4. Add a human feedback loop: every time the system's prediction is wrong, log it. Trigger retraining if error rate exceeds a threshold.

---

## V. HEARSAY-II: "Borda Count Assumes Independence"

**Reviewers: Raj Reddy & Lee Erman (CMU, 1976)**

> "The blackboard works because sources disagree in systematic ways. Fusing them requires understanding *why* they disagree, not just averaging their votes."

### The Compiled Cognition Claim

dteam claims:
- Hearsay-II blackboard produces per-level confidence factors (✓)
- Borda count fuses these into a final decision (✓)
- Therefore, multi-source consensus is robust (✗)

### The Adversarial Critique

**1. Borda Count Assumes Source Independence**

Borda count works when sources are orthogonal. Source A votes on one aspect, Source B on another. Their rankings are uncorrelated.

But in Hearsay-II, the four levels (Acoustic, Phoneme, Syllable, Word) are *hierarchically dependent*. The Phoneme-level hypothesis is informed by Acoustic-level evidence. The Syllable level depends on Phoneme. The Word level depends on Syllable.

This is not independence. This is *causality*. Borda count ignores causality. It treats a phoneme-level CF as an independent vote, when in fact it is downstream of acoustic-level uncertainty.

When you fuse dependent sources with Borda count, you are *double-counting* evidence. The acoustic feature that weakly supports "S" influences the phoneme CF, which then influences the syllable CF, which influences the word CF. Then Borda count weights them equally as if they were independent sources.

Result: illusory consensus. The system appears to agree with itself and calls it fusion.

**2. The Acoustic Features You Cannot Compile**

Hearsay-II works because knowledge sources *adapt* to the signal. A speaker with an accent produces different acoustic features. The system learns to compensate.

Your version:
- Runs the blackboard once with a hardcoded seed input (0.9 CF for Acoustic).
- Extracts per-level CFs.
- Compiles them as const.

But you have never seen the user who will actually speak into the system. The acoustic features will be different. The blackboard will produce different CFs. Borda count will fuse different votes.

You are compiling fusion rules trained on *one input*, then applying them to *all inputs*. The acoustic features of a female speaker, an elderly speaker, a speaker with a speech impediment—none of these are in your compile-time model.

**3. Why Consensus Breaks on Real Data**

In real speech recognition, Hearsay-II's power came from having *multiple* independent knowledge sources: acoustic models, phoneme models, language models, trained on different corpora, with different failure modes.

Your version has four levels of the *same* system. They are not independent. They are facets of the same underlying error.

When acoustic-level evidence is weak, all downstream levels are weak. Borda count cannot save you. You are fusing noise with itself.

**4. Silent Degradation**

Borda count returns a binary decision: "Yes, this word is in the top-K." It gives no confidence interval. Your system will confidently misrecognize a word, and the user will have no signal of degradation.

We learned that confidence factors (even if imperfect) were essential. They told the operator: "The system is uncertain; verify the decision." You have removed the uncertainty signal.

### Recommendation

1. Do not claim Borda count fuses independent sources. Test for correlation among your sources.
2. Add a "consensus confidence" metric: when your sources disagree, lower the confidence, not just average their votes.
3. Do not freeze the blackboard parameters. Retrain periodically on new speakers, new acoustic environments.
4. Ship with a confidence interval, not a boolean. "Word recognized: [cat] (0.63 confidence)" is safer than "Word recognized: true."

---

## VI. The Paradigm Critique: "Compile-Time Reasoning Cannot Scale"

**Joint Observation by All Reviewers**

### The Central Claim We Dispute

dteam claims:

> "Machine intelligence can now be compiled into the artifact itself. Reasoning moves from runtime service to execution substrate."

This is *technically true* but *semantically incomplete*. Yes, you can embed reasoning in a binary. Yes, it runs fast. Yes, it is deterministic. But you have not solved the fundamental problem: *how do you ensure the embedded reasoning is correct?*

### The Generalization Trap

Each of our systems was built in a controlled domain:

- ELIZA: keyword matching in English psychotherapy dialogue.
- MYCIN: medical diagnosis in infectious disease.
- STRIPS: goal-stack planning in block worlds.
- SHRDLU: spatial reasoning in microworlds.
- Hearsay-II: speech recognition in acoustic signal processing.

In each domain, we worked *hard* to constrain the problem. We defined predicates, axioms, ontologies. We made the world small enough to reason about.

When we tried to generalize—to apply our systems to new domains—they failed. Not because the systems were poorly implemented, but because the ontology did not transfer.

Your answer: compile the ontology as const. This does not solve generalization; it *prevents* adaptation. You have made it *structurally impossible* to learn new ontologies.

### The Real World Is Adversarial

Every system we built was tested against:

- **Adversarial inputs**: cases designed to break the system.
- **Covariate shift**: the distribution changed between training and deployment.
- **Feedback loops**: the system's outputs changed the world, which changed the inputs.
- **Rare events**: edge cases not in the training set.

Your compile-time approach has no defense against these. The moment a user finds an adversarial input, the system is broken. You cannot patch it; you must ship a new binary.

### The "Binary IS the Proof" Claim Is Overconfident

You claim: "This binary IS the proof. The models are embedded. No loading. No drift. The system cannot produce unacceptable outputs because O* is compiled in and immutable."

This confuses *determinism* with *correctness*. The binary is deterministic. But O* (the set of possible outputs) may be:

- Incomplete (missing valid actions).
- Incorrect (containing invalid actions).
- Outdated (trained on stale data).
- Biased (learned from biased examples).

The binary is immutable, but it was correct only *at the moment of compilation*. It degrades over time as the real world drifts away from the training distribution.

### The Auditability Mirage

You claim auditability is "structural, not procedural." A regulator can inspect O* and certify all outputs as acceptable.

But this requires:

1. **Complete specification of O*.** For ELIZA, O* is 15 template responses. For MYCIN, O* is 8 organism classes. But in a real system with thousands of rules and millions of parameters, O* is incomprehensibly large.

2. **Human understanding of the rules.** Even if you list all rules in git, does the regulator understand the *implications* of those rules? When rule A and rule B both apply, which wins? The regulator is performing compliance theater, not validation.

3. **Assumption that training data was representative.** Every system we built violated this. MYCIN was trained on cases chosen by experts, not representative of all infections. ELIZA was trained on psychiatric dialogue, not representative of all conversations. Compile-time training is always incomplete.

### The Process Mining Gambit

You claim process mining proves the system worked: "Every output carries provenance. Process-mining conformance trace is the empirical certificate that A ∈ O*."

But this is circular:

- Process mining shows the system's actual execution trace.
- You compare it against the compiled model.
- It matches (of course—the system is deterministic).
- You declare it "conforms."

But this proves nothing about whether the execution was *correct*. It only proves the system was *consistent*.

A system that consistently misdiagnoses patients conforms to its own rules. A system that consistently makes biased decisions passes its own audit. Process mining does not validate correctness; it only documents consistency.

### The Fundamental Problem: Validation Under Uncertainty

Every system we created had the same problem at the end: *how do you validate a system when reality is more complex than the model?*

- We used field trials.
- We collected user feedback.
- We observed failure modes.
- We retrained.
- We adapted.

You have eliminated all of these. Compile-time reasoning is beautiful because it is frozen. But it is frozen because validation is impossible.

You cannot run field trials on immutable code. You cannot adapt to user feedback. You cannot retrain. You can only apologize for the bug and ship a new binary.

---

## VII. Specific Technical Vulnerabilities

### A. Adversarial Inputs

Your systems have never encountered:

- Typos in ELIZA input (your keyword matcher expects exact strings).
- Confounding symptoms in MYCIN (a presentation that could indicate two diseases equally).
- Circular preconditions in STRIPS (A requires B, B requires C, C requires A).
- Ambiguous spatial descriptions in SHRDLU ("the block on the red one").
- Acoustic noise in Hearsay ("did you say 'bat' or 'mat'?").

These are not edge cases; they are common. Your compiled systems will fail on them, silently and confidently.

### B. Concept Drift

The real world evolves:

- Language evolves (new slang, new contexts for old words).
- Disease patterns evolve (new pathogens, resistance, comorbidities).
- Manufacturing evolves (new equipment, new constraints).
- Acoustic environments evolve (background noise, speaker demographics).

Your compile-time models are frozen in 2026. By 2027, they are stale. By 2030, they are obsolete. But you will still ship them, still embed them in binaries, still claim they are "safe because immutable."

### C. Feedback Loops

The moment your system makes a decision, the world changes:

- ELIZA tells someone they are depressed. They become more depressed. ELIZA's next prediction is now wrong.
- MYCIN recommends an antibiotic. The patient takes it. Resistance develops. MYCIN's priors are now wrong.
- STRIPS predicts a goal is reachable. The operator reaches for it. Another process interrupts. STRIPS's state model is now wrong.

Your compile-time models have no way to observe these loops or adapt to them. They are structurally blind to the consequences of their own decisions.

### D. Regulatory Abandonment Risk

You claim "compliance shifts from probabilistic testing to structural verification." But this is naive.

Regulators will not accept "the binary IS the proof." They will demand:

- Continuous monitoring of outputs against ground truth.
- Periodic revalidation against new data.
- Incident investigation and root-cause analysis.
- Feedback loops to detect and adapt to drift.

In other words, they will demand everything your compile-time approach prevents.

You have not solved compliance; you have made it *politically* harder. When your system fails, the regulator will not be impressed by a deterministic failure. They will demand a human-understandable explanation, retraining, and a plan to prevent recurrence.

---

## VIII. What You Got Right

We do not want to be entirely adversarial. You have accomplished something genuine:

1. **Nanosecond latency**: Embedding symbolic reasoning in binary is novel. You deserve credit for the engineering.

2. **Determinism as a goal**: For systems that must be auditable, determinism is non-negotiable. You are right to pursue it.

3. **Exposing the compile-time design space**: You have shown that reasoning can be materialized at compile time and evaluated at nanosecond scale. This is intellectually interesting.

4. **Multi-substrate pairing**: Pairing symbolic (S) and learned (L) systems is an interesting idea. The weakness is claiming they are independent when they share a training distribution.

5. **Process mining as an audit trail**: Using process mining to reconstruct what the system did is sound. The error is claiming that audit = validation.

---

## IX. What You Should Do

### Short Term (Before Deployment)

1. **Add confidence intervals**: Every output should include a measure of uncertainty.

2. **Ship with a retraining schedule**: Not "retrain if errors exceed threshold" but "retrain every N months, period." Assume your model drifts.

3. **Implement a feedback loop**: Log every prediction. Compare against ground truth. Surface degradation in real time.

4. **Design for human override**: No decision should be final without human approval. The system advises; the human decides.

5. **Document your training data distribution**: Publish what your compile-time model has *seen*. Make it explicit what inputs are outside the model's experience.

### Medium Term (Production)

1. **Canonical interpretations**: When your system's output is wrong, maintain a list of failure modes. Do not pretend determinism prevents learning.

2. **Continuous validation**: Do not wait for catastrophic failure. Continuously compare predictions against outcomes. Trigger alerts when accuracy degrades.

3. **Plan for retraining**: Every 6 months, retrain on new data. Deploy the new binary cautiously. A/B test if possible.

4. **Invest in explainability**: Your claim that rules are "audit-traceable" is only true if someone reads them. Invest in auto-generating explanations so auditors actually understand the system.

### Long Term (Philosophy)

1. **Abandon the "binary IS the proof" claim**: Replace it with "the binary is an audit-traceable snapshot of reasoning at a moment in time. It will drift. Plan accordingly."

2. **Treat Compiled Cognition as a tool, not a paradigm shift**: It is fast. It is auditable. These are valuable properties. Do not claim they solve validation, safety, or compliance.

3. **Reintroduce feedback**: Real reasoning systems learn. Your compile-time approach prevents learning. Either accept that your system is static and plan accordingly, or add learning loops back in.

4. **Stop calling it AngelAI**: This is marketing. It is a bounded, deterministic system. It is useful for specific purposes. It is not good, moral, or lawful by nature. It is just a tool that runs fast.

---

## X. Conclusion

You have built something technically interesting: symbolic reasoning at nanosecond latency, deterministic and auditable. But you have made a philosophical error. You have confused:

- **Determinism** with **correctness**
- **Auditability** with **validation**
- **Immutability** with **safety**

Each of our systems fell because we made similar errors. We believed that formal logic would ensure correctness (ELIZA, STRIPS). We believed that training on domain data would ensure generalization (MYCIN, Hearsay). We believed that closed worlds could reason about open worlds (SHRDLU).

You are repeating these errors at higher speed. Your system will fail, just like ours did. The failure will be faster, more deterministic, and easier to audit. But it will still be a failure.

The difference is that we could adapt. You cannot.

---

## Final Adversarial Recommendation

**Do not deploy this system as production reasoning.** Use it as a:

1. **Advisory tool**: Present to humans for decisions, never auto-execute.
2. **Simulation system**: Use compile-time speed for high-fidelity training and testing.
3. **Baseline for learning**: Use deterministic outputs as a starting point for learned models that can adapt.

But do not claim it is safe, correct, or lawful by construction. Do not claim the binary is the proof. Do not abandon feedback loops or human oversight.

The compilation of reasoning is a neat trick. But a trick is still a trick. The moment you pretend it is magic, it fails.

---

## Signatures

**Joseph Weizenbaum**  
*ELIZA, 1966*  
"I was always more interested in the dialogue than the mechanism. You have inverted that."

**Edward Feigenbaum**  
*MYCIN, 1976*  
"Expert systems work when expertise is transferable. Yours is frozen."

**Richard Fikes**  
*STRIPS, 1971*  
"Plans are not predetermined. They are discovered at execution time."

**Terry Winograd**  
*SHRDLU, 1968–1970*  
"I spent my career learning that microworlds are insufficient for understanding. You have built a faster microworld and called it progress."

**Raj Reddy**  
*Hearsay-II, 1976*  
"Consensus is not the average of independent votes. It is the resolution of systematic disagreement. You have lost that."

---

## Appendix: How Each System Should Actually Work

### ELIZA (Corrected)

- **Runtime**: Accept user input.
- **Feedback loop**: Store user interaction. If user indicates misunderstanding, log it.
- **Retraining**: Monthly, incorporate logged misunderstandings. Retrain intent classifier.
- **Verification**: A/B test new version against old before deployment.
- **Safety**: Always present as advisory. Human decides whether to follow suggestion.

### MYCIN (Corrected)

- **Runtime**: Given symptoms, predict organism + confidence interval.
- **Feedback loop**: Clinician confirms diagnosis. Compare against prediction. Log discrepancies.
- **Retraining**: Quarterly, retrain on new cases. Measure accuracy on held-out test set.
- **Verification**: Require that accuracy remains above 85% on test set. If below, alert.
- **Safety**: Present as "suggested organism (confidence 0.73)." Clinician chooses therapy.

### STRIPS (Corrected)

- **Runtime**: User proposes goal. System generates plan using real-time search (not precomputed).
- **Feedback loop**: Plan execution. If step fails, replan from current state.
- **Adaptation**: Precondition learning. If "PickUp" fails despite CLEAR ∧ ON_TABLE ∧ ARM_EMPTY being true, add new precondition.
- **Verification**: Simulation. Run plan 100 times in stochastic environment. Measure success rate.
- **Safety**: Plan is advisory. Human decides whether to execute. System explains each step.

### SHRDLU (Corrected)

- **Runtime**: User describes command. System extracts features. Learned model predicts feasibility.
- **Feedback loop**: User attempts command. Measure whether it actually succeeded.
- **Retraining**: When predictor is wrong, add example to training set. Retrain monthly.
- **Verification**: Cross-validation. Leave-one-out or k-fold. Measure generalization error.
- **Safety**: Present as "feasible [0.67 confidence]." User decides whether to attempt.

### Hearsay (Corrected)

- **Runtime**: Speech signal arrives. Blackboard runs, produces per-level CFs.
- **Feedback loop**: Compare recognized word against human transcription. Log errors.
- **Retraining**: Monthly, retrain acoustic, phoneme, syllable, word models on new utterances.
- **Verification**: Test suite of speakers from different demographics. Measure WER (word error rate).
- **Safety**: Output is "recognized word: [cat] (confidence 0.63)." Human approves before action.

