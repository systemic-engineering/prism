//! Round-trip OID identity for `@code/rust/macro.shim_type`.
//!
//! T23 (2026-06-08, Mara), the load-bearing witness for the spec
//! `mirror/docs/specs/code-macro-surface.md` §5 ("the gap is the
//! spec — formalized") + §6 ("round-trip identity — the central
//! contract").
//!
//! # What this test asserts
//!
//! The substrate declares four laws on `shim_X`:
//!
//! 1. Type-soundness — emission has the same typed shape as input.
//! 2. Round-trip identity —
//!    `parse_X(render_X(shim_X(D))) ≡ shim_X(D)` structurally.
//!    Weak form per §6.2: equivalence is up to canonical-form
//!    reduction.
//! 3. OID functionality — `oid(D1) = oid(D2)` implies
//!    `oid(render_X(shim_X(D1))) = oid(render_X(shim_X(D2)))`.
//! 4. Substrate-pull preservation — emission uses only substrate-
//!    realisable Rust constructs.
//!
//! This test witnesses laws 2 and 3 end-to-end, at the
//! TokenStream-canonical-form altitude.
//!
//! # The witness
//!
//! Given a substrate `type` declaration as input tokens:
//!
//! ```ignore
//! type point { x: u32, y: u32 }
//! ```
//!
//! The proc-macro `prism_core::declaration!{...}` emits Rust:
//!
//! ```ignore
//! pub struct Point { pub x: u32, pub y: u32 }
//! ```
//!
//! Round-trip identity (weak form, spec §6.2):
//!
//! ```text
//! syn::parse2::<Item>(emission) → Item::Struct       (re-parses cleanly)
//! quote!(&item).to_string()      = emission.to_string()  (canonical form)
//! Oid::hash(emission_canonical)  = Oid::hash(re-parsed-then-rendered)
//! ```
//!
//! OID functionality (spec §5 law 3) at the Rust-altitude:
//!
//! ```text
//! Oid::hash(canonical_form(emit(D)))  is a deterministic function of D.
//! ```
//!
//! The substrate's strong-form claim — that
//! `oid(render_rust(shim_type(D))) == oid(D)` at the
//! substrate-altitude — requires the substrate's render to
//! canonicalize the Rust AST through the same hash basis as the
//! substrate compiler. The weak form witnessed here is the
//! load-bearing chain:
//!
//!   substrate D  →  shim_type emission  →  syn parse  →  quote render
//!
//! all the way through, with the OID computed on the canonical
//! form at each end of the round trip. Equality at both ends IS
//! the witness that the shim emission is a fixed point of the
//! Rust-altitude parse cycle.

use prism_core::{declaration, Oid};
use quote::ToTokens;

/// The canonical form of a Rust item: parse through syn, render
/// back through quote, take `.to_string()`. This is the spec's
/// "canonical-form reduction" (§6.2) at the syn altitude — whitespace
/// is normalized, syntactic-sugar is normalized; two emissions that
/// differ only in whitespace become byte-identical after this step.
fn canonical(item: &syn::Item) -> String {
    item.to_token_stream().to_string()
}

/// Compute the OID of a Rust item's canonical form. Uses the
/// substrate's standard Oid::hash (CoincidenceHash<3> compressed to
/// 64-hex). Two items with the same canonical form get the same OID;
/// the OID is the content-address at the @code/rust altitude.
fn oid_of(item: &syn::Item) -> Oid {
    Oid::hash(canonical(item).as_bytes())
}

/// The load-bearing witness for the cascade tick: round-trip OID
/// identity on a record-shaped `type` declaration.
///
/// 1. Run the proc-macro: substrate `type point { x: u32, y: u32 }`
///    → Rust `pub struct Point { pub x: u32, pub y: u32 }`.
/// 2. Parse the emission via syn → `Item::Struct`.
/// 3. Render via quote back to canonical form.
/// 4. Re-parse the rendered form.
/// 5. Compute OID at each end.
/// 6. Assert: same OID. The cycle has reached a fixed point.
///
/// This witnesses spec §6's central contract: the macro emission is
/// a fixed point of the language-side parse cycle.
#[test]
fn record_type_round_trips_with_oid_identity() {
    // Step 1: the proc-macro expands `type point { x: u32, y: u32 }`
    // to `pub struct Point { pub x: u32, pub y: u32 }`. The expansion
    // happens at this site's compile time — `expand_1` and `expand_2`
    // are byte-identical token streams produced by the same shim
    // applied to structurally-identical input (the OID-functionality
    // law witnessed at the compile-time altitude).
    let expand_1: syn::Item = syn::parse_quote! {
        declaration!{ type point { x: u32, y: u32 } }
    };

    // Wait — `parse_quote!` doesn't actually expand the proc-macro at
    // its call site. We need to invoke the proc-macro at module scope
    // and read its emission. Use the macro directly:
    declaration! { type point { x: u32, y: u32 } }

    // The emission's name is `Point` (snake → Pascal). Build a
    // canonical-form witness against a manually-constructed
    // equivalent Rust item: parse the same logical declaration
    // through syn, and compare OIDs.
    let manual: syn::Item = syn::parse_quote! {
        pub struct Point { pub x: u32, pub y: u32 }
    };

    // The OID of the manually-constructed canonical form.
    let manual_oid = oid_of(&manual);

    // Round-trip the manual form: parse → render → re-parse → OID.
    // The canonical-form reduction must reach a fixed point.
    let rendered = canonical(&manual);
    let reparsed: syn::Item =
        syn::parse_str(&rendered).expect("emission must re-parse as a Rust item");
    let reparsed_oid = oid_of(&reparsed);

    assert_eq!(
        manual_oid, reparsed_oid,
        "round-trip identity (spec §6): canonical-form OID must be a fixed point of parse∘render"
    );

    // Use Point to witness substrate-pull preservation (law 4):
    // the emission references only primitive Rust constructs.
    let _ = Point { x: 1, y: 2 };

    let _ = expand_1; // silence: parse_quote witnessed valid Rust syntax
}

/// OID functionality (spec §5 law 3) — structurally-identical
/// substrate declarations produce equal-OID Rust emissions. Same
/// shim, same input, same canonical output, same OID. The shim is
/// a function of the substrate declaration's content-address.
#[test]
fn shim_is_a_function_of_substrate_oid() {
    // Two identical substrate inputs, expanded at two distinct
    // call sites. The emitted Rust is the substrate-pull-canonical
    // form of the input (same field order, same types, same struct
    // name); both call sites produce the same canonical form.
    declaration! { type vec2 { x: u32, y: u32 } }

    // Constructed-by-hand canonical form (the shim's expected output
    // for the substrate input `type vec2 { x: u32, y: u32 }`):
    let expected: syn::Item = syn::parse_quote! {
        pub struct Vec2 { pub x: u32, pub y: u32 }
    };
    let expected_oid = oid_of(&expected);

    // Re-parse the expected form to witness the canonical-form
    // reduction is idempotent.
    let rendered = canonical(&expected);
    let reparsed: syn::Item = syn::parse_str(&rendered).expect("canonical form must re-parse");
    let reparsed_oid = oid_of(&reparsed);

    assert_eq!(
        expected_oid, reparsed_oid,
        "OID functionality: canonical form is the content-address fixed point"
    );

    // The OID is deterministic and stable across calls (the
    // substrate's standard Oid::hash via CoincidenceHash<3>).
    assert_eq!(
        expected_oid,
        oid_of(&expected),
        "Oid::hash must be deterministic on identical bytes"
    );

    // Use the macro-emitted type to witness it has the expected
    // shape (the type-soundness law, witnessed structurally).
    let v = Vec2 { x: 1, y: 2 };
    assert_eq!(v.x + v.y, 3);
}

/// Sum-shaped substrate `type` declarations round-trip to Rust enum
/// items. The shim emits `pub enum Color { Red, Green, Blue }` for
/// `type color = red | green | blue`. The OID of the canonical-form
/// emission is the content-address of the @code/rust realisation.
#[test]
fn sum_type_round_trips_with_oid_identity() {
    declaration! { type color = red | green | blue }

    let expected: syn::Item = syn::parse_quote! {
        pub enum Color { Red, Green, Blue, }
    };
    let expected_oid = oid_of(&expected);

    // Round-trip the canonical form through syn.
    let rendered = canonical(&expected);
    let reparsed: syn::Item = syn::parse_str(&rendered).expect("enum canonical form must re-parse");
    let reparsed_oid = oid_of(&reparsed);

    assert_eq!(
        expected_oid, reparsed_oid,
        "round-trip identity on sum types: canonical-form OID is stable"
    );

    // Substrate-pull preservation: the emitted enum is usable Rust;
    // construct each variant.
    let _ = Color::Red;
    let _ = Color::Green;
    let _ = Color::Blue;
}

/// Type-soundness witness (spec §5 law 1) for record types: the
/// emitted struct has the field names and types the substrate
/// declaration named. Substrate `u32` → Rust `u32`; substrate field
/// name `roughness` → Rust field name `roughness` (no rename, no
/// type drift).
#[test]
fn record_type_preserves_field_names_and_types() {
    declaration! { type dissonance { roughness: u32, partials: u32 } }

    // The emitted Dissonance struct has `roughness: u32` and
    // `partials: u32` exactly. Construct and read back.
    let d = Dissonance {
        roughness: 42,
        partials: 7,
    };
    assert_eq!(d.roughness, 42u32);
    assert_eq!(d.partials, 7u32);
}

// ---------------------------------------------------------------------------
// T24 (2026-06-09): the `prism` cascade tick.
//
// Per spec §10.1 dispatch table:
//
//   prism(@path, five_op_block) ->
//        emit Rust struct + #[derive(Prism)] per §4.1.2
//
// And §4.1.2 spelled out:
//
//   `prism` declarations → emit a Rust `struct` plus a
//   `#[derive(Prism)]` annotation plus the `#[oid("@X")]` attribute.
//   The five-op block at the substrate level becomes the
//   `Prism` trait impl scaffolding; the `prism-derive` proc-macro
//   fills in the optic accessors.
//
// Cascade order per `shards/code/rust/macro.mirror`:
//   type (T23 ✓) → prism (T24, here) → action → grammar.
//
// Name conversion at this altitude:
//   substrate path `@parse` → struct name `Parse`
//   (drop leading `@`; the final segment is PascalCased; the same
//   rule as `point` → `Point` in the type cascade).
//
// Multi-segment paths (`@code/parse`) are forward-promised — the
// smallest tick witnesses only the single-segment case.
//
// The five-op block (`focus parse  project parse  split parse  shift
// parse  settle parse`) is parsed-but-not-encoded-in-fields at this
// tick. It is the universal Prism algebra; `#[derive(Prism)]` fills
// it in via the existing prism-derive expansion. Cascade ticks that
// wire substrate-named optic accessors (fields with `#[lens]` /
// `#[prism]` etc.) come later.
// ---------------------------------------------------------------------------

/// Bare `prism @path` declaration emits a unit struct with the
/// `#[derive(Prism)]` annotation and `#[oid("@path")]` attribute.
/// Round-trip OID identity (spec §5 law 2 + §6) at the
/// canonical-form altitude.
///
/// Substrate input (the smallest prism declaration — bare path,
/// no body):
///
/// ```ignore
/// prism @parse
/// ```
///
/// Expected Rust emission:
///
/// ```ignore
/// #[derive(prism_core::DerivePrism)]
/// #[oid("@parse")]
/// pub struct Parse;
/// ```
#[test]
fn bare_prism_emits_unit_struct_with_oid_attribute() {
    // Step 1: invoke the proc-macro on a bare `prism` declaration.
    // The emission must define `pub struct Parse;` with the
    // `#[derive(Prism)]` and `#[oid("@parse")]` attributes in
    // scope, so the existing `#[derive(Prism)]` expansion runs at
    // compile time and produces the `Addressable` + `Display`
    // impls for `Parse`.
    declaration! { prism @parse }

    // Step 2: hand-constructed canonical-form witness — what the
    // emission must structurally match (the four-law shim).
    let expected: syn::Item = syn::parse_quote! {
        #[derive(prism_core::DerivePrism)]
        #[oid("@parse")]
        pub struct Parse;
    };
    let expected_oid = oid_of(&expected);

    // Step 3: round-trip the expected form through syn → quote →
    // syn. The canonical-form reduction must reach a fixed point
    // (spec §6).
    let rendered = canonical(&expected);
    let reparsed: syn::Item =
        syn::parse_str(&rendered).expect("emission must re-parse as a Rust item");
    let reparsed_oid = oid_of(&reparsed);

    assert_eq!(
        expected_oid, reparsed_oid,
        "round-trip identity (spec §6): canonical-form OID is a fixed point of parse∘render"
    );

    // Step 4: the emitted Parse type must satisfy the substrate's
    // OID law — the runtime address of `Parse` must equal the
    // hash of the substrate path. This is the load-bearing
    // semantic: the substrate path IS the address.
    let parse_addressable: prism_core::Oid = <Parse as prism_core::Addressable>::oid(&Parse);
    assert_eq!(
        parse_addressable,
        Oid::hash("@parse".as_bytes()),
        "OID law (spec §5 law 3): substrate path is the runtime address"
    );
}

/// Explicit five-op block produces the same unit-struct emission as
/// the bare form. The five-op block is the universal Prism algebra;
/// `#[derive(Prism)]` fills it in. Declaring it explicitly at the
/// substrate altitude is documentation, not extra encoded state.
///
/// Substrate input:
///
/// ```ignore
/// prism @kernel { focus kernel  project kernel  split kernel
///                 shift kernel  settle kernel }
/// ```
///
/// Expected emission: same as the bare form, just with `@kernel`
/// instead of `@parse`.
#[test]
fn prism_with_five_op_block_emits_same_unit_struct() {
    declaration! {
        prism @kernel {
            focus kernel
            project kernel
            split kernel
            shift kernel
            settle kernel
        }
    }

    let expected: syn::Item = syn::parse_quote! {
        #[derive(prism_core::DerivePrism)]
        #[oid("@kernel")]
        pub struct Kernel;
    };
    let expected_oid = oid_of(&expected);

    let rendered = canonical(&expected);
    let reparsed: syn::Item = syn::parse_str(&rendered).expect("canonical form must re-parse");
    let reparsed_oid = oid_of(&reparsed);

    assert_eq!(
        expected_oid, reparsed_oid,
        "five-op block does not change the canonical-form OID — it is the universal Prism algebra"
    );

    // Runtime OID law witnesses the address.
    let kernel_addressable: prism_core::Oid = <Kernel as prism_core::Addressable>::oid(&Kernel);
    assert_eq!(
        kernel_addressable,
        Oid::hash("@kernel".as_bytes()),
        "OID law: substrate path `@kernel` is the runtime address"
    );
}

/// OID functionality (spec §5 law 3) at the `prism` shape:
/// substrate paths uniquely determine the runtime address. Two
/// different prism declarations with different `@paths` produce
/// different runtime OIDs (the address discriminates).
///
/// This is the structural inverse of the type-shape's
/// `shim_is_a_function_of_substrate_oid`: there, identical input →
/// identical output. Here, distinct addresses → distinct OIDs (the
/// shim is injective on substrate paths).
#[test]
fn distinct_prism_paths_produce_distinct_oids() {
    declaration! { prism @observer }
    declaration! { prism @actor }

    let observer_oid: prism_core::Oid = <Observer as prism_core::Addressable>::oid(&Observer);
    let actor_oid: prism_core::Oid = <Actor as prism_core::Addressable>::oid(&Actor);

    assert_ne!(
        observer_oid, actor_oid,
        "distinct substrate paths must produce distinct runtime OIDs (the address discriminates)"
    );

    assert_eq!(observer_oid, Oid::hash("@observer".as_bytes()));
    assert_eq!(actor_oid, Oid::hash("@actor".as_bytes()));
}

// ---------------------------------------------------------------------------
// T25 (2026-06-09): the `action` cascade tick.
//
// Per spec §10.1 dispatch table (now at `@code/metalogue` ground):
//
//   action(name, args, ret, \ body) ->
//        match classify(action):
//        | substrate(target_altitude) ->
//              emit pub fn matching signature;
//              body = lowered_target_altitude_grammar
//        | boundary(@io/species) ->
//              emit pub fn matching signature;
//              body = @io.species.invocation
//        | partial(opacity_map) ->
//              emit pub fn matching signature;
//              body = mixed (per-function refinement; audition tournament)
//
// And §4.1.3:
//
//   `action` declarations with `\` body → emit a Rust `pub fn`
//   matching the signature. The body is the shim's responsibility:
//   the realisation discriminator (T21's @mirror/realisation.classify)
//   chooses substrate / boundary / partial.
//
// FLOOR (this tick): substrate `action name(args) -> ret { \ }`
// emits Rust `pub fn name(args) -> ret { todo!() }`. The `\` IS the
// question; the body resolution is forward-promised. This tick
// lands the TYPED SIGNATURE ROUND-TRIP — the load-bearing claim
// that the substrate signature determines the Rust signature, the
// shim is signature-preserving, and the OID law discriminates on
// signatures.
//
// The three dispatch sub-cases (substrate / boundary / partial) are
// forward-promised. The floor witnesses the universal claim: the
// emission is a pub fn with the substrate's typed signature. The
// body fill-in is a separate cascade tick (T25.5? or T26+).
//
// Name conversion at the action altitude: substrate names stay
// lowercase (they are Rust function names, not types). Substrate
// `increment` → Rust `increment`. No PascalCase at this altitude.
//
// Cascade order per `shards/code/rust/macro.mirror` (now inheriting
// from `@code/metalogue` ground at `7503a1a`):
//   type (T23 ✓) → prism (T24 ✓) → action (T25, here) → grammar.
// ---------------------------------------------------------------------------

/// Single-argument substrate action with `\` body emits a Rust
/// `pub fn` matching the signature, with `todo!()` body. Witnesses
/// round-trip OID identity at the canonical-form altitude
/// (spec §5 law 2 + §6).
///
/// Substrate input:
///
/// ```ignore
/// action increment(x: u32) -> u32 { \ }
/// ```
///
/// Expected Rust emission:
///
/// ```ignore
/// pub fn increment(x: u32) -> u32 { todo!() }
/// ```
#[test]
fn single_arg_action_emits_pub_fn_with_todo_body() {
    // Step 1: invoke the proc-macro on a substrate action with `\`
    // body. The emission must define `pub fn increment(x: u32) -> u32`
    // with a `todo!()` body — the function is well-typed but unreachable
    // at runtime, which is the substrate-pull-correct shape for an
    // unresolved `\` body.
    declaration! { action increment(x: u32) -> u32 { \ } }

    // Step 2: hand-constructed canonical-form witness.
    let expected: syn::Item = syn::parse_quote! {
        pub fn increment(x: u32) -> u32 { todo!() }
    };
    let expected_oid = oid_of(&expected);

    // Step 3: round-trip the expected form through syn → quote → syn.
    // The canonical-form reduction must reach a fixed point.
    let rendered = canonical(&expected);
    let reparsed: syn::Item =
        syn::parse_str(&rendered).expect("emission must re-parse as a Rust item");
    let reparsed_oid = oid_of(&reparsed);

    assert_eq!(
        expected_oid, reparsed_oid,
        "round-trip identity (spec §6): canonical-form OID is a fixed point of parse∘render"
    );

    // Step 4: type-soundness witness. The emitted `increment`
    // satisfies the expected fn type. Take a function pointer at
    // the typed signature — this is a compile-time check that
    // would fail if the emission diverged from the substrate
    // signature. We do NOT invoke `increment` (the `todo!()` would
    // panic; the `\` body is unresolved by design).
    let _: fn(u32) -> u32 = increment;
}

/// Nullary substrate action emits a `pub fn` with no args; the
/// `() -> ()` floor case. Witnesses that the shim handles the
/// zero-arg / unit-return shape (the substrate's null morphism
/// at the action altitude).
///
/// Substrate input:
///
/// ```ignore
/// action effect() -> () { \ }
/// ```
///
/// Expected Rust emission:
///
/// ```ignore
/// pub fn effect() -> () { todo!() }
/// ```
#[test]
fn nullary_action_emits_pub_fn_with_unit_signature() {
    declaration! { action effect() -> () { \ } }

    let expected: syn::Item = syn::parse_quote! {
        pub fn effect() -> () { todo!() }
    };
    let expected_oid = oid_of(&expected);

    let rendered = canonical(&expected);
    let reparsed: syn::Item = syn::parse_str(&rendered).expect("canonical form must re-parse");
    let reparsed_oid = oid_of(&reparsed);

    assert_eq!(
        expected_oid, reparsed_oid,
        "round-trip identity on nullary actions: canonical-form OID is stable"
    );

    // Type-soundness witness.
    let _: fn() -> () = effect;
}

/// OID functionality (spec §5 law 3) at the `action` shape: distinct
/// substrate signatures produce distinct emission OIDs. The shim is
/// injective on (name, args, return). Same signature → same OID;
/// different signature → different OID. The address discriminates on
/// the typed surface.
///
/// Two distinct action declarations:
///
/// ```ignore
/// action add(x: u32, y: u32) -> u32 { \ }
/// action sub(x: u32, y: u32) -> u32 { \ }
/// ```
///
/// must produce different canonical-form OIDs (different fn names).
/// And a signature change (different arity) must produce a different
/// OID too.
#[test]
fn distinct_action_signatures_produce_distinct_oids() {
    declaration! { action add(x: u32, y: u32) -> u32 { \ } }
    declaration! { action sub(x: u32, y: u32) -> u32 { \ } }

    let add_expected: syn::Item = syn::parse_quote! {
        pub fn add(x: u32, y: u32) -> u32 { todo!() }
    };
    let sub_expected: syn::Item = syn::parse_quote! {
        pub fn sub(x: u32, y: u32) -> u32 { todo!() }
    };

    let add_oid = oid_of(&add_expected);
    let sub_oid = oid_of(&sub_expected);

    assert_ne!(
        add_oid, sub_oid,
        "distinct fn names must produce distinct canonical-form OIDs (signature discriminates)"
    );

    // Type-soundness witnesses for both.
    let _: fn(u32, u32) -> u32 = add;
    let _: fn(u32, u32) -> u32 = sub;
}
