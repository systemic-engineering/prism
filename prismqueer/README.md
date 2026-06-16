# prismqueer

[![Crates.io](https://img.shields.io/crates/v/prismqueer.svg)](https://crates.io/crates/prismqueer)
[![Documentation](https://docs.rs/prismqueer/badge.svg)](https://docs.rs/prismqueer)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A typed transformation pipeline. The `Prism` trait + `Beam` carrier +
`Optic` concrete beam. Zero deps above the foundation; staged
transformations compose with compile-time type checking.

## Install

```sh
cargo add prismqueer
```

Optional features:

- `optics` — classical optics (`Lens`, `Iso`, `Traversal`, `Fold`, `Setter`,
  `OpticPrism`) expressed through the `Prism` trait.
- `bundle` — the principal-bundle tower (`Fiber` → `Connection` → `Gauge`
  → `Transport` → `Closure`) and `IdentityPrism`. Implies `optics`.
- `lambda` — a tiny content-addressed lambda calculus + `#[derive(Lambda)]`.
- `pq` — the typed pq wire DSL (`Target` / `Filter` / `Output`) with
  serde + JSON Schema derivation.
- `lapack` — dispatch the `KernelSpec` / `SpectralDimension` machinery
  to Fortran via a small `cc` build.

## What it gives you

- **`Prism`** — a trait with three associated beams and three methods
  (`focus`, `project`, `settle`). The compiler enforces that each
  stage's output type matches the next stage's input type.
- **`Beam`** — the semifunctor over [`Imperfect`](https://docs.rs/terni).
  Carries a value, the input that produced it, and accumulated loss.
  Failure beams are dark: a fixpoint under `smap` and `next`.
- **`Optic<In, Out, E, L>`** — the standard `Beam` implementation.
- **`apply`** — run a prism end-to-end: `focus`, then `project`,
  then `settle`.
- **`IdentityPrism`** — the identity element. (Feature: `bundle`.)

## Example

```rust
use prismqueer::{apply, Beam, Optic, Prism};

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

let out = apply(&CountPrism, Optic::ok((), "hello".to_string()));
assert_eq!(out.result().ok(), Some(&"5 chars".to_string()));
```

Full API on [docs.rs](https://docs.rs/prismqueer).

## Why `prismqueer`

The crate was internally `prism-core` during development; that name
was taken on crates.io by an unrelated project. The cascade renamed.
The math is unchanged; the kernel is the same kernel.

## Status

Pre-1.0. The trait surface is shaped by ongoing substrate work in the
[mirror](https://github.com/systemic-engineering/mirror) and
[prism](https://github.com/systemic-engineering/prism) repositories.
Breaking changes are possible at any minor version until 1.0; minor
versions track substrate movement, not LTS.

## License

Apache-2.0.
