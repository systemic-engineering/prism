# prismqueer

[![Crates.io](https://img.shields.io/crates/v/prismqueer.svg)](https://crates.io/crates/prismqueer)
[![Documentation](https://docs.rs/prismqueer/badge.svg)](https://docs.rs/prismqueer)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

The spectral-triple substrate — the algebra A. Five operations, the `Prism` trait, `IdentityPrism`, zero deps. The foundation.

## What it is

`prismqueer` is the kernel of a sub-Turing compiler whose generated code inherits formal verification by structural construction. The crate exposes:

- **The five operations** — `focus`, `project`, `split`, `shift`, `settle`. The same algebra at every depth.
- **The `Prism` trait** — every transformation in the substrate implements it.
- **`IdentityPrism`** — the identity element of the monoid.
- **`Beam<T>`** — the typed-carrier flowing through a pipeline.
- **Zero deps** — nothing above this in the stack.

It's the foundational crate. Everything else — [terni](https://crates.io/crates/terni)'s ternary carrier, [prismqueer-projections](https://crates.io/crates/prismqueer-projections)'s derive macros, the mirror compiler — builds on it.

## Why `prismqueer`

The crate was internally called `prism-core` during development. By the time the cascade was ready for crates.io, that name was taken by an unrelated agentic-AI reliability project. We named what we made: `prismqueer`. The kernel IS ours; the name says so.

Nothing about the math changed. Substrate-pull was the rename.

## Status

Pre-1.0. The trait surface is shaped by ongoing substrate work in [systemic-engineering/mirror](https://github.com/systemic-engineering/mirror) and [systemic-engineering/prism](https://github.com/systemic-engineering/prism). Breaking changes possible at any minor version until 1.0.

The math is stable. The names are stable enough that the published surface won't churn weekly. But this is research infrastructure, not LTS.

## License

Apache-2.0.

The glass is Apache-2.0. The wine governs itself.
