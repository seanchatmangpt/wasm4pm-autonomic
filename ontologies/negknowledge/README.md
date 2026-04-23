# negknowledge — ontology-driven codegen demo

This directory demonstrates the full `@unrdf/cli` pipeline: an
ontology expressed as Turtle, a SPARQL query that extracts rows, a
Nunjucks template that emits Rust, and a checked-in generated artifact
that a downstream crate can consume as-is.

The artifact generated here is a Rust constant array matching the
`unibit_negknowledge::NegativeResult` shape:

```rust
pub const NEGATIVE_KNOWLEDGE_FROM_ONTOLOGY: &[NegativeResult] = &[ /* … */ ];
```

It is **not** wired into the `unibit-negknowledge` crate's build. The
canonical in-tree registry lives at `crates/unibit-negknowledge/src/lib.rs`
and is hand-authored. This scaffold proves that an ontology-driven
registry with the same shape is viable; a future production wiring
would switch from hand-authored to generated, or merge the two tables.

## Layout

```
ontologies/negknowledge/
├── unrdf.toml                         — codegen rule (query + template → output)
├── ontology/
│   └── negknowledge.ttl               — 4 NegativeResult entries
├── sparql/
│   └── negknowledge.rq                — SELECT id/attempt/source/outcome/reason
├── templates/
│   └── negknowledge_table.njk         — Rust const array emitter
├── generated/
│   └── negknowledge_from_ontology.rs  — checked-in expected output
└── README.md
```

## Regenerating

If `@unrdf/cli` is installed globally (`pnpm add -g @unrdf/cli`, or
equivalent npm/yarn), regeneration is:

```sh
cd ontologies/negknowledge
unrdf sync
```

The `unrdf.toml` points at `ontology/negknowledge.ttl`, runs the
SPARQL query in `sparql/negknowledge.rq`, applies the Nunjucks
template in `templates/negknowledge_table.njk`, and overwrites
`generated/negknowledge_from_ontology.rs`.

If `@unrdf/cli` is not installed, the checked-in
`generated/negknowledge_from_ontology.rs` file is the expected output
and can be consumed directly — its content is an exact projection of
the ontology at authoring time.

## Why this matters

The canonical `unibit-negknowledge` registry preserves what the
architecture tried and rejected. Expressing that knowledge as an
ontology gives the project three additional properties:

1. **Governed** — the ontology can be validated, linted, and diffed
   across versions using the standard `@unrdf/cli` lifecycle.
2. **Queryable** — SPARQL queries can slice the registry by outcome
   (all `Pessimisation` vs all `Rejected`), by source doc, by date
   (future), or by any other predicate added later.
3. **Reprojectable** — the same ontology can emit more than one
   artifact: Rust const array today, TypeScript enum tomorrow,
   documentation markdown after that.

The Rust-core-team view from docs 55/58/60: ontology is *input to the
compiler*, not *data for the hot kernel*. This scaffold keeps that
invariant — the ontology is consumed at build time, the output is a
static Rust constant, and the hot path never traverses RDF.
