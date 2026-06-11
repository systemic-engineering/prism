//! `@code/rust/macro.shim_{type,prism,...}` — the substrate → Rust
//! emission. Realises the four shims declared at
//! `mirror/shards/code/metalogue.mirror` (the universal ground; the
//! 34th-instance reframe lifts the contract from per-species
//! `@code/X/macro` to `@code/metalogue`) and bound to
//! `code/rust.ast` at `mirror/shards/code/rust/macro.mirror`. The
//! spec is `mirror/docs/specs/code-metalogue-surface.md` §4.1 (the
//! four declaration kinds) and §5 (the four laws).
//!
//! # Input grammar (T23 + T24)
//!
//! The proc-macro's input is a single substrate declaration. Two
//! kinds are now supported; the cascade continues with `action`
//! (T25) and `grammar` (T26).
//!
//! ```ignore
//! // T23 — `type` declarations.
//! //
//! // Record shape — emits a Rust `pub struct`:
//! type Foo { a: u32, b: u64 }
//!
//! // Sum shape, unit variants — emits a Rust `pub enum`:
//! type Color = red | green | blue
//!
//! // Sum shape, parametric variants — emits a Rust `pub enum`
//! // with tuple variants:
//! type Result = ok(u32) | err(u32)
//!
//! // T24 — `prism` declarations.
//! //
//! // Bare path — emits a Rust unit struct with #[derive(Prism)]:
//! prism @parse
//!
//! // With five-op block — same emission (the block is the
//! // universal Prism algebra; #[derive(Prism)] fills it in):
//! prism @kernel {
//!     focus kernel
//!     project kernel
//!     split kernel
//!     shift kernel
//!     settle kernel
//! }
//!
//! // T25 — `action` declarations.
//! //
//! // Substrate `action name(args) -> ret { \ }` — emits a Rust
//! // `pub fn` matching the signature, with `todo!()` body. The
//! // body resolution is forward-promised to T25.5+ (consumer-pull).
//! //
//! // The substrate-pull seam: rustc's lexer rejects free `\`, so
//! // the Rust-altitude call site uses empty body `{ }` as the
//! // realisation of substrate `{ \ }`. The semantic is unchanged
//! // ("unresolved body"); the glyph crosses the glass wall as `{}`.
//! // Substrate `.mirror` declarations keep their `\`; Rust
//! // proc-macro call sites use `{ }`. The shim emits `todo!()`.
//! action increment(x: u32) -> u32 { }
//! action effect() -> () { }
//!
//! // T26 — `grammar` declarations.
//! //
//! // Substrate `grammar @path(ext1, ext2, ...) { body }` — emits a
//! // Rust unit struct with #[derive(prism_core::DerivePrism)] +
//! // #[oid("@path")]. The extension list (`("spec")` /
//! // `("mirror", "shard")`) is parsed-but-not-encoded at the floor;
//! // same precedent as the prism five-op block — the substrate
//! // surface declaration is structural, and the Rust-altitude floor
//! // witnesses the path as the runtime address. Body encoding (the
//! // inner `<op> <keyword>` pairs) is forward-promised to T26.5+
//! // (consumer-pull) when a downstream tick needs the keyword table
//! // at the Rust altitude.
//! //
//! // The extensions list is OPTIONAL — bare `grammar @path { ... }`
//! // is admitted (some substrate grammars carry no extension claim).
//! grammar @parse_spec("spec") { }
//! grammar @mirror_grammar("mirror", "shard") { focus prism }
//! ```
//!
//! # The dispatch
//!
//! Per the spec's §10.1 dispatch table (now declared at
//! `@code/metalogue` per the 34th-instance reframe):
//!
//! ```text
//! shim(D, @code/rust) match D.kind:
//!    | type(name, params, variants) ->
//!         emit Rust struct/enum per §4.1.1
//!    | prism(@path, five_op_block) ->
//!         emit Rust unit struct + #[derive(prism_core::DerivePrism)]
//!         + #[oid("@path")] per §4.1.2
//!    | action(name, args, ret, \ body) ->
//!         emit Rust `pub fn name(args) -> ret { todo!() }` per
//!         §4.1.3 (T25). The three classify sub-cases (substrate /
//!         boundary / partial) all share this floor; body fill-in
//!         is forward-promised to T25.5+ (consumer-pull).
//!    | grammar(@path, extensions, body) ->
//!         emit Rust unit struct + #[derive(prism_core::DerivePrism)]
//!         + #[oid("@path")] per §4.1.4 (T26). Same shape as `prism`
//!         at the floor — the extension list and body block are
//!         parsed-but-not-encoded; the path IS the runtime address.
//! ```
//!
//! # The four laws witnessed
//!
//! 1. **Type-soundness** — substrate `u32` → Rust `u32` (verbatim);
//!    record fields preserve names; sum variants preserve names;
//!    action signatures preserve arg patterns / types / return type
//!    verbatim via syn's `FnArg` + `Type` carriers.
//! 2. **Round-trip identity** — the emitted TokenStream re-parses
//!    through `syn::parse2::<syn::Item>`. The substrate's declaration
//!    ≡ the Rust declaration's structural shape.
//! 3. **OID functionality** — `expand` is a pure function of its
//!    input token tree (deterministic, no env/clock/IO). Same input
//!    → byte-identical TokenStream → same OID.
//! 4. **Substrate-pull preservation** — emission uses only `pub
//!    struct`, `pub enum`, `pub fn`, primitive integer types,
//!    `todo!()` for unresolved bodies, and snake_case field/variant
//!    names. No `@io` reach-through.
//!
//! Per the brief and spec §12: this module supports `type` (T23),
//! `prism` (T24), `action` (T25), and `grammar` (T26) declarations.
//! T26 closes the four-shim cascade at the Rust altitude.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parenthesized, FnArg, Ident, LitStr, Result, Token, Type};

/// The substrate declaration carrier — the outer dispatch over the
/// four declaration kinds per `@code/metalogue`'s `declaration_kind`
/// enum. T23 lands `Type`; T24 lands `Prism`; T25 lands `Action`;
/// T26 lands `Grammar` — the four-shim cascade is now total at the
/// Rust altitude.
enum SubstrateDecl {
    Type(SubstrateTypeDecl),
    Prism(SubstratePrismDecl),
    Action(SubstrateActionDecl),
    Grammar(SubstrateGrammarDecl),
}

impl Parse for SubstrateDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        // Dispatch on the leading keyword. Rust's `type` is a
        // reserved keyword (Token![type]); the substrate's `prism`,
        // `action`, and `grammar` are not (substrate-level keywords
        // that surface as plain `Ident` in the Rust token stream).
        // Peek for `type` first; otherwise look for the substrate
        // identifier and match its text.
        if input.peek(Token![type]) {
            Ok(SubstrateDecl::Type(input.parse()?))
        } else if input.peek(Ident::peek_any) {
            // The next token is an identifier; check its text.
            let fork = input.fork();
            let ident: Ident = fork.call(Ident::parse_any)?;
            match ident.to_string().as_str() {
                "prism" => Ok(SubstrateDecl::Prism(input.parse()?)),
                "action" => Ok(SubstrateDecl::Action(input.parse()?)),
                "grammar" => Ok(SubstrateDecl::Grammar(input.parse()?)),
                other => Err(syn::Error::new(
                    ident.span(),
                    format!(
                        "unexpected substrate declaration kind `{other}`; expected one of \
                         `type`, `prism`, `action`, `grammar` per \
                         mirror/shards/code/metalogue.mirror's `declaration_kind`."
                    ),
                )),
            }
        } else {
            Err(input
                .error("expected a substrate declaration: `type`, `prism`, `action`, or `grammar`"))
        }
    }
}

/// The substrate-`action` declaration carrier — what the macro parses
/// from its input token stream when the leading keyword is `action`.
///
/// Per spec §4.1.3: `action name(args) -> ret { \ }` emits a Rust
/// `pub fn name(args) -> ret { todo!() }`. The body is the shim's
/// responsibility per the realisation discriminator's three sub-cases
/// (substrate / boundary / partial); the T25 floor lands the typed
/// signature with `todo!()` body for all three.
///
/// === The `\` → `{}` substrate-pull seam (T25 recognition) ===
///
/// Substrate `.mirror` declarations use `{ \ }` as the unresolved-body
/// marker (the `\` IS the substrate's question per
/// `[[architecture-prism-as-trait-as-everything]]`'s obligation block).
/// Rust's lexer rejects free `\` as "unknown start of token", so the
/// Rust-altitude call-site form is `{ }` (empty body). The semantic
/// is preserved ("unresolved"); only the glyph is species-local.
/// The shim translates either to `todo!()` body at emission.
struct SubstrateActionDecl {
    /// The action's name — stays lowercase (Rust fn names, not types).
    name: Ident,
    /// The argument list: `Punctuated<FnArg, Comma>` parsed as a Rust
    /// function signature would. Each `FnArg::Typed` carries a pat +
    /// type (e.g. `x: u32`).
    args: Punctuated<FnArg, Token![,]>,
    /// The return type: `syn::Type`. Covers `u32`, `()`, paths,
    /// references, tuples — anything syn parses as a type.
    ret: Type,
}

impl Parse for SubstrateActionDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        // Consume the `action` keyword (an Ident at the Rust token
        // altitude).
        let _action_kw: Ident = input.call(Ident::parse_any)?;

        // The fn name. `parse_any` to admit substrate identifiers
        // that may overlap Rust keywords (the substrate's `action ref`,
        // `action type`, etc.).
        let name: Ident = input.call(Ident::parse_any)?;

        // Parse args: `(x: u32, y: u32)` or `()`. Use syn's `FnArg`
        // parser, which handles `pat: type` natively.
        let args_content;
        parenthesized!(args_content in input);
        let args = Punctuated::<FnArg, Token![,]>::parse_terminated(&args_content)?;

        // Parse return: `-> ret`.
        input.parse::<Token![->]>()?;
        let ret: Type = input.parse()?;

        // Parse the body block. Per the substrate-pull seam doc'd at
        // the type doc above, the Rust-altitude form is `{ }` (empty);
        // we accept any block content and discard it (the realisation
        // discriminator's substrate / boundary / partial dispatch is
        // forward-promised to T25.5+; the T25 floor emits `todo!()`
        // regardless).
        let body_content;
        braced!(body_content in input);
        while !body_content.is_empty() {
            let _tok: proc_macro2::TokenTree = body_content.parse()?;
        }

        Ok(SubstrateActionDecl { name, args, ret })
    }
}

/// The substrate-`prism` declaration carrier — what the macro parses
/// from its input token stream when the leading keyword is `prism`.
///
/// Per spec §4.1.2: `prism @path { five_op_block }` emits a Rust
/// unit struct with `#[derive(prism_core::DerivePrism)]` and
/// `#[oid("@path")]`. The five-op block is the universal Prism
/// algebra; `#[derive(Prism)]` fills it in. This tick supports the
/// single-segment path case (`@parse`); multi-segment paths
/// (`@code/parse`) are forward-promised.
struct SubstratePrismDecl {
    /// The substrate path, with the leading `@` (e.g. `"@parse"`).
    /// Carried verbatim because it IS the OID hash input — the
    /// substrate-pull discipline names the path as the
    /// content-address.
    path: String,
    /// The PascalCased final segment of the path — the Rust struct
    /// name. `"@parse"` → `Parse`; `"@code/rust/macro"` → `Macro`.
    struct_name: Ident,
}

impl Parse for SubstratePrismDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        // Consume the `prism` keyword (an Ident at the Rust token
        // altitude). We've already peeked at it in the outer
        // dispatch; consume it here as the per-decl parser.
        let _prism_kw: Ident = input.call(Ident::parse_any)?;

        // Parse the substrate path — `@` then a slash-separated
        // identifier chain. Per the substrate convention the leading
        // `@` marks the path as a substrate reference.
        input.parse::<Token![@]>()?;
        let mut segments: Vec<String> = Vec::new();
        loop {
            let seg: Ident = input.call(Ident::parse_any)?;
            segments.push(seg.to_string());
            if input.peek(Token![/]) {
                input.parse::<Token![/]>()?;
                continue;
            }
            break;
        }
        let path = format!("@{}", segments.join("/"));
        // The struct name is the PascalCased final segment.
        let final_seg = segments.last().expect("path has at least one segment");
        let struct_name = format_ident!("{}", to_pascal_case(final_seg));

        // Optional five-op block. Consume-but-don't-encode: the
        // universal Prism algebra is supplied by #[derive(Prism)]
        // at the Rust altitude. Parsing it here lets the substrate
        // surface form match the spec's §4.1.2 grammar; emitting it
        // is not needed for round-trip identity (the canonical Rust
        // form is the unit struct + derive + oid attribute).
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            // Drain the block. The grammar is `focus IDENT \n project
            // IDENT \n ...`; we don't validate the five operations
            // strictly here — the substrate compiler does that.
            while !content.is_empty() {
                let _tok: proc_macro2::TokenTree = content.parse()?;
            }
        }

        Ok(SubstratePrismDecl { path, struct_name })
    }
}

/// The substrate-`grammar` declaration carrier — what the macro parses
/// from its input token stream when the leading keyword is `grammar`.
///
/// Per spec §4.1.4 (the four-shim contract at `@code/metalogue`):
/// `grammar @path(ext1, ext2, ...) { body }` declares a grammar
/// extension at the altitude named by `@path`, claiming the file
/// extensions listed in parens and binding the `<op> <keyword>`
/// pairs in the body block. The Rust-altitude floor follows the
/// `prism` shape: emit a unit struct with
/// `#[derive(prism_core::DerivePrism)]` + `#[oid("@path")]`. The
/// path IS the runtime address (Addressable::oid law).
///
/// The extensions list and body block are parsed-but-not-encoded at
/// the floor (same precedent as `prism`'s five-op block). Encoding
/// the extensions as a runtime `&[&str]` or the body as a keyword
/// table is forward-promised to T26.5+ (consumer-pull) when a
/// downstream Rust-altitude consumer needs them. The smallest tick
/// witnesses the path as the runtime address; that's the load-bearing
/// claim.
///
/// === Why `grammar` shares `prism`'s emission shape ===
///
/// Both are substrate altitudes addressed by a path. Both surface as
/// Rust unit structs whose `Addressable::oid` is the substrate path
/// hash. The substrate differentiates them by their family-root
/// declaration (`prism @X` vs `grammar @X`); the Rust altitude's
/// `Addressable` impl is structural — same shape, distinct OIDs by
/// virtue of distinct substrate paths. The four laws hold identically.
///
/// Examples (from the substrate):
/// - `grammar @mirror/spec("spec") { ... }` → `pub struct Spec;` with
///   `#[oid("@mirror/spec")]`.
/// - `grammar @code/mirror/grammar("mirror", "spec", "meta", "glass",
///   "shard", "shatter") { ... }` → `pub struct Grammar;` with
///   `#[oid("@code/mirror/grammar")]`.
struct SubstrateGrammarDecl {
    /// The substrate path, with the leading `@` (e.g. `"@mirror/spec"`).
    /// Carried verbatim because it IS the OID hash input — the
    /// substrate-pull discipline names the path as the content-address.
    path: String,
    /// The PascalCased final segment of the path — the Rust struct
    /// name. `"@mirror/spec"` → `Spec`; `"@code/mirror/grammar"` →
    /// `Grammar`. Same convention as the `prism` shape.
    struct_name: Ident,
    /// The extension claim (e.g. `["spec"]`, `["mirror", "shard"]`).
    /// Parsed but not encoded at the floor; carried on the struct so
    /// forward-promised consumer-pull ticks can emit them without
    /// re-parsing. Empty when the substrate declaration has no
    /// parenthesised extension list (bare `grammar @path { ... }`).
    #[allow(dead_code)]
    extensions: Vec<String>,
}

impl Parse for SubstrateGrammarDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        // Consume the `grammar` keyword (an Ident at the Rust token
        // altitude). We've already peeked at it in the outer
        // dispatch; consume it here as the per-decl parser.
        let _grammar_kw: Ident = input.call(Ident::parse_any)?;

        // Parse the substrate path — `@` then a slash-separated
        // identifier chain. Same shape as the prism path parser.
        input.parse::<Token![@]>()?;
        let mut segments: Vec<String> = Vec::new();
        loop {
            let seg: Ident = input.call(Ident::parse_any)?;
            segments.push(seg.to_string());
            if input.peek(Token![/]) {
                input.parse::<Token![/]>()?;
                continue;
            }
            break;
        }
        let path = format!("@{}", segments.join("/"));
        let final_seg = segments.last().expect("path has at least one segment");
        let struct_name = format_ident!("{}", to_pascal_case(final_seg));

        // Optional parenthesised extension list — `("spec")`,
        // `("mirror", "shard")`, or absent. The substrate's grammar
        // form admits both `grammar @path { ... }` (no extension
        // claim) and `grammar @path("ext1", "ext2") { ... }`
        // (claiming the listed file extensions). Parse the list as a
        // comma-separated `LitStr` punctuation if present.
        let mut extensions: Vec<String> = Vec::new();
        if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let lits =
                Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?;
            extensions = lits.iter().map(|l| l.value()).collect();
        }

        // Optional body block. Consume-but-don't-encode: the body's
        // `<op> <keyword>` pairs are the substrate's keyword table at
        // the substrate altitude; the Rust-altitude floor witnesses
        // the path as address. Body encoding is forward-promised.
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            while !content.is_empty() {
                let _tok: proc_macro2::TokenTree = content.parse()?;
            }
        }

        Ok(SubstrateGrammarDecl {
            path,
            struct_name,
            extensions,
        })
    }
}

/// The substrate-`type` declaration carrier — what the macro parses
/// from its input token stream when the leading keyword is `type`.
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
        // surface form.
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

/// The shim's expansion — substrate `type` / `prism` / `action`
/// declaration → Rust item. Pure function of the input TokenStream;
/// deterministic (the OID functionality law). Dispatches per the
/// substrate's `declaration_kind` (at `@code/metalogue`).
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let decl = match syn::parse2::<SubstrateDecl>(input) {
        Ok(d) => d,
        Err(e) => return e.to_compile_error(),
    };
    match decl {
        SubstrateDecl::Type(t) => emit_type(&t),
        SubstrateDecl::Prism(p) => emit_prism(&p),
        SubstrateDecl::Action(a) => emit_action(&a),
        SubstrateDecl::Grammar(g) => emit_grammar(&g),
    }
}

/// Emit a Rust `pub fn name(args) -> ret { todo!() }` per spec §4.1.3.
/// The three classify sub-cases (substrate / boundary / partial) all
/// share this floor at T25; per-case body fill-in is forward-promised
/// to T25.5+ (consumer-pull). The typed signature is the load-bearing
/// claim of T25 — the shim is signature-preserving, the OID law
/// discriminates on signatures.
///
/// The `#[allow(unused_variables)]` attribute on the emitted fn is
/// substrate-pull-correct: the `\` body's unresolved nature MEANS
/// the args are not yet bound to any computation (the realisation
/// discriminator's per-case body fill-in is what will bind them at
/// T25.5+). Without the allow, every emitted fn surfaces an
/// `unused_variables` warning at the call site, polluting downstream
/// build output. The semantic is: "args are typed-but-not-yet-used,
/// pending consumer-pull."
fn emit_action(decl: &SubstrateActionDecl) -> TokenStream {
    let name = &decl.name;
    let args = &decl.args;
    let ret = &decl.ret;
    quote! {
        #[allow(unused_variables)]
        pub fn #name(#args) -> #ret { todo!() }
    }
}

/// Emit a Rust unit struct with `#[derive(prism_core::DerivePrism)]`
/// and `#[oid("@path")]` per spec §4.1.2. The five-op block at the
/// substrate level becomes the Prism trait impl scaffolding via the
/// existing `#[derive(Prism)]` proc-macro; this shim's job is to
/// emit the unit struct shape that derive consumes.
fn emit_prism(decl: &SubstratePrismDecl) -> TokenStream {
    let struct_name = &decl.struct_name;
    let oid_lit = LitStr::new(&decl.path, proc_macro2::Span::call_site());
    quote! {
        #[derive(prism_core::DerivePrism)]
        #[oid(#oid_lit)]
        pub struct #struct_name;
    }
}

/// Emit a Rust unit struct with `#[derive(prism_core::DerivePrism)]`
/// and `#[oid("@path")]` per spec §4.1.4 (T26 cascade tick). The
/// extension list and body block at the substrate level are
/// parsed-but-not-encoded at the floor (same precedent as
/// `prism`'s five-op block): the substrate path IS the runtime
/// address, and the four laws hold structurally on the same shape
/// as `prism`.
///
/// The substrate differentiates `grammar @X` from `prism @X` by its
/// family-root declaration (the `.mirror` source); the Rust
/// altitude's `Addressable::oid` is structural (path-hashed).
/// Forward-promised consumer-pull ticks (T26.5+) can extend the
/// emission to carry the extensions list and the keyword table
/// when a downstream Rust-altitude consumer needs them.
fn emit_grammar(decl: &SubstrateGrammarDecl) -> TokenStream {
    let struct_name = &decl.struct_name;
    let oid_lit = LitStr::new(&decl.path, proc_macro2::Span::call_site());
    quote! {
        #[derive(prism_core::DerivePrism)]
        #[oid(#oid_lit)]
        pub struct #struct_name;
    }
}

fn emit_type(decl: &SubstrateTypeDecl) -> TokenStream {
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

    // === T24 (prism) unit tests ===

    #[test]
    fn bare_prism_emits_unit_struct_with_derive_and_oid() {
        let input = quote! { prism @parse };
        let emitted = expand(input);
        let parsed: syn::Result<syn::Item> = syn::parse2(emitted);
        let item = parsed.expect("emission must be a valid Rust item");
        let s = match item {
            syn::Item::Struct(s) => s,
            _ => panic!("prism-shaped substrate must emit a struct item"),
        };
        assert_eq!(s.ident.to_string(), "Parse");
        assert!(matches!(s.fields, syn::Fields::Unit));

        // The struct must carry #[derive(prism_core::DerivePrism)]
        // and #[oid("@parse")].
        let has_derive = s.attrs.iter().any(|a| a.path().is_ident("derive"));
        let has_oid = s.attrs.iter().any(|a| a.path().is_ident("oid"));
        assert!(has_derive, "emission must carry a #[derive(...)] attribute");
        assert!(has_oid, "emission must carry a #[oid(\"@path\")] attribute");
    }

    #[test]
    fn prism_with_five_op_block_emits_same_unit_struct() {
        let input = quote! {
            prism @kernel {
                focus kernel
                project kernel
                split kernel
                shift kernel
                settle kernel
            }
        };
        let emitted = expand(input);
        let parsed: syn::Result<syn::Item> = syn::parse2(emitted);
        let item = parsed.expect("emission must be a valid Rust item");
        let s = match item {
            syn::Item::Struct(s) => s,
            _ => panic!("prism-shaped substrate must emit a struct item"),
        };
        assert_eq!(s.ident.to_string(), "Kernel");
        assert!(matches!(s.fields, syn::Fields::Unit));
    }

    #[test]
    fn distinct_prism_paths_produce_distinct_emissions() {
        let a = expand(quote! { prism @observer }).to_string();
        let b = expand(quote! { prism @actor }).to_string();
        assert_ne!(
            a, b,
            "distinct substrate paths must produce distinct emissions"
        );
        assert!(a.contains("\"@observer\""));
        assert!(b.contains("\"@actor\""));
    }

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

    // === T26 (grammar) unit tests ===

    /// Bare `grammar @path { }` (no extension list, empty body) emits
    /// a unit struct with the substrate-pull-canonical shape:
    /// `#[derive(prism_core::DerivePrism)]` + `#[oid("@path")]` +
    /// `pub struct Path;`. Same floor as `prism`.
    #[test]
    fn bare_grammar_emits_unit_struct_with_derive_and_oid() {
        let input = quote! { grammar @parse_spec { } };
        let emitted = expand(input);
        let parsed: syn::Result<syn::Item> = syn::parse2(emitted);
        let item = parsed.expect("emission must be a valid Rust item");
        let s = match item {
            syn::Item::Struct(s) => s,
            _ => panic!("grammar-shaped substrate must emit a struct item"),
        };
        assert_eq!(s.ident.to_string(), "ParseSpec");
        assert!(matches!(s.fields, syn::Fields::Unit));

        // The struct must carry #[derive(prism_core::DerivePrism)]
        // and #[oid("@parse_spec")].
        let has_derive = s.attrs.iter().any(|a| a.path().is_ident("derive"));
        let has_oid = s.attrs.iter().any(|a| a.path().is_ident("oid"));
        assert!(has_derive, "emission must carry a #[derive(...)] attribute");
        assert!(has_oid, "emission must carry a #[oid(\"@path\")] attribute");
    }

    /// `grammar @path("ext1") { ... }` with a single extension claim
    /// and a body block emits the same unit struct as the bare form.
    /// The extensions and body are parsed-but-not-encoded (forward-
    /// promised per `[[feedback-craft-not-deliver]]`).
    #[test]
    fn grammar_with_single_extension_emits_same_unit_struct() {
        let input = quote! {
            grammar @mirror_spec("spec") {
                project in
                project out
            }
        };
        let emitted = expand(input);
        let parsed: syn::Result<syn::Item> = syn::parse2(emitted);
        let item = parsed.expect("emission must be a valid Rust item");
        let s = match item {
            syn::Item::Struct(s) => s,
            _ => panic!("grammar-shaped substrate must emit a struct item"),
        };
        assert_eq!(s.ident.to_string(), "MirrorSpec");
        assert!(matches!(s.fields, syn::Fields::Unit));
    }

    /// `grammar @path("ext1", "ext2", ...) { ... }` with multiple
    /// extension claims parses cleanly; the multi-extension form is
    /// the one used by substrate's `@code/mirror/grammar` (six
    /// extensions: mirror, spec, meta, glass, shard, shatter).
    #[test]
    fn grammar_with_multi_extension_emits_same_unit_struct() {
        let input = quote! {
            grammar @mirror_grammar("mirror", "spec", "meta") {
                focus prism
                focus grammar
            }
        };
        let emitted = expand(input);
        let parsed: syn::Result<syn::Item> = syn::parse2(emitted);
        let item = parsed.expect("emission must be a valid Rust item");
        let s = match item {
            syn::Item::Struct(s) => s,
            _ => panic!("grammar-shaped substrate must emit a struct item"),
        };
        assert_eq!(s.ident.to_string(), "MirrorGrammar");
        assert!(matches!(s.fields, syn::Fields::Unit));
    }

    /// Distinct substrate paths produce distinct emissions. The OID
    /// functionality law (spec §5 law 3) discriminates grammars on
    /// the substrate path, same shape as `distinct_prism_paths`.
    #[test]
    fn distinct_grammar_paths_produce_distinct_emissions() {
        let a = expand(quote! { grammar @first("a") { } }).to_string();
        let b = expand(quote! { grammar @second("b") { } }).to_string();
        assert_ne!(
            a, b,
            "distinct substrate paths must produce distinct emissions"
        );
        assert!(a.contains("\"@first\""));
        assert!(b.contains("\"@second\""));
    }

    /// The shim is a pure function of its input. Same substrate
    /// grammar declaration → byte-identical Rust emission. This
    /// witnesses the `oid_function(shim_grammar)` law at the
    /// TokenStream-string altitude.
    #[test]
    fn grammar_emission_is_deterministic() {
        let input = quote! { grammar @repeat("x") { focus a } };
        let a = expand(input.clone());
        let b = expand(input);
        assert_eq!(a.to_string(), b.to_string());
    }
}
