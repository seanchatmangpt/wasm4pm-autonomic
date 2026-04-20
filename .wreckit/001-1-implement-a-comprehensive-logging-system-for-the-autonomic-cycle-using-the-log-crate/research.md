I will begin by checking if the `log` crate is already a dependency in `Cargo.toml` and then I'll search for existing logging patterns and the autonomic cycle implementation in `src/autonomic/`.
I'll examine the autonomic cycle implementation in `src/autonomic/kernel.rs` and `src/autonomic/vision_2030_kernel.rs`, and check `src/autonomic/types.rs` for relevant data structures and any existing logging mechanisms.
I've identified the `AutonomicKernel` trait and its cycle steps, noting the absence of `log` crate usage. To implement a comprehensive logging system, I'll add the `log` crate, initialize it, and insert appropriate logging macros throughout the `AutonomicKernel` implementations and `run_cycle` helper. I'll now examine `examples/autonomic_runner.rs` to see how `run_cycle` is currently utilized.
`autonomic_runner.rs` relies on `println!`, which I'll now replace or supplement with `log` macros, while also reviewing `src/lib.rs` to understand the overall exposure.
I've gathered a comprehensive understanding of the project and autonomic cycle, and I'm ready to draft the `research.md` report. It will detail the current lack of a logging system, propose specific integration points and strategies across different logging levels and contexts, and outline implementation steps while addressing potential performance and WASM compatibility challenges. I'll now proceed with writing the `research.md`.
I'll create the `research.md` report as `plans/research-logging.md`, detailing existing patterns, file paths, and integration points for the comprehensive logging system as requested.
I'm in Plan Mode and must save plans as .md files in `/Users/sac/.gemini/tmp/dteam/2c722ab8-b44c-49d6-ae84-c36597fabb55/plans/`. Per the prompt, I'll now write a detailed `research.md` report to that directory, analyzing existing patterns, file paths, and integration points for a comprehensive logging system using the 'log' crate.
I have completed the research on implementing a comprehensive logging system for the autonomic cycle using the `log` crate. The findings, including existing patterns, key files, proposed integration points, and an implementation strategy, are detailed in the `research.md` report below.

# Research Report: Comprehensive Logging System for Autonomic Cycle

## 1. Objective
Implement a robust and comprehensive logging system for the autonomic cycle in the `dteam` project using the `log` crate. This will replace or augment current `println!` usage and provide better observability into the "Digital Team" process intelligence engine.

## 2. Current State
- **Logging Infrastructure:** The `log` crate is not currently a dependency in `Cargo.toml`.
- **Existing Patterns:**
    - High-level orchestration in `examples/autonomic_runner.rs` uses `println!`.
    - Internal logic in `src/autonomic/kernel.rs` and `src/autonomic/vision_2030_kernel.rs` lacks systematic logging.
    - Some methods return `String` (e.g., `manifest`) to communicate results, but don't log internal state transitions.
- **Autonomic Cycle Structure:** Defined by the `AutonomicKernel` trait in `src/autonomic/kernel.rs`:
    1. `observe(event)`
    2. `infer() -> AutonomicState`
    3. `propose(state) -> Vec<AutonomicAction>`
    4. `accept(action, state) -> bool`
    5. `execute(action) -> AutonomicResult`
    6. `manifest(result) -> String`
    7. `adapt(feedback)`

## 3. Key Files & Context
- `Cargo.toml`: Add `log` (and a facade like `env_logger` for tests/examples).
- `src/autonomic/kernel.rs`: Main trait and `DefaultKernel` implementation.
- `src/autonomic/vision_2030_kernel.rs`: Advanced implementation with OCPM 2.0 and contextual bandits.
- `src/autonomic/types.rs`: Data structures (`AutonomicEvent`, `AutonomicState`, `AutonomicAction`, `AutonomicResult`, `AutonomicFeedback`).
- `examples/autonomic_runner.rs`: Entry point for simulation.
- `src/lib.rs`: Re-exports and core types.

## 4. Proposed Integration Points

### 4.1. `AutonomicKernel::run_cycle`
This helper orchestrates the entire cycle. It should log high-level phase transitions:
- **INFO:** Starting autonomic cycle for event from `{source}`.
- **DEBUG:** Inferred state: `{state}`.
- **INFO:** Proposed `{n}` actions.
- **WARN:** Action `{id}` rejected due to `{reason}`.
- **INFO:** Action `{id}` accepted and executed: `{result}`.
- **INFO:** Cycle complete. Manifest hash: `{hash:X}`.

### 4.2. `observe`
Log incoming payloads and internal parsing:
- **DEBUG:** Observing payload: `{payload}`.
- **DEBUG:** OCPM 2.0 binding frequencies updated.

### 4.3. `infer`
Log state metrics:
- **DEBUG:** Current health: `{health}`, throughput: `{throughput}`, conformance: `{conformance}`.

### 4.4. `propose`
Log decision-making logic:
- **DEBUG:** Bandit context extracted. Selected action index: `{idx}`.
- **DEBUG:** MCTS UCT scores: Repair=`{repair}`, Opt=`{opt}`.
- **INFO:** Proposing action `{id}` (`{type}`) for params: `{params}`.

### 4.5. `accept`
Log safety/soundness checks:
- **DEBUG:** Simulating action `{id}`. Expected reward: `{reward}`.
- **INFO:** Action `{id}` accepted (Risk `{risk}` <= Threshold `{threshold}`).
- **WARN:** Action `{id}` rejected (Risk `{risk}` > Threshold `{threshold}`).
- **WARN:** Action `{id}` rejected (Soundness guard violation).

### 4.6. `execute`
Log state mutations and execution metrics:
- **DEBUG:** Executing action `{id}`. Latency: `{latency}ms`.
- **INFO:** State repair acknowledged. Conformance improved by `{delta}`.

### 4.7. `adapt`
Log feedback and learning:
- **DEBUG:** Adapting bandit with reward: `{reward}`.
- **DEBUG:** State health updated: `{old}` -> `{new}`.

## 5. Implementation Strategy
1. **Dependency Management:** Add `log = "0.4"` to `[dependencies]` in `Cargo.toml`. Add `env_logger = "0.11"` to `[dev-dependencies]`.
2. **Global Initialization:** Initialize `env_logger` in `examples/autonomic_runner.rs` and in tests.
3. **Instrumentation:**
    - Add `use log::{info, debug, warn, error};` to `src/autonomic/kernel.rs` and `src/autonomic/vision_2030_kernel.rs`.
    - Insert appropriate macros based on the integration points in Section 4.
4. **Refactoring Examples:** Replace `println!` with `info!` where appropriate, or keep `println!` for user-facing output but augment with internal `debug!` logs.

## 6. Challenges & Considerations
- **Performance:** Ensure logging does not degrade the "zero-heap hot path" performance. Use `if log_enabled!(Level::Debug)` for expensive payload formatting if necessary.
- **WASM Support:** The `log` crate is compatible with WASM. The facade (e.g., `web_sys` logger) should be used on the JS side, while the Rust code just uses the macros.
- **Log Correlation:** Use `action_id` or a new `trace_id` to correlate logs across a single `run_cycle`.

## 7. Next Steps
1. Add `log` crate dependency.
2. Update `DefaultKernel` with basic logging.
3. Update `Vision2030Kernel` with detailed logging for bandit/OCPM logic.
4. Update `examples/autonomic_runner.rs` to initialize a logger and use log macros.
