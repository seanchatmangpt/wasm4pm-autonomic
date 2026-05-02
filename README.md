# dteam 

**Deterministic Process Intelligence Engine (dteam) - Digital Team**

Compiled Cognition for enterprise decision pipelines.

Every model in this repository is designed as a strict `const`: trained once at build time, embedded in the binary, and evaluated in nanoseconds at runtime. We integrate Classical symbolic AI (ELIZA, MYCIN, STRIPS, SHRDLU, Hearsay-II, Prolog, Dendral, GPS) alongside verified learned equivalents. 

## Architecture: INSA
The governing equation is `A = μ(O*)`: 
* **O***: Closed, typed, policy-valid field context.
* **μ**: Lawful transition function.
* **A**: Admitted action / proof.

Action is lawful *only* when projected from a semantically closed, compiled output set.

## Workspace Layout
* `crates/insa/*`: The Instinctual Autonomics (INSA) primitives, executing kernel (`hotpath`), classical engines (`kappa8`), and verification gates (`truthforge`).
* `crates/ccog`: The Compiled Cognition library and facade.
* `crates/ccog-bridge`: The dependency translation layer.
* `crates/autoinstinct`: Trace compiler CLI tooling.
