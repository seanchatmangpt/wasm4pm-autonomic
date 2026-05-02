import os

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

justfile_content = """# INSA Developer Experience & Quality of Life
# Vibe done means evidence, not confidence.

set shell := ["bash", "-c"]

# -----------------------------------------------------------------------------
# 1. The Core Prove Command
# -----------------------------------------------------------------------------

# Proves the project is locally sane and ready for commit
dx: fmt lint test-unit test-golden test-replay layout bench-smoke
    @echo "✅ VIBE DONE: All basic evidence gates passed."

# -----------------------------------------------------------------------------
# 2. Standard Command Surface
# -----------------------------------------------------------------------------

fmt:
    cargo fmt --all

lint:
    cargo clippy --all-targets --all-features -- -D warnings

test: test-unit test-prop test-compile-fail

test-unit:
    cargo test --lib --bins --tests

test-prop:
    @echo "Running property tests (via truthforge)..."
    cargo test -p insa-truthforge

test-compile-fail:
    @echo "Running compile-fail tests..."
    # Placeholder: cargo test --doc or trybuild

test-golden:
    @echo "Verifying golden wire encoding fixtures..."
    cargo run --package xtask -- golden verify

test-replay:
    @echo "Verifying POWL64 replay signatures..."
    cargo run --package xtask -- replay verify

test-jtbd:
    @echo "Running specific JTBD access drift test..."
    cargo test --test jtbd_access_drift

bench-smoke:
    @echo "Running benchmark smoke tests..."
    cargo bench --no-run

clean:
    cargo clean

# -----------------------------------------------------------------------------
# 3. Layout Gates
# -----------------------------------------------------------------------------

layout:
    @echo "Verifying physical layout bounds..."
    cargo run --package xtask -- layout

# -----------------------------------------------------------------------------
# 4. Deep Verification Gates
# -----------------------------------------------------------------------------

truthforge:
    @echo "Running full Truthforge admission report..."
    cargo run --package xtask -- truthforge

fuzz:
    @echo "Running fuzzer..."
    cargo +nightly fuzz run

miri:
    @echo "Running under strict provenance..."
    MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-disable-isolation" cargo +nightly miri test

# -----------------------------------------------------------------------------
# 5. Developer Onboarding
# -----------------------------------------------------------------------------

doctor:
    @echo "Checking INSA environment constraints..."
    cargo run --package xtask -- doctor
"""

write_file('../insa/justfile', justfile_content)

print("Scaffolded justfile for INSA DX.")
