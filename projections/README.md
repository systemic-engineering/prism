# prismqueer-projections

[![Crates.io](https://img.shields.io/crates/v/prismqueer-projections.svg)](https://crates.io/crates/prismqueer-projections)
[![Documentation](https://docs.rs/prismqueer-projections/badge.svg)](https://docs.rs/prismqueer-projections)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

The projection layer of [`prismqueer`](https://crates.io/crates/prismqueer)'s algebra. Proc-macros that turn declarations into substrate.

## What it does

`prismqueer-projections` is the proc-macro half of the `prismqueer` substrate. You write the declaration; the macro emits the boilerplate that puts your types into the algebra.

The macros:

- `#[derive(Prism)]` — the canonical `Prism` trait derive
- `#[oid]` — OID-shaped identity for content-addressed substrate values
- `declaration!` — the round-trip-stable declaration form (T23 of the substrate roadmap)

All of them emit token streams that reference `prismqueer::...`. You depend on this crate at compile time; the substrate runtime stays in `prismqueer` proper.

## Why `projections`

`project` is one of the five operations. The projection layer projects declarations from their written form into the substrate algebra. The naming is structural, not cosmetic.

## Status

Pre-1.0. Tracks `prismqueer` versions; expect breaking changes until 1.0.

## License

Apache-2.0.
