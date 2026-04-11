# prism

[![CI](https://github.com/systemic-engineering/prism/actions/workflows/ci.yml/badge.svg)](https://github.com/systemic-engineering/prism/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-99.89%25-brightgreen)](https://github.com/systemic-engineering/prism)
[![Dependencies](https://img.shields.io/badge/dependencies-0-blue)](https://github.com/systemic-engineering/prism)
[![unsafe](https://img.shields.io/badge/unsafe-0-green)](https://github.com/systemic-engineering/prism)

Focus | project | refract. A typed transformation pipeline in Rust.

## Crates

### `imperfect`

`Result` extended with partial success. Three states: Success, Partial (value + measured loss), Failure. The `Loss` trait measures what didn't survive a transformation; `ShannonLoss` is the default implementation, measuring information loss in bits. Standalone crate, no dependencies on prism.

### `prism-core`

Beam (semifunctor) + Prism (monoid). A `Beam` carries a value, the input that produced it, and accumulated loss through a pipeline. A `Prism` defines three operations over beams: focus (select), project (transform), refract (produce output). The three operations compose into type-safe pipelines enforced at compile time.

## Dependency relationship

```
imperfect          (standalone, zero dependencies)
    ^
    |
prism-core         (depends on imperfect, zero external dependencies)
```

## Example

```rust
use prism_core::{Beam, Prism, PureBeam, Focus, Project, Refract};
use imperfect::Imperfect;

// Seed a beam and run it through a prism
let result = PureBeam::ok((), "hello".to_string())
    .apply(Focus(&my_prism))
    .apply(Project(&my_prism))
    .apply(Refract(&my_prism));

assert!(result.is_ok());
```

Each `apply` step is type-checked: the output type of one stage must match the input type of the next. Mismatches are compile errors, not runtime panics.

## Tests

220 tests across the workspace:

```
nix develop -c cargo test --workspace --features optics
```

## License

TBD
