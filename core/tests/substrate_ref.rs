//! Tests for `prism_core::Ref` — substrate-reference primitive (the
//! `@`-prefixed nav-ref hoisted from mirror's crystallize.rs). Tick:
//! prism/transparency 🔴.

use std::collections::BTreeMap;

use prism_core::Ref;
use terni::{Loss, PropertyVerdict, Transparency};

// ---------------------------------------------------------------------------
// Validating constructor
// ---------------------------------------------------------------------------

#[test]
fn ref_accepts_at_prefixed_non_empty_no_whitespace() {
    let p = Ref::new("@kintsugi/fracture/rename").expect("valid ref");
    assert_eq!(p.as_str(), "@kintsugi/fracture/rename");
}

#[test]
fn ref_rejects_empty() {
    assert!(Ref::new("").is_err());
}

#[test]
fn ref_rejects_missing_at_prefix() {
    assert!(Ref::new("kintsugi/fracture").is_err());
}

#[test]
fn ref_rejects_whitespace() {
    assert!(Ref::new("@kintsugi fracture").is_err());
    assert!(Ref::new("@kintsugi\tfracture").is_err());
    assert!(Ref::new("@kintsugi\nfracture").is_err());
}

// ---------------------------------------------------------------------------
// Validator hardening (Seam I2) — reject control characters and the bare
// `@` prefix. `char::is_whitespace` catches ASCII whitespace but does NOT
// catch the C0 control range (NUL, ESC, DEL, etc.). The defence-in-depth
// rule is: a substrate ref is a path-shaped name; control bytes have no
// business in it.
// ---------------------------------------------------------------------------

#[test]
fn ref_rejects_null_byte() {
    assert!(Ref::new("@evil\u{0000}path").is_err());
}

#[test]
fn ref_rejects_terminal_escape() {
    // U+001B (ESC) is the lead byte for ANSI control sequences. A Ref
    // carrying "\x1b[2J" could clear a terminal if it ever reaches one.
    assert!(Ref::new("@evil\u{001B}[2J").is_err());
}

#[test]
fn ref_rejects_del() {
    assert!(Ref::new("@evil\u{007F}path").is_err());
}

#[test]
fn ref_rejects_other_control_chars() {
    // Spot-check a few other C0 codes that `is_whitespace` doesn't catch.
    for c in [0x01u32, 0x07, 0x0E, 0x1F] {
        let ch = char::from_u32(c).unwrap();
        let s = format!("@evil{ch}path");
        assert!(
            Ref::new(&s).is_err(),
            "expected rejection for U+{c:04X}, got Ok"
        );
    }
}

#[test]
fn ref_rejects_bare_at_prefix() {
    // `@` alone has no path — meaningless as a substrate location.
    assert!(Ref::new("@").is_err());
}

// ---------------------------------------------------------------------------
// Ord / Hash / Clone — usable as BTreeMap key
// ---------------------------------------------------------------------------

#[test]
fn ref_is_orderable_btreemap_key() {
    let a = Ref::new("@a").unwrap();
    let b = Ref::new("@b").unwrap();
    let c = Ref::new("@c").unwrap();
    let mut m: BTreeMap<Ref, &'static str> = BTreeMap::new();
    m.insert(c.clone(), "c");
    m.insert(a.clone(), "a");
    m.insert(b.clone(), "b");
    let keys: Vec<&str> = m.keys().map(|k| k.as_str()).collect();
    assert_eq!(keys, vec!["@a", "@b", "@c"], "BTreeMap iterates sorted");
}

#[test]
fn ref_clone_eq() {
    let a = Ref::new("@thing").unwrap();
    let b = a.clone();
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// Ref-Transparency interop — the structural payoff. Ref has no Default,
// Transparency<Ref> must instantiate anyway (P: Default bound dropped).
// ---------------------------------------------------------------------------

#[test]
fn transparency_over_ref_does_not_require_default() {
    let p = Ref::new("@quantize").unwrap();
    let t: Transparency<Ref> = Transparency::single(
        p.clone(),
        PropertyVerdict::Fail(terni::Diagnostic::new("non-integer state")),
    );
    let z: Transparency<Ref> = Transparency::zero();
    let cat: Transparency<Ref> = Transparency::total();
    assert!(t.is_opaque_at(&p));
    assert!(z.is_zero());
    assert!(cat.is_catastrophic());
}
