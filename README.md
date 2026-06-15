# prism

[![CI](https://github.com/systemic-engineering/prism/actions/workflows/ci.yml/badge.svg)](https://github.com/systemic-engineering/prism/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-99.89%25-brightgreen)](https://github.com/systemic-engineering/prism)
[![Dependencies](https://img.shields.io/badge/dependencies-0-blue)](https://github.com/systemic-engineering/prism)
[![unsafe](https://img.shields.io/badge/unsafe-0-green)](https://github.com/systemic-engineering/prism)

Focus | project | settle. A typed transformation pipeline in Rust.

## Crates

### [`terni`](https://crates.io/crates/terni)

`Result` extended with partial success. Three states: Success, Partial (value + measured loss), Failure. The `Loss` trait measures what didn't survive a transformation. Standalone crate, no dependencies on prism. Source: [`imperfect/`](./imperfect/).

### [`prismqueer`](https://crates.io/crates/prismqueer)

Beam (semifunctor) + Prism (monoid). A `Beam` carries a value, the input that produced it, and accumulated loss through a pipeline. A `Prism` defines five operations over beams: focus, project, split, shift, settle. They compose into type-safe pipelines enforced at compile time. Source: [`prismqueer/`](./prismqueer/).

### [`prismqueer-projections`](https://crates.io/crates/prismqueer-projections)

Proc-macros for `prismqueer`: `#[derive(Prism)]`, `#[oid]`, `declaration!`. Source: [`projections/`](./projections/).

## Dependency relationship

```
terni                       (standalone, zero deps)
    ^
    |
prismqueer-projections       (proc-macros, depend on prismqueer types at expansion)
    ^
    |
prismqueer                   (depends on terni, zero external deps)
```

## Example

```rust
use prismqueer::{Beam, Prism, Optic, Focus, Project, Settle};
use terni::Imperfect;

// Seed a beam and run it through a prism
let result = Optic::ok((), "hello".to_string())
    .apply(Focus(&my_prism))
    .apply(Project(&my_prism))
    .apply(Settle(&my_prism));

assert!(result.is_ok());
```

Each `apply` step is type-checked: the output type of one stage must match the input type of the next. Mismatches are compile errors, not runtime panics.

## Tests

435 tests across the workspace:

```
nix develop -c cargo test --workspace --features optics,bundle
```

## License

TBD
