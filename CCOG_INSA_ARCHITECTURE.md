# CCOG & INSA Architecture

## Overview

The DTEAM project operates within the **INSA (Instinctual Autonomics)** architecture. At the heart of this system lies the core equation: `A = µ(O*)`.
*   `O`: Raw observation. (LLMs generate from this, but this system does NOT).
*   `O*`: Closed, typed, policy-valid field context.
*   `µ`: Lawful transition function.
*   `A`: Admitted action / proof.

The overarching directive is that the system must never generate action or state mutations from unclosed fields.

Within this framework, the two central components are **CCOG (Compiled Cognition)** and **INSA (Instinctual Autonomics)**.

---

## CCOG: Compiled Cognition

**CCOG** serves as the Compiled Cognition core, functioning as a field-cognition facade over an RDF graph closure. Its primary responsibility is to know what the graph permits the operational field to do.

The core formula within CCOG is: `U → O*_U → C_U → A_U → R_U`
*   **`U`**: Bounded operational field.
*   **`O*_U`**: Semantic closure of that field (derived from the RDF graph).
*   **`C_U`**: Compiled cognition artifact (cognitive pass).
*   **`A_U`**: Admissible operations.
*   **`R_U`**: PROV receipt (proof + provenance).

CCOG's MVP implementation handles phrase binding, missing evidence detection, blocked transitions, and eventually yields admissible operations backed by PROV receipts.

### Cognitive Breeds

CCOG relies on multiple "Cognitive Breed" passes to process fields. These breeds are modeled after historical AI architectures, applying specific reasoning paradigms to the bounded fields:
*   **ELIZA**: Pattern matching and basic reflective responses.
*   **MYCIN**: Rule-based certainty and diagnostics.
*   **STRIPS**: Precondition/effect based planning.
*   **SHRDLU**: Grounding and spatial/state reasoning.
*   **Prolog**: Logic programming and unification.
*   **Hearsay-II**: Blackboard architecture and fusion of knowledge sources.
*   **DENDRAL**: Hypothesis generation and specialized expert chain reasoning.
*   *Phase-9 expansions* include GPS (General Problem Solver), SOAR (Cognitive Architecture), PRS (Procedural Reasoning System), and CBR (Case-Based Reasoning).

---

## INSA: Instinctual Autonomics

**INSA** is a multi-crate workspace that defines the execution environment, types, hot-path semantics, and cold-path evidence required by the architecture. It enforces "Vibe Done" (Evidentiary done) over "Vibe coding". Code is only considered complete when it passes strict layout offsets, cross-platform wire encoding checks, and Truthforge admission gates.

The INSA architecture utilizes **Byte-Width Semantic Multiplexing**, meaning `u8` is the semantic lane holding a power set of states and instructions.

The `insa` workspace is divided into several specialized crates:

### 1. `insa-types`
Provides `no_std` fundamental data types and domain concepts:
*   **Domain**: `DictionaryDigest`, `ObjectRef`, `PolicyEpoch`.
*   **Identifiers**: `BreedId`, `EdgeId`, `GroupId`, `NodeId`, `PackId`, `RouteId`, `RuleId`.
*   **Masks**: `CompletedMask`, `FieldBit`, `FieldMask`.

### 2. `insa-hotpath`
The reference "Law Path" for INSA execution. It acts as the semantic oracle for the immediate, high-performance execution of constraints and instincts.
*   Contains core resolution logic: `cog8`, `construct8`, `powl8` (Packaged Web Ontology Language via `u8`), and generic `resolution`.
*   Operates using LUTs (Look-Up Tables) and SIMD/scalar fallback implementations to quickly determine if an action is admitted or blocked.

### 3. `insa-proof`
The cold-path evidence and replay layer. While the hot-path makes rapid decisions, the cold-path guarantees verifiable provenance and cryptographic receipts.
*   Handles `powl64` (the 64-bit expanded ontology reasoning).
*   Manages `receipt` structures and cross-platform `wire` encodings.

### 4. `insa-truthforge`
The comprehensive verification harness for INSA. It acts as the central gatekeeper.
*   Contains property tests, compile-fail assertions, layout gates, and mutation tests.
*   Guarantees the layout and semantic invariants of the entire INSA ecosystem.
*   Handles `admission`, ensuring that only operations passing the strict Proof of Work/Logic are admitted into the system.

### 5. `insa-instinct` / `insa-kappa8` / `insa-security`
*   **`insa-instinct`**: Implements the base byte-level (`u8`) resolution and instinctual reactions based on the current field mask.
*   **`insa-kappa8`**: Specific engine implementations matching the cognitive breeds (e.g., `precondition_strips`, `prove_prolog`, `rule_mycin`, `reflect_eliza`, `fuse_hearsay`). It resolves the 8-bit cognitive logic.
*   **`insa-security`**: Enforces security constraints and validation across the operational bounds.

---

## Interaction Summary

1.  **Field Definition**: The operational field `U` is defined and passed to CCOG.
2.  **Closure (`O*_U`)**: CCOG resolves the semantic closure of the field using the RDF graph, determining what facts and limits apply.
3.  **Cognitive Pass (`C_U`)**: `insa-kappa8` and CCOG's breeds evaluate the closure using specific reasoning engines (STRIPS, Prolog, MYCIN, etc.).
4.  **Hot-Path Resolution**: `insa-hotpath` applies byte-width (`u8`) rules (`cog8`, `powl8`) to determine immediately if the state transition is valid (`µ`).
5.  **Admittance & Proof**: If valid, the action `A_U` is admitted. `insa-proof` generates a PROV receipt `R_U` (`powl64`), ensuring the operation is independently verifiable and cryptographically sound.
6.  **Gatekeeping**: `insa-truthforge` tests and asserts continuously that this entire loop remains sound, deterministic, and bound to byte-law.