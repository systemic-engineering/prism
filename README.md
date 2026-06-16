# prism

[![CI](https://github.com/systemic-engineering/prism/actions/workflows/ci.yml/badge.svg)](https://github.com/systemic-engineering/prism/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-99.89%25-brightgreen)](https://github.com/systemic-engineering/prism)
[![Dependencies](https://img.shields.io/badge/dependencies-0-blue)](https://github.com/systemic-engineering/prism)
[![unsafe](https://img.shields.io/badge/unsafe-0-green)](https://github.com/systemic-engineering/prism)

A typed transformation pipeline in Rust. Carry a value, the input that
produced it, and the loss accumulated along the way — through a chain
that the compiler checks end-to-end.

This workspace ships three crates to crates.io: [`terni`](#terni),
[`prismqueer`](#prismqueer), and [`prismqueer-projections`](#prismqueer-projections).

## Crates

### [`terni`](https://crates.io/crates/terni)

`Result` extended with partial success. Three states: `Success`,
`Partial(value, loss)`, `Failure`. The `Loss` trait measures what didn't
survive a transformation. Standalone, zero deps on the rest of the
workspace. Source: [`imperfect/`](./imperfect/).

### [`prismqueer`](https://crates.io/crates/prismqueer)

The `Prism` trait, the `Beam` carrier, and the `Optic` concrete beam.
A `Beam` is a semifunctor over `Imperfect`; a `Prism` is three
staged transformations (`focus`, `project`, `settle`) over beams. They
compose into pipelines whose type-shape is checked at compile time.
Source: [`prismqueer/`](./prismqueer/).

### [`prismqueer-projections`](https://crates.io/crates/prismqueer-projections)

Proc-macros for `prismqueer`: `#[derive(Prism)]`, `#[derive(Lambda)]`,
`declaration!{}`. Source: [`projections/`](./projections/).

## Dependency direction

```
terni                       (standalone, zero deps)
    ^
    |
prismqueer-projections      (proc-macros; reference prismqueer paths in their output)
    ^
    |
prismqueer                  (depends on terni + prismqueer-projections)
```

## Example

```rust
use prismqueer::{apply, Beam, Optic, Prism};

// A trivial prism: focus splits the input into chars,
// project counts them, settle renders the count.
struct CountPrism;

impl Prism for CountPrism {
    type Input     = Optic<(), String>;
    type Focused   = Optic<String, Vec<char>>;
    type Projected = Optic<Vec<char>, usize>;
    type Refracted = Optic<usize, String>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let s = beam.result().ok().unwrap().clone();
        beam.next(s.chars().collect())
    }
    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let n = beam.result().ok().unwrap().len();
        beam.next(n)
    }
    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        let n = *beam.result().ok().unwrap();
        beam.next(format!("{n} chars"))
    }
}

let result = apply(&CountPrism, Optic::ok((), "hello".to_string()));
assert_eq!(result.result().ok(), Some(&"5 chars".to_string()));
```

Each stage's output type feeds the next stage's input type. A type
mismatch between stages is a compile error, not a runtime panic.

## Tests

Workspace tests across all features:

```
cargo test --workspace --all-features
```

## License

Apache-2.0 across the workspace. See each crate's `LICENSE`.
