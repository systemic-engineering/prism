//! Derive macros for prism-core.
//!
//! `#[derive(Prism)]` with `#[oid("@something")]` generates:
//! - `Addressable` impl (Oid from the `@name` string, deterministic)
//! - `Display` impl (prints the `@name`)
//!
//! Field-level optic annotations generate accessor structs:
//! - `#[lens]` — total, bidirectional. Generates `FieldNameLens` with `view`/`set`.
//! - `#[prism]` — partial access. Generates `FieldNamePrism` with `extract`/`review`.
//! - `#[traversal]` — multiple targets. Generates `FieldNameTraversal` with `traverse`/`traverse_mut`.
//! - `#[iso]` — round-trip lossless. Generates `FieldNameIso` with `forward`/`backward`.
//!
//! Also generates `optic_fields()` returning `&'static [FieldOptic]` metadata.

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Expr, Fields, Lit, Meta, Type};

/// Which optic annotation a field carries.
#[derive(Clone, Copy)]
enum FieldKind {
    Lens,
    Prism,
    Traversal,
    Iso,
}

/// A field with its optic annotation.
struct AnnotatedField {
    ident: syn::Ident,
    ty: Type,
    kind: FieldKind,
}

/// Derive `Addressable`, `Display`, optic accessors, and `optic_fields()`.
///
/// # Struct-level attributes
///
/// - `#[oid("@name")]` — required. The `@name` must start with `@`.
///
/// # Field-level attributes
///
/// - `#[lens]` — total, bidirectional access. Compile error on `Option<T>`.
/// - `#[prism]` — partial access. Typically on `Option<T>` or `Result<T, E>`.
/// - `#[traversal]` — multiple targets. Typically on `Vec<T>`.
/// - `#[iso]` — round-trip lossless conversion.
/// - `#[prism(inner)]` — delegation marker (recognized, no code generated yet).
///
/// # Example
///
/// ```ignore
/// #[derive(Prism)]
/// #[oid("@claims")]
/// struct ClaimProcessor {
///     #[lens]
///     adjuster_id: u64,
///     #[prism]
///     override_reason: Option<String>,
///     #[traversal]
///     history: Vec<Event>,
/// }
/// ```
#[proc_macro_derive(Prism, attributes(oid, prism, lens, traversal, iso))]
pub fn derive_prism(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Extract #[oid("@something")] from struct attributes
    let oid_name = extract_oid_name(&input);

    // Extract annotated fields from struct body
    let annotated = extract_annotated_fields(&input);

    // Validate annotations
    validate_annotations(&annotated);

    // Generate accessor structs for each annotated field
    let accessor_structs = generate_accessors(name, &annotated);

    // Generate optic_fields() metadata
    let optic_fields_impl = generate_optic_fields(name, &annotated, &impl_generics, &ty_generics, where_clause);

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

        #accessor_structs

        #optic_fields_impl
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
    panic!("#[derive(Prism)] requires #[oid(\"@name\")] attribute");
}

fn extract_annotated_fields(input: &DeriveInput) -> Vec<AnnotatedField> {
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };

    let mut annotated = Vec::new();

    for field in fields {
        let ident = match &field.ident {
            Some(id) => id.clone(),
            None => continue,
        };

        let mut kind = None;

        for attr in &field.attrs {
            if attr.path().is_ident("lens") {
                kind = Some(FieldKind::Lens);
            } else if attr.path().is_ident("prism") {
                // Distinguish #[prism] (optic) from #[prism(inner)] (delegation)
                match &attr.meta {
                    Meta::Path(_) => {
                        kind = Some(FieldKind::Prism);
                    }
                    Meta::List(_) => {
                        // #[prism(inner)] — delegation marker, skip
                    }
                    _ => {}
                }
            } else if attr.path().is_ident("traversal") {
                kind = Some(FieldKind::Traversal);
            } else if attr.path().is_ident("iso") {
                kind = Some(FieldKind::Iso);
            }
        }

        if let Some(k) = kind {
            annotated.push(AnnotatedField {
                ident,
                ty: field.ty.clone(),
                kind: k,
            });
        }
    }

    annotated
}

/// Validate optic annotations against field types.
fn validate_annotations(fields: &[AnnotatedField]) {
    for field in fields {
        match field.kind {
            FieldKind::Lens => {
                // #[lens] on Option<T> is a compile error
                if is_option_type(&field.ty) {
                    panic!(
                        "#[lens] on field `{}`: Lens is total, but Option<T> is partial. Use #[prism] instead.",
                        field.ident
                    );
                }
            }
            _ => {}
        }
    }
}

/// Check if a type is `Option<T>`.
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Check if a type is `Vec<T>`.
fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Vec";
        }
    }
    false
}

/// Extract the inner type from `Option<T>` or `Vec<T>`.
fn extract_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

/// Convert a snake_case field name to PascalCase for the accessor struct name.
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn generate_accessors(struct_name: &syn::Ident, fields: &[AnnotatedField]) -> proc_macro2::TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();

    for field in fields {
        let field_ident = &field.ident;
        let field_name = field.ident.to_string();
        let pascal = to_pascal_case(&field_name);

        match field.kind {
            FieldKind::Lens => {
                let accessor_name = format_ident!("{}Lens", pascal, span = Span::call_site());
                let field_ty = &field.ty;

                tokens.extend(quote! {
                    /// Lens accessor for field `#field_name`. Total, bidirectional.
                    pub struct #accessor_name;

                    impl #accessor_name {
                        /// Get a reference to the field value.
                        pub fn view(source: &#struct_name) -> &#field_ty {
                            &source.#field_ident
                        }

                        /// Set the field value.
                        pub fn set(source: &mut #struct_name, value: #field_ty) {
                            source.#field_ident = value;
                        }
                    }

                    impl prism_core::Named<#accessor_name> {
                        /// Create a named lens for this field.
                        pub fn lens() -> prism_core::Named<#accessor_name> {
                            prism_core::Named(#field_name, #accessor_name)
                        }
                    }
                });
            }
            FieldKind::Prism => {
                let accessor_name = format_ident!("{}Prism", pascal, span = Span::call_site());
                let inner_ty = extract_inner_type(&field.ty);

                if let Some(inner) = inner_ty {
                    if is_option_type(&field.ty) {
                        tokens.extend(quote! {
                            /// Prism accessor for field `#field_name`. Partial access.
                            pub struct #accessor_name;

                            impl #accessor_name {
                                /// Extract the value if present.
                                pub fn extract(source: &#struct_name) -> Option<&#inner> {
                                    source.#field_ident.as_ref()
                                }

                                /// Set the value (wraps in Some).
                                pub fn review(source: &mut #struct_name, value: #inner) {
                                    source.#field_ident = Some(value);
                                }
                            }

                            impl prism_core::Named<#accessor_name> {
                                /// Create a named prism for this field.
                                pub fn prism() -> prism_core::Named<#accessor_name> {
                                    prism_core::Named(#field_name, #accessor_name)
                                }
                            }
                        });
                    } else {
                        // Non-Option prism: direct access with Option wrapping
                        let field_ty = &field.ty;
                        tokens.extend(quote! {
                            /// Prism accessor for field `#field_name`. Partial access.
                            pub struct #accessor_name;

                            impl #accessor_name {
                                /// Extract the value (always Some for non-Option types).
                                pub fn extract(source: &#struct_name) -> Option<&#field_ty> {
                                    Some(&source.#field_ident)
                                }

                                /// Set the value.
                                pub fn review(source: &mut #struct_name, value: #field_ty) {
                                    source.#field_ident = value;
                                }
                            }
                        });
                    }
                } else {
                    // Fallback for non-generic types
                    let field_ty = &field.ty;
                    tokens.extend(quote! {
                        pub struct #accessor_name;

                        impl #accessor_name {
                            pub fn extract(source: &#struct_name) -> Option<&#field_ty> {
                                Some(&source.#field_ident)
                            }

                            pub fn review(source: &mut #struct_name, value: #field_ty) {
                                source.#field_ident = value;
                            }
                        }
                    });
                }
            }
            FieldKind::Traversal => {
                let accessor_name = format_ident!("{}Traversal", pascal, span = Span::call_site());
                let field_ty = &field.ty;

                if is_vec_type(&field.ty) {
                    if let Some(inner) = extract_inner_type(&field.ty) {
                        tokens.extend(quote! {
                            /// Traversal accessor for field `#field_name`. Multiple targets.
                            pub struct #accessor_name;

                            impl #accessor_name {
                                /// Get a slice of all elements.
                                pub fn traverse(source: &#struct_name) -> &[#inner] {
                                    &source.#field_ident
                                }

                                /// Get a mutable reference to the vec.
                                pub fn traverse_mut(source: &mut #struct_name) -> &mut Vec<#inner> {
                                    &mut source.#field_ident
                                }
                            }

                            impl prism_core::Named<#accessor_name> {
                                /// Create a named traversal for this field.
                                pub fn traversal() -> prism_core::Named<#accessor_name> {
                                    prism_core::Named(#field_name, #accessor_name)
                                }
                            }
                        });
                    }
                } else {
                    // Non-Vec traversal: just expose the field
                    tokens.extend(quote! {
                        pub struct #accessor_name;

                        impl #accessor_name {
                            pub fn traverse(source: &#struct_name) -> &#field_ty {
                                &source.#field_ident
                            }

                            pub fn traverse_mut(source: &mut #struct_name) -> &mut #field_ty {
                                &mut source.#field_ident
                            }
                        }
                    });
                }
            }
            FieldKind::Iso => {
                let accessor_name = format_ident!("{}Iso", pascal, span = Span::call_site());
                let field_ty = &field.ty;

                tokens.extend(quote! {
                    /// Iso accessor for field `#field_name`. Round-trip lossless.
                    pub struct #accessor_name;

                    impl #accessor_name {
                        /// Forward: get the field value.
                        pub fn forward(source: &#struct_name) -> &#field_ty {
                            &source.#field_ident
                        }

                        /// Backward: set the field value.
                        pub fn backward(source: &mut #struct_name, value: #field_ty) {
                            source.#field_ident = value;
                        }
                    }
                });
            }
        }
    }

    tokens
}

fn generate_optic_fields(
    struct_name: &syn::Ident,
    fields: &[AnnotatedField],
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    if fields.is_empty() {
        return quote! {};
    }

    let entries: Vec<proc_macro2::TokenStream> = fields
        .iter()
        .map(|f| {
            let field_name = f.ident.to_string();
            let kind = match f.kind {
                FieldKind::Iso => quote! { prism_core::OpticKind::Iso },
                FieldKind::Lens => quote! { prism_core::OpticKind::Lens },
                FieldKind::Prism => quote! { prism_core::OpticKind::Prism },
                FieldKind::Traversal => quote! { prism_core::OpticKind::Traversal },
            };
            quote! {
                prism_core::FieldOptic {
                    name: #field_name,
                    kind: #kind,
                }
            }
        })
        .collect();

    let count = entries.len();

    quote! {
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Returns metadata about all optic-annotated fields.
            pub fn optic_fields() -> &'static [prism_core::FieldOptic] {
                static FIELDS: [prism_core::FieldOptic; #count] = [
                    #(#entries),*
                ];
                &FIELDS
            }
        }
    }
}
