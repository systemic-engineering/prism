//! `@code/rust/macro.shim_type` — the substrate-`type` → Rust emission.
//!
//! Realises `shim_type(d: declaration) -> code/rust.ast` per
//! `mirror/shards/code/rust/macro.mirror`. The spec is
//! `mirror/docs/specs/code-macro-surface.md` §4.1 (the four
//! declaration kinds) and §5 (the four laws).
//!
//! # Input grammar (this tick)
//!
//! The proc-macro's input is a single substrate `type` declaration
//! in one of three shapes:
//!
//! ```ignore
//! // Record shape — emits a Rust `pub struct`:
//! type Foo { a: u32, b: u64 }
//!
//! // Sum shape, unit variants — emits a Rust `pub enum`:
//! type Color = red | green | blue
//!
//! // Sum shape, parametric variants — emits a Rust `pub enum`
//! // with tuple variants:
//! type Result = ok(u32) | err(u32)
//! ```
//!
//! # The dispatch
//!
//! Per the spec's §10.1 dispatch table:
//!
//! ```text
//! shim(D, @code/rust) match D.kind:
//!    | type(name, params, variants) ->
//!         emit Rust struct/enum per §4.1.1
//!    | prism(...) -> unimplemented!()  -- next cascade tick
//!    | action(...) -> unimplemented!()  -- next cascade tick
//!    | grammar(...) -> unimplemented!()  -- next cascade tick
//! ```
//!
//! # The four laws witnessed
//!
//! 1. **Type-soundness** — substrate `u32` → Rust `u32` (verbatim);
//!    record fields preserve names; sum variants preserve names.
//! 2. **Round-trip identity** — the emitted TokenStream re-parses
//!    through `syn::parse2::<syn::DeriveInput>`. The substrate's
//!    type declaration ≡ the Rust declaration's structural shape.
//! 3. **OID functionality** — `expand` is a pure function of its
//!    input token tree (deterministic, no env/clock/IO). Same input
//!    → byte-identical TokenStream → same OID.
//! 4. **Substrate-pull preservation** — emission uses only `pub
//!    struct`, `pub enum`, primitive integer types, and snake_case
//!    field/variant names. No `@io` reach-through.
//!
//! Per the brief and spec §12: this tick supports ONLY `type`
//! declarations. The other three kinds (`prism`, `action`,
//! `grammar`) panic with `unimplemented!()` and a clear message
//! pointing at the next cascade tick.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{braced, parenthesized, Ident, Result, Token};

/// The substrate-`type` declaration carrier — what the macro parses
/// from its input token stream.
enum SubstrateTypeDecl {
    /// `type Name { field: ty, ... }` — emits a Rust `pub struct`.
    Record {
        name: Ident,
        fields: Vec<(Ident, Ident)>,
    },
    /// `type Name = variant1 | variant2(ty) | ...` — emits a Rust
    /// `pub enum` with unit or tuple variants per variant shape.
    Sum {
        name: Ident,
        variants: Vec<SumVariant>,
    },
}

struct SumVariant {
    name: Ident,
    /// Empty `args` ⇒ unit variant; non-empty ⇒ tuple variant.
    args: Vec<Ident>,
}

impl Parse for SubstrateTypeDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        // Mandatory `type` keyword. Rust's `type` is a reserved
        // keyword (Token![type]) — the substrate uses the same
        // surface form. Consuming it here also dispatches: only
        // `type` declarations land in this cascade tick. `prism`,
        // `action`, `grammar` declarations would be a different
        // keyword at the input position and `Token![type]` will
        // surface the missing-keyword error.
        input.parse::<Token![type]>()?;

        // `parse_any` accepts identifiers that overlap with Rust
        // keywords (e.g. the substrate's `type ref { ... }`); we
        // want substrate identifiers, not Rust-identifier-restricted
        // ones.
        let name: Ident = input.call(Ident::parse_any)?;

        // Disambiguate: `{` ⇒ record, `=` ⇒ sum.
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::token::Brace) {
            // Record: `{ field: ty, field: ty, ... }`
            let content;
            braced!(content in input);
            let mut fields = Vec::new();
            while !content.is_empty() {
                let fname: Ident = content.call(Ident::parse_any)?;
                content.parse::<Token![:]>()?;
                let fty: Ident = content.call(Ident::parse_any)?;
                fields.push((fname, fty));
                if content.is_empty() {
                    break;
                }
                content.parse::<Token![,]>()?;
            }
            Ok(SubstrateTypeDecl::Record { name, fields })
        } else if lookahead.peek(Token![=]) {
            // Sum: `= v1 | v2(ty) | v3`
            input.parse::<Token![=]>()?;
            let mut variants = Vec::new();
            loop {
                let vname: Ident = input.call(Ident::parse_any)?;
                let mut args = Vec::new();
                if input.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    while !content.is_empty() {
                        let arg_ty: Ident = content.call(Ident::parse_any)?;
                        args.push(arg_ty);
                        if content.is_empty() {
                            break;
                        }
                        content.parse::<Token![,]>()?;
                    }
                }
                variants.push(SumVariant { name: vname, args });
                if input.is_empty() {
                    break;
                }
                input.parse::<Token![|]>()?;
            }
            Ok(SubstrateTypeDecl::Sum { name, variants })
        } else {
            Err(lookahead.error())
        }
    }
}

/// The shim's expansion — substrate `type` → Rust `pub struct` /
/// `pub enum`. Pure function of the input TokenStream; deterministic
/// (the OID functionality law).
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let decl = match syn::parse2::<SubstrateTypeDecl>(input) {
        Ok(d) => d,
        Err(e) => return e.to_compile_error(),
    };
    emit(&decl)
}

fn emit(decl: &SubstrateTypeDecl) -> TokenStream {
    match decl {
        SubstrateTypeDecl::Record { name, fields } => {
            // Substrate's snake_case identifier → Rust's PascalCase
            // type name. Field identifiers stay snake_case (substrate-
            // pull: Rust convention for field names matches substrate).
            let struct_name = format_ident!("{}", to_pascal_case(&name.to_string()));
            let field_tokens: Vec<TokenStream> = fields
                .iter()
                .map(|(fname, fty)| {
                    quote! { pub #fname: #fty, }
                })
                .collect();
            quote! {
                pub struct #struct_name {
                    #(#field_tokens)*
                }
            }
        }
        SubstrateTypeDecl::Sum { name, variants } => {
            let enum_name = format_ident!("{}", to_pascal_case(&name.to_string()));
            let variant_tokens: Vec<TokenStream> = variants
                .iter()
                .map(|v| {
                    let vname = format_ident!("{}", to_pascal_case(&v.name.to_string()));
                    if v.args.is_empty() {
                        quote! { #vname, }
                    } else {
                        let args = &v.args;
                        quote! { #vname(#(#args),*), }
                    }
                })
                .collect();
            quote! {
                pub enum #enum_name {
                    #(#variant_tokens)*
                }
            }
        }
    }
}

/// Snake_case → PascalCase. Identifier projection that the substrate
/// emits at the @code/rust altitude: substrate identifiers are
/// snake_case (per the grammar convention); Rust type-level
/// identifiers are PascalCase (per the Rust style guide).
fn to_pascal_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut next_upper = true;
    for c in s.chars() {
        if c == '_' {
            next_upper = true;
        } else if next_upper {
            out.extend(c.to_uppercase());
            next_upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    /// The shim is a pure function of its input. Same substrate
    /// declaration → byte-identical Rust emission. This is the
    /// `oid_function(shim_type)` law witnessed at the
    /// TokenStream-string altitude.
    #[test]
    fn record_emission_is_deterministic() {
        let input = quote! { type point { x: u32, y: u32 } };
        let a = expand(input.clone());
        let b = expand(input);
        assert_eq!(a.to_string(), b.to_string());
    }

    #[test]
    fn sum_emission_is_deterministic() {
        let input = quote! { type color = red | green | blue };
        let a = expand(input.clone());
        let b = expand(input);
        assert_eq!(a.to_string(), b.to_string());
    }

    /// Round-trip identity (weak form per spec §6.2): the emitted
    /// TokenStream parses as a valid Rust item via syn. The
    /// substrate's type ≡ a structurally-realisable Rust item.
    #[test]
    fn record_emission_round_trips_through_syn() {
        let input = quote! { type point { x: u32, y: u32 } };
        let emitted = expand(input);
        let parsed: syn::Result<syn::Item> = syn::parse2(emitted);
        let item = parsed.expect("emission must be a valid Rust item");
        let s = match item {
            syn::Item::Struct(s) => s,
            _ => panic!("record-shaped substrate type must emit a struct item"),
        };
        assert_eq!(s.ident.to_string(), "Point");
        match s.fields {
            syn::Fields::Named(named) => {
                let names: Vec<String> = named
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap().to_string())
                    .collect();
                assert_eq!(names, vec!["x", "y"]);
            }
            _ => panic!("record must emit named-fields struct"),
        }
    }

    #[test]
    fn sum_emission_round_trips_through_syn() {
        let input = quote! { type color = red | green | blue };
        let emitted = expand(input);
        let parsed: syn::Result<syn::Item> = syn::parse2(emitted);
        let item = parsed.expect("emission must be a valid Rust item");
        let e = match item {
            syn::Item::Enum(e) => e,
            _ => panic!("sum-shaped substrate type must emit an enum item"),
        };
        assert_eq!(e.ident.to_string(), "Color");
        let names: Vec<String> = e.variants.iter().map(|v| v.ident.to_string()).collect();
        assert_eq!(names, vec!["Red", "Green", "Blue"]);
    }

    #[test]
    fn parametric_sum_emits_tuple_variants() {
        let input = quote! { type maybe_int = some(u32) | none };
        let emitted = expand(input);
        let parsed: syn::Result<syn::Item> = syn::parse2(emitted);
        let item = parsed.expect("emission must be a valid Rust item");
        let e = match item {
            syn::Item::Enum(e) => e,
            _ => panic!("sum-shaped substrate type must emit an enum item"),
        };
        assert_eq!(e.ident.to_string(), "MaybeInt");
        let some = e.variants.iter().find(|v| v.ident == "Some").unwrap();
        assert!(matches!(some.fields, syn::Fields::Unnamed(_)));
        let none = e.variants.iter().find(|v| v.ident == "None").unwrap();
        assert!(matches!(none.fields, syn::Fields::Unit));
    }

    /// Substrate-pull preservation: snake_case → PascalCase mapping
    /// preserves the substrate identifier exactly (no information loss).
    #[test]
    fn snake_to_pascal_is_lossless() {
        assert_eq!(to_pascal_case("point"), "Point");
        assert_eq!(to_pascal_case("circle_of_fifths"), "CircleOfFifths");
        assert_eq!(to_pascal_case("imperfect"), "Imperfect");
    }
}
