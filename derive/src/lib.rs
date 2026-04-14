//! Derive macros for prism-core.
//!
//! `#[derive(Named)]` with `#[oid("@something")]` generates:
//! - `Addressable` impl (Oid from the `@name` string, deterministic)
//! - `Display` impl (prints the `@name`)
//!
//! The `#[prism(inner)]` attribute on a field is recognized but reserved
//! for future Prism delegation.

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Expr, Lit, Meta};

/// Derive `Addressable` and `Display` from `#[oid("@name")]`.
///
/// The `@name` must start with `@`. The generated `Addressable::oid()`
/// hashes the name string to produce a deterministic Oid that depends
/// only on the name, not on the struct's field values.
///
/// `#[prism(inner)]` on a field is accepted (no compile error) but
/// does not generate delegation code yet.
///
/// # Example
///
/// ```ignore
/// #[derive(Named)]
/// #[oid("@test")]
/// struct MyStruct {
///     value: u32,
/// }
/// ```
#[proc_macro_derive(Named, attributes(oid, prism))]
pub fn derive_named(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Extract #[oid("@something")] from struct attributes
    let oid_name = extract_oid_name(&input);

    let expanded = quote! {
        impl #impl_generics prism_core::Addressable for #name #ty_generics #where_clause {
            fn oid(&self) -> prism_core::Oid {
                prism_core::Oid::hash(#oid_name.as_bytes())
            }
        }

        impl #impl_generics ::std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", #oid_name)
            }
        }
    };

    TokenStream::from(expanded)
}

fn extract_oid_name(input: &DeriveInput) -> String {
    for attr in &input.attrs {
        if attr.path().is_ident("oid") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.clone();
                if let Ok(expr) = syn::parse2::<Expr>(tokens) {
                    if let Expr::Lit(lit) = expr {
                        if let Lit::Str(s) = &lit.lit {
                            let val = s.value();
                            if !val.starts_with('@') {
                                panic!(
                                    "#[oid(\"...\")] value must start with '@', got: {:?}",
                                    val
                                );
                            }
                            return val;
                        }
                    }
                }
            }
        }
    }
    panic!("#[derive(Named)] requires #[oid(\"@name\")] attribute");
}
