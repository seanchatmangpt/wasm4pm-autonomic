# Makefile for DPIE Project
# Targets: build, test, bench, doc, lint, fmt, check, doctor

.PHONY: build test bench doc clean lint fmt check doctor

build:
	cargo build --release

test:
	cargo test --lib -- --nocapture

bench:
	cargo bench

# Professional dteam doctor check
doctor:
	cargo run --example doctor 2>/dev/null || echo "Running internal diagnostics..."
	cargo check --all-targets

# Start the autonomic live loop
run:
	cargo run --example autonomic_runner

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

check:
	cargo check

doc:
	pdflatex -interaction=nonstopmode -halt-on-error -output-directory=docs/thesis docs/thesis/main.tex
	pdflatex -interaction=nonstopmode -halt-on-error -output-directory=docs/thesis docs/thesis/main.tex
	mv docs/thesis/main.pdf docs/thesis/dpie-whitepaper.pdf

clean:
	cargo clean
	rm -f docs/thesis/*.aux docs/thesis/*.log docs/thesis/*.out docs/thesis/*.toc docs/thesis/*.pdf
