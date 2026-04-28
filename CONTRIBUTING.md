# Contributing to Compiled Cognition

We welcome contributions from the community. This document explains how to contribute and the governance framework for code ownership.

---

## How to Contribute

Contributions are welcome in these areas:

1. **Bug fixes** — Issues reported in the repo or discovered by you
2. **Tests** — New test cases, improved coverage, edge case testing
3. **Documentation** — Clarity improvements, examples, diagrams
4. **Performance** — Benchmarks, optimization, latency improvements
5. **New AI systems** — Additional symbolic or learned models paired with AutoML equivalents
6. **Dependency updates** — Keeping Cargo.toml and lockfile current

### What Requires Discussion First

Before starting work, **open an issue** to discuss:

1. **New AI systems** — Any new classical or learned model should be discussed first
2. **API changes** — Changes to public interfaces need design discussion
3. **License-affecting changes** — Any change to licensing or IP strategy
4. **Breaking changes** — Anything that breaks backward compatibility
5. **Large refactors** — Major code reorganization should be scoped in advance

For small bug fixes and tests, you can just open a PR.

---

## Developer Certificate of Origin (DCO)

We use **Developer Certificate of Origin (DCO)** instead of a Contributor License Agreement (CLA).

**What this means:**

You certify that:
- You wrote the code you're submitting (or have permission to submit it)
- The contribution is original and doesn't violate anyone else's IP
- You're granting us the right to use, modify, and distribute your contribution

**How to sign off:**

Add `-s` to your git commit:

```bash
git commit -s -m "Fix latency bug in branchless token replay"
```

This adds a `Signed-off-by: Your Name <email@example.com>` line to your commit message.

**Why DCO not CLA?**

- **Simpler**: No lawyers, no corporate agreements, just a line in your commit
- **Lighter weight**: You retain copyright in your contributions
- **More permissive**: Anyone (individual or corporate) can contribute
- **Standard**: Used by Linux, Docker, Kubernetes, and many open-source projects

---

## License Terms for Contributors

When you contribute to dteam:

- Your contribution is licensed under **BUSL-1.1** until April 18, 2029
- After April 18, 2029, your contribution automatically converts to **Apache License 2.0**
- You retain copyright in your contribution
- We have the right to sublicense your contribution under commercial licenses (with compensation per contract negotiation)

By submitting a PR, you agree to these terms.

---

## Code Style & Quality

- **Rust edition**: 2021 edition only
- **Linting**: `cargo make lint` must pass with no errors
- **Formatting**: `cargo make fmt` must pass
- **Testing**: All new code must have tests; `cargo test --lib` must pass
- **Benchmarking**: Performance-sensitive code should include `benches/*` benchmarks
- **Documentation**: Public APIs must have doc comments; examples are encouraged

Run the full CI locally before pushing:

```bash
cargo make ci
```

---

## Code Organization

### File Structure

- `src/` — Core library code
  - `src/lib.rs` — Crate root
  - `src/ml/` — Machine learning systems (ELIZA, MYCIN, STRIPS, SHRDLU, Hearsay + AutoML)
  - `src/conformance/` — Process conformance and token-based replay
  - `src/io/` — Input/output, logging, observability
  - `src/utils/` — Utility functions (hashing, bit operations, etc.)
  - `src/agentic/` — Agent-based orchestration
- `tests/` — Integration tests
- `benches/` — Performance benchmarks
- `docs/` — Documentation and thesis

### Adding a New System

If you're adding a new classical AI system or AutoML equivalent:

1. Create `src/ml/newsystem.rs` (symbolic) and `src/ml/newsystem_automl.rs` (learned)
2. Add both to `src/ml/mod.rs`
3. Add tests to each module (≥90% coverage)
4. Add a doctest showing usage
5. Add an entry to `docs/ML_INVENTORY.md`
6. Add the system to the ensemble in `src/ml/automl_config.rs`
7. Update `PLAYGROUND_README.md` if it changes user-facing features

---

## Pull Request Process

1. **Fork** the repo
2. **Create a branch** off `main`
3. **Make changes** and commit locally with `git commit -s`
4. **Push** to your fork
5. **Open a PR** with:
   - **Title**: Descriptive, short (max 70 chars)
   - **Description**: Explains what and why (not just what)
   - **Issue**: References issue number if applicable
   - **Testing**: Describes test coverage
6. **Wait for CI** — All checks must pass
7. **Respond to feedback** — We may request changes
8. **Merge** — Maintainers merge when ready

---

## Commit Message Convention

Use **conventional commits**:

```
type(scope): brief description

Optional longer explanation here.
```

Types:

- `feat:` — New feature
- `fix:` — Bug fix
- `test:` — New test or test improvement
- `docs:` — Documentation
- `refactor:` — Code reorganization without behavior change
- `perf:` — Performance improvement
- `ci:` — CI/CD changes

Examples:

```
feat(ml): add DQN learner to reinforcement module

fix(conformance): resolve off-by-one error in token replay
test(utils): add property tests for branchless operations

```

---

## Testing Requirements

### Unit Tests

All new code must have unit tests. Target ≥90% coverage:

```bash
cargo tarpaulin --out Html --timeout 300
# Open tarpaulin-report.html
```

### Integration Tests

If your change affects end-to-end behavior, add integration tests in `tests/`.

### Benchmarks

If your change affects latency-critical paths:

```bash
cargo bench --bench hot_path_performance_bench
```

If performance regresses, your PR will be asked to optimize.

---

## Documentation Requirements

### Public APIs

All public functions, structs, and traits must have doc comments:

```rust
/// Trains a logistic regression model on the provided features and labels.
///
/// Returns a binary classifier with the learned weights.
///
/// # Example
/// ```
/// let features = vec![vec![0.5, 0.3], vec![0.1, 0.9]];
/// let labels = vec![true, false];
/// let model = train_logistic_regression(&features, &labels);
/// assert_eq!(model.predict(&[0.5, 0.3]), true);
/// ```
pub fn train_logistic_regression(features: &[Vec<f64>], labels: &[bool]) -> LogisticModel {
    // ...
}
```

### Module-Level Documentation

Modules should have a top-level doc comment explaining the module's purpose:

```rust
//! Token-based conformance checking for Petri nets.
//!
//! This module implements branchless u64 bitmask token replay for nets with ≤64 places.
//! For larger nets, see `replay_trace_standard`.
```

---

## Governance & Maintainers

**Current maintainers:**
- Sean Chatman (xpointsh@gmail.com) — Architecture, theory, release decisions

**Decision process:**

- **Bug fixes**: Merged by any maintainer
- **New features**: Discussed in issues; consensus among maintainers
- **API changes**: Discussed in design proposals; approval from Sean
- **License/IP changes**: Require Sean's explicit approval (governance-affecting)

---

## Code of Conduct

All contributors must follow our values:

1. **Respect each other** — Treat others with kindness and professionalism
2. **Assume good intent** — If something seems off, ask before accusing
3. **Focus on ideas** — Critique code and ideas, not people
4. **Embrace diversity** — Welcome contributors of all backgrounds
5. **Serve the mission** — Contributions should serve civilization benefit, not personal gain

Bad behavior (harassment, discrimination, hostility) will result in removal from the project.

---

## Licensing & Ownership

**Your contributions:**
- License: BUSL-1.1 until April 18, 2029; then Apache 2.0
- Ownership: You retain copyright; dteam retains non-exclusive rights to sublicense
- Commercial: Contributions may be included in commercial licenses; you can discuss compensation separately

**Third-party code:**
- Must be compatible with BUSL-1.1 (Apache 2.0, MIT, BSD, etc.)
- Must include license attribution
- Vendor separately in `vendors/` with license file

---

## Getting Help

**Questions?**
- **Chat**: Issues are the best place to ask
- **Email**: xpointsh@gmail.com for governance questions
- **Docs**: `PHILOSOPHY.md` explains the vision; `AGENTS.md` explains the architecture

---

## Credits

Thank you for contributing to Compiled Cognition. Your work helps civilization benefit from this technology.

