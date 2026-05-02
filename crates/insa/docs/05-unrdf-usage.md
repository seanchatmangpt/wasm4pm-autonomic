# Generalized unrdf Usage — End-to-End Production Pattern

The clean generalization is:
> unrdf = ontology projection compiler

It is not the runtime. It is not the CLI framework. It is not the admission authority.
It is the station that turns semantic source material into repeatable generated artifacts.

`RDF/TTL -> SPARQL -> Nunjucks -> GeneratedArtifact`

The INSA best practice is:
> Do not wrap unrdf. Industrialize its inputs, templates, outputs, and checks.

Use:
- `just unrdf-sync`
- `just unrdf-check`
- `just unrdf-diff`
- `just unrdf-verify`

## The Core Separation
- TTL = semantic source
- SPARQL = selection law
- Nunjucks = projection law

SPARQL selects. Nunjucks projects.

## What unrdf Should Generate
Good targets: Rust IRI constants, term catalogs, failure-code catalogs, doctor check catalogs, Zod schemas, OpenAPI specs, MCP descriptors.
Avoid: COG8 hot evaluator, INST8 bit operations, POWL8 route logic, WireV1 encoder internals.

Ontology can project semantic artifacts. Machine law must be handcrafted, gated, and admitted.
