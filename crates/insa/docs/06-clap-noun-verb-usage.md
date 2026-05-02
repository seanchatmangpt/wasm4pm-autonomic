# Generalized clap-noun-verb Usage — End-to-End Pattern

The clean generalization is:
> clap-noun-verb = operational grammar compiler

It gives INSA the command grammar; INSA supplies the admitted law.

## The Core Separation
- **CLI**: Parse, validate, route, serialize output
- **Integration**: Filesystem, network, process, terminal, config
- **Domain**: Actual law and computation

No INSA law inside CLI wrapper functions.

## Noun and Verb Design Rules
Noun = responsibility boundary.
Verb = admissible motion.

Good nouns: `doctor`, `wizard`, `telco`, `replay`, `pack`, `bench`, `release`
Good verbs: `check`, `explain`, `report`, `verify`, `plan`, `apply`, `provision`, `test`, `trace`

## Structured Output and Agent-Grade CLI
Every command emits machine-readable output.
Every status maps to a stable exit code.
No hidden interactive prompts in CI paths.
No mutation without receipt.
