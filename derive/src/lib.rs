//! Derive macros for prism-core.
//!
//! `#[derive(Named)]` with `#[oid("@something")]` generates:
//! - `Addressable` impl (Oid from the `@name` string)
//! - `Display` impl (prints the `@name`)

extern crate proc_macro;

use proc_macro::TokenStream;

/// Derive `Addressable` and `Display` from `#[oid("@name")]`.
///
/// Optionally marks a field with `#[prism(inner)]` for future
/// Prism delegation.
#[proc_macro_derive(Named, attributes(oid, prism))]
pub fn derive_named(_input: TokenStream) -> TokenStream {
    // TODO: implement
    TokenStream::new()
}
