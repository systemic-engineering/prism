# prismqueer-projections

[![Crates.io](https://img.shields.io/crates/v/prismqueer-projections.svg)](https://crates.io/crates/prismqueer-projections)
[![Documentation](https://docs.rs/prismqueer-projections/badge.svg)](https://docs.rs/prismqueer-projections)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

Proc-macros for [`prismqueer`](https://crates.io/crates/prismqueer).
You write the declaration; the macro emits the boilerplate that puts
your types into the algebra.

## Install

You don't usually depend on this crate directly — `prismqueer`
re-exports its derives. If you need it standalone:

```sh
cargo add prismqueer-projections
```

## Macros

- **`#[derive(Prism)]`** — emits `Addressable` + `Display` from
  `#[oid("@name")]`. Field-level annotations (`#[lens]`, `#[prism]`,
  `#[traversal]`, `#[iso]`) generate per-field accessor structs and a
  `Self::optic_fields() -> &'static [FieldOptic]` metadata function.
- **`#[derive(Lambda)]`** — emits `Addressable` + `Display` +
  `LambdaImpl` + `Composable` for a named lambda phase. Requires the
  `lambda` feature on `prismqueer`.
- **`declaration!{ … }`** — the function-like macro that reads a
  substrate declaration (`type`, `prism`, `action`, `grammar`) and
  emits the realising Rust item. The substrate path IS the runtime
  address: `prism @kernel` becomes a unit struct whose
  `Addressable::oid()` is `Oid::hash("@kernel")`.

## Example

```rust,ignore
use prismqueer::{Addressable, DerivePrism};

#[derive(DerivePrism)]
#[oid("@claims")]
struct ClaimProcessor {
    #[lens]     adjuster_id:     u64,
    #[prism]    override_reason: Option<String>,
    #[traversal] history:        Vec<u32>,
}

assert_eq!(format!("{}", ClaimProcessor { adjuster_id: 0, override_reason: None, history: vec![] }),
           "@claims");
```

Full API on [docs.rs](https://docs.rs/prismqueer-projections).

## Why `projections`

`project` is one of the canonical Prism operations. The projection
layer projects declarations from their written form into the substrate
algebra. The naming is structural.

## Status

Pre-1.0. Tracks `prismqueer` versions; expect breaking changes
until 1.0.

## License

Apache-2.0.
