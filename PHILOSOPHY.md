# Philosophy: Civilization-First Licensing for Compiled Cognition

## The Blue River Dam Pattern

In the 1950s, the U.S. Corps of Engineers dammed the Blue River in Colorado. The dam was built to provide flood control, water storage, and power generation—public goods that benefited civilization.

Then Congress privatized the dam's management. The agency that operated it could now extract rent from water users, farmers, and cities downriver. A public good became a private tollbooth.

**This happens in technology**, and Compiled Cognition must prevent it.

---

## Why Permissive Licensing Enables Platform Capture

Suppose dteam is released under MIT or Apache 2.0 (fully permissive, no restrictions).

A large AI infrastructure company (call it "Platform X") could:

1. **Fork dteam**
2. **Wrap it in a managed cloud service** ("Compiled Cognition Cloud")
3. **Charge per decision, per deployment, or per audit**

Platform X has now **recreated the exact dependency tax that Compiled Cognition was designed to eliminate**.

The math:
- Before: organizations had to use an external LLM API (rent-seeking)
- After: organizations thought they had escaped dependency rent
- But: now they pay Platform X instead (rent still extracted, just rebranded)

**Platform X did nothing wrong under a permissive license.** But they violated the *spirit* of the invention: freedom from dependency.

---

## The Three-Layer Solution

Compiled Cognition uses a **three-layer release model** to prevent this:

### Layer 1: Public Theory
The thesis, equations, principles, and arguments are published freely.

- `docs/COMPILED_COGNITION.md`
- `docs/thesis/thesis.md`
- `PHILOSOPHY.md` (this document)
- Academic papers, talks, teaching materials

**Goal**: Civilization learns the pattern. If anyone reinvents it independently, good.

### Layer 2: Source-Available Implementation
The reference implementation is readable, inspectable, and usable under BUSL 1.1.

- **Permitted**: Study, research, education, personal use, small organization production
- **Restricted**: Platform capture, commercial embedding, dependency-rent recreation
- **Mechanism**: Business Source License with a Change Date

**Goal**: Developers can inspect, learn, and experiment. Megacorps cannot immediately enclose it.

### Layer 3: Commercial Sustainability
Large commercial users can license the technology under mission-aligned terms.

- **Who**: Organizations >$1M revenue, or those embedding in commercial products
- **What**: Production support, certification, audit readiness, patent coverage
- **Mission Covenant**: You may use this to remove dams, not to build a larger dam downstream

**Goal**: The creators can sustain development. Commercial users can build on dteam. But neither recreates dependency rent.

---

## Governing Equation

The license implements a governance equation:

```
License_CC = Read ∧ Learn ∧ Experiment ∧ NonExtractiveUse ∧ DelayedCommons ∧ ¬PlatformCapture
```

Parsed:

- **Read**: The source is available. You can inspect it.
- **Learn**: You can study it, teach from it, understand the pattern.
- **Experiment**: You can use it for research, prototyping, internal projects.
- **NonExtractiveUse**: You can use it as long as you're not recreating dependency rent.
- **DelayedCommons**: On April 18, 2029, it becomes Apache 2.0. The promise is written into law.
- **¬PlatformCapture**: You cannot use it to build a platform that extracts rent from the same dependency Compiled Cognition removes.

---

## Why April 18, 2029?

The Change Date is not arbitrary.

- **4 years** is long enough for the creators to build a sustainable business
- **4 years** is short enough that no megacorp can lock in customers on a dependency
- **April 18, 2029** is a specific date, locked in the LICENSE file. It's a promise written into code.

When that date arrives, dteam becomes Apache 2.0. Permissive. Open. Commons.

**There is no ambiguity, no fine print, no "we might extend it." On April 18, 2029, it's open source.**

---

## Contrast with Other Models

### Permissive OSS (MIT, Apache 2.0)
- **Pros**: Maximum reuse, immediate adoption, no friction
- **Cons**: Enables immediate platform capture; the Blue River Dam pattern succeeds

### Proprietary (Closed Source)
- **Pros**: Creator controls the business model
- **Cons**: Civilization cannot inspect it; adoption is bottlenecked; theory spreads slowly

### BUSL with Civilization-First Grant
- **Pros**: Theory spreads (Layer 1); adoption happens (Layer 2); capture prevented (Layer 2 restriction); commons guaranteed (Layer 3 timing); sustainability (Layer 3 licensing)
- **Cons**: Requires clarity, enforcement, and trust in the Change Date

---

## Why BUSL, Not Other Approaches?

Other restricted licenses exist:

- **Elastic License**: Restricts use by competitors; too narrow
- **Server Side Public License (SSPL)**: Requires source release of entire application; too broad and legally uncertain
- **Commons Clause**: Adds a commercial-use restriction to OSS; but doesn't solve the Blue River problem (you can't prevent Platform X from licensing separately)

**BUSL is ideal because:**

1. It explicitly permits non-commercial, research, and small-org use
2. The Additional Use Grant can articulate the civilization-first principle
3. The Change Date is a hard commitment to openness
4. It's legally established (used by MariaDB, MongoDB post-revert, Stripe, etc.)
5. It clearly communicates: "today source-available, later Apache 2.0"

---

## The Angelic AI Connection

Compiled Cognition is a form of **Angelic AI**: bounded, lawful, non-sovereign intelligence.

The license enforces this socially:

- **Bounded**: Restricted use cases prevent a system from becoming a universal platform
- **Lawful**: The Additional Use Grant defines what uses are permitted, what forbidden
- **Non-Sovereign**: No single entity can impose arbitrary changes; the Change Date is law

**The license prevents Angelic AI from becoming OracleAI** (a new dependency) **or SovereignAI** (an unrestricted platform).

---

## Enterprise Trust

Large organizations worry: "Will this license change? Will the creators go proprietary later?"

The answer is in the LICENSE file, legally binding:

> "On April 18, 2029, this License will automatically terminate and the Licensed Work will thereafter be licensed under the Change License: Apache License 2.0."

This is not a promise. It's a legal obligation. Changing it would breach the BUSL 1.1 specification.

Enterprise users can:
- Fork before April 18, 2029, and keep their fork under BUSL indefinitely
- Wait until April 18, 2029, and get Apache 2.0 automatically
- License commercially today and keep their license indefinitely

---

## Principles Summary

1. **Release the source, not the extraction rights**
2. **Civilization deserves to learn the pattern**
3. **Small organizations deserve to use it freely**
4. **Large commercial users can license it affordably**
5. **The eventual destination is the commons**
6. **The path is BUSL with a fixed Change Date, not forever proprietary**

---

## See Also

- `LICENSE` — The legal text (BUSL 1.1)
- `USE-GRANT.md` — Plain English: what you can/cannot do
- `COMMERCIAL.md` — Commercial licensing terms
- `PATENTS.md` — Patent strategy and no-offensive-use covenant
- `CONTRIBUTING.md` — How to contribute
- `docs/COMPILED_COGNITION.md` — The technical theory

