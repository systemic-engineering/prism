//! Certificate replay, slice R2: the Dark-2 report (docs/spec/echo.md
//! §10.4).
//!
//! From the lines a source reads, the holes Note v3.3.2 §6.7 H1 names:
//! `# L:C` for a line that doesn't read; the reader's own checks (`repeat
//! L:1`, `arity L:1`); the engine's (`arity p`, `safety r ?v`,
//! `declaration p <line>`), in A6's order. Then `@mirror/dark-order`'s
//! head (certificate §2.4; Note §6.4), `basis` (§6.4), and how much light
//! is left (H3), as far as R2 can tell without a run. The digests are
//! Taut's, read from the vector files.

use std::path::PathBuf;

use echo::address::{self, Address, Tau};
use echo::dark2::{self, Light, Report};
use echo::source;
use echo::value::{Atom, Value};

fn data(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/echo-vectors/mirror")
        .join(rel)
}

fn read(rel: &str) -> Vec<u8> {
    std::fs::read(data(rel)).expect(rel)
}

fn hex(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

/// A vector file's `<key>` field.
fn field(rel: &str, key: &str) -> String {
    String::from_utf8(read(rel))
        .expect("utf-8")
        .lines()
        .find_map(|l| {
            l.strip_prefix(key)
                .filter(|r| r.starts_with(' '))
                .map(|r| r.trim().to_string())
        })
        .unwrap_or_else(|| panic!("{rel}: no {key}"))
}

fn report_of(text: &[u8]) -> Report {
    dark2::report(&source::read(text))
}

fn report_file(rel: &str) -> Report {
    report_of(&read(rel))
}

fn texts(r: &Report) -> Vec<String> {
    r.holes.iter().map(|h| h.text()).collect()
}

fn certificate_atom(a: &Atom) -> String {
    hex(&address::atom(&Tau::mirror_certificate(), a))
}

// -------------------------------------------------- the holes, in order

/// Taut's 48 / 55: two lines that don't read, a repeated rule id, an
/// arity and a safety, in A6's order (H1 v3.0.1): `#` by offset, then the
/// reader's, then the engine's. Exactly 55's record's hole texts.
#[test]
fn vector_48_holes_in_a6_order() {
    let r = report_file("48-source.pact-text");
    assert_eq!(
        texts(&r),
        ["# 2:6", "# 7:11", "repeat 6:1", "arity q", "safety s ?y"]
    );
    let record = String::from_utf8(read("55-record-dark2-strata.record")).expect("utf-8");
    let pinned: Vec<String> = record
        .lines()
        .filter_map(|l| l.strip_prefix("prefix     hole        "))
        .map(|l| l.strip_suffix(" #<said>").expect("said").to_string())
        .collect();
    assert_eq!(texts(&r), pinned);
}

/// 48a: the performance's one `dark` atom is the head of the merged
/// order (2:6, 6:1, 7:11, then the engine's), not 2:1 and not 7:11.
#[test]
fn vector_48a_the_head() {
    let r = report_file("48-source.pact-text");
    let head = r.head().expect("a head");
    assert_eq!(
        head,
        Atom {
            pred: "dark".into(),
            args: vec![
                Value::Nat(2),
                Value::Sym("at".into()),
                Value::Nat(2),
                Value::Nat(6)
            ]
        }
    );
    assert_eq!(
        certificate_atom(&head),
        field("48a-dark-head.vector", "blake3")
    );
}

/// 48b and 48's song: basis = the unread lines {2, 7} (6 read: its fault is
/// the reader's), `unread 2, 7`, the atom `basis(nat 2, nat 7)` at
/// `@song/interval`; and the run rendering of {2, 3, 4, 7, 9, 10}.
#[test]
fn vector_48b_basis() {
    let r = report_file("48-source.pact-text");
    assert_eq!(r.basis, [2, 7]);
    assert_eq!(dark2::basis_text(&r.basis).as_deref(), Some("unread 2, 7"));
    assert_eq!(
        hex(&address::atom(
            &Tau::song_interval(),
            &dark2::basis_atom(&r.basis)
        )),
        field("48b-basis-atom.vector", "blake3")
    );
    assert_eq!(
        dark2::basis_text(&[2, 3, 4, 7, 9, 10]).as_deref(),
        Some("unread 2-4, 7, 9-10")
    );
    assert_eq!(dark2::basis_text(&[]), None);
    assert_eq!(dark2::basis_text(&[7]).as_deref(), Some("unread 7"));
}

/// Taut's 22 and 24: the performance's `dark` child holds the head alone,
/// `fragment(@mirror/certificate, "dark", [head])`.
#[test]
fn vectors_22f_24f_the_dark_child() {
    for (src, vector, text) in [
        ("22-source.pact-text", "22f-dark.vector", "arity q"),
        ("24-source.pact-text", "24f-dark.vector", "repeat 2:1"),
    ] {
        let r = report_file(src);
        assert_eq!(texts(&r), [text], "{src}");
        assert_eq!(r.light, Light::Dark, "{src}");
        let head = address::atom(&Tau::mirror_certificate(), &r.head().expect("head"));
        assert_eq!(
            hex(&address::fragment(
                &Tau::mirror_certificate(),
                "dark",
                &[head]
            )),
            field(vector, "blake3"),
            "{src}"
        );
    }
}

/// H5: a hole's atom is `hole(sym …)`, its text split at its first two
/// spaces, at `@mirror/certificate`: 77, 81, 73/74 (two declarations, and
/// 75 their holes child), 90, 91.
#[test]
fn hole_atoms_and_the_holes_child() {
    let r = report_file("77-source.pact-text");
    assert_eq!(texts(&r), ["# 2:6"]);
    assert_eq!(
        certificate_atom(&r.holes[0].atom()),
        field("77-hole-atom-hash.vector", "blake3")
    );
    let r = report_file("81-source.pact-text");
    assert_eq!(texts(&r), ["# 3:6"]);
    assert_eq!(
        certificate_atom(&r.holes[0].atom()),
        field("81-hole-atom-hash-3.vector", "blake3")
    );
    let r = report_file("71-source.pact-text");
    assert_eq!(
        texts(&r),
        [
            "declaration p functional p(2) -> 1",
            "declaration p functional p(3) -> 1"
        ]
    );
    let atoms: Vec<Address> = r
        .holes
        .iter()
        .map(|h| address::atom(&Tau::mirror_certificate(), &h.atom()))
        .collect();
    assert_eq!(
        hex(&atoms[0]),
        field("73-hole-atom-declaration-p2.vector", "blake3")
    );
    assert_eq!(
        hex(&atoms[1]),
        field("74-hole-atom-declaration-p3.vector", "blake3")
    );
    assert_eq!(
        hex(&address::set(&Tau::mirror_certificate(), "holes", &atoms).expect("a set")),
        field("75-holes-child-declarations.vector", "blake3")
    );
    for (src, vector, text) in [
        (
            "90-source.pact-text",
            "90-hole-atom-rule-3.vector",
            "declaration f functional f(1) -> 3",
        ),
        (
            "91-source.pact-text",
            "91-hole-atom-rule-5.vector",
            "declaration f functional f(1, 1) -> 2",
        ),
    ] {
        let r = report_file(src);
        assert_eq!(texts(&r), [text], "{src}");
        assert_eq!(
            certificate_atom(&r.holes[0].atom()),
            field(vector, "blake3"),
            "{src}"
        );
    }
}

/// 71c: a head that is a declaration hole names the predicate only.
#[test]
fn vector_71c_a_declaration_head() {
    let r = report_of(b"functional p(1) -> 2\n");
    assert_eq!(texts(&r), ["declaration p functional p(1) -> 2"]);
    assert_eq!(r.light, Light::Dark);
    assert_eq!(
        certificate_atom(&r.head().expect("head")),
        field("71c-dark-head-declaration.vector", "blake3")
    );
}

// ------------------------------------------------------ reader checks

/// A repeated rule id is a hole at each later line, column 1 (§3.1.1 J,
/// J′), and leaves out every rule with that id (H1). Its atoms still count
/// for arity (G7).
#[test]
fn repeated_rule_ids() {
    let r = report_of(b"rule a p requires q\nfact q\nrule a p requires r\nrule a p requires s\n");
    assert_eq!(texts(&r), ["repeat 3:1", "repeat 4:1"]);
    let r = report_of(b"rule a p requires q\nrule a p(?x) requires q(?x)\n");
    assert_eq!(texts(&r), ["repeat 2:1", "arity p", "arity q"]);
}

/// M4, the checker's reading (a) for both questions: a query is checked
/// beside a grammar failure, and it agrees if its arity is any of the
/// program's arities for its predicate. Seam's P8: `arity 1:1` beside
/// `# 4:6`, and the merged head is 1:1.
#[test]
fn query_arity() {
    let r = report_file("fixtures/p8-swapped/source.pact");
    assert_eq!(texts(&r), ["# 4:6", "arity 1:1"]);
    assert_eq!(
        r.head(),
        Some(Atom {
            pred: "dark".into(),
            args: vec![
                Value::Nat(2),
                Value::Sym("at".into()),
                Value::Nat(1),
                Value::Nat(1)
            ]
        })
    );
    // A predicate with two arities: the query matches one, so only `arity p`.
    let r = report_of(b"fact p(a)\nfact p(a, b)\nquery p(a, b)\n");
    assert_eq!(texts(&r), ["arity p"]);
    // A query on a predicate the program never uses.
    let r = report_of(b"fact p(a)\nquery z(a)\n");
    assert_eq!(texts(&r), ["arity 2:1"]);
}

// ------------------------------------------------------ engine checks

/// Safety: every variable of the head or of a negative literal must occur
/// in a positive literal. One hole per variable, after repeats are left
/// out (H1 v3.0.1), and beside an arity hole on the same rule.
#[test]
fn safety() {
    let r = report_of(b"rule s r(?y) requires not t(?y)\nrule u w(?a, ?b) requires v(?c)\n");
    assert_eq!(texts(&r), ["safety s ?y", "safety u ?a", "safety u ?b"]);
    let r = report_file("fixtures/pos-dark2-engine/source.pact");
    assert_eq!(texts(&r), ["arity q", "safety s ?x"]);
    let r = report_of(b"rule a p(?x) requires q\nrule a p requires q\n");
    assert_eq!(texts(&r), ["repeat 2:1", "arity p"]);
}

/// Note H7 v3.2.5: a declaration is ill-formed when its predicate has no
/// arity, a position is outside `1 … n`, or a key position repeats; a
/// predicate with two arities speaks as `arity p` alone; a trivial
/// declaration (value among the keys) is no hole (v3.2.7).
#[test]
fn declarations() {
    let r = report_of(b"fact f(k, v)\nfunctional f(1) -> 1\nfunctional f(2, 1) -> 2\n");
    assert!(r.holes.is_empty(), "{:?}", texts(&r));
    let r = report_of(
        b"fact f(k, v)\nfunctional f(1) -> 3\nfunctional f(1, 1) -> 2\nfunctional g(1) -> 2\nfunctional f(1) -> 3\n",
    );
    assert_eq!(
        texts(&r),
        [
            "declaration f functional f(1) -> 3",
            "declaration f functional f(1, 1) -> 2",
            "declaration g functional g(1) -> 2"
        ]
    );
    let r = report_of(b"fact p(a)\nfact p(a, b)\nfunctional p(1) -> 3\n");
    assert_eq!(texts(&r), ["arity p"]);
}

// --------------------------------------------------- the dark order

/// Certificate §2.4: the reader's faults merged by least byte offset, then
/// the engine's, arity before safety before declarations. CT6 /
/// pos-dark2-reader: 3:14, not the TAB at 5:5.
#[test]
fn the_head_is_the_least_offset_then_the_engine_order() {
    let head = |text: &[u8]| -> String {
        let a = report_of(text).head().expect("head");
        a.args
            .iter()
            .map(|v| match v {
                Value::Nat(n) => n.to_string(),
                Value::Sym(s) => s.clone(),
                other => panic!("{other:?}"),
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(
        head(&read("fixtures/pos-dark2-reader/source.pact")),
        "2 at 3 14"
    );
    assert_eq!(
        head(&read("fixtures/pos-dark2-engine/source.pact")),
        "2 engine arity q"
    );
    assert_eq!(
        head(b"rule s r(?y) requires not t(?y)\nfunctional z(1) -> 2\n"),
        "2 engine safety s ?y"
    );
    assert_eq!(
        head(b"functional z(1) -> 2\nfunctional a(1) -> 2\n"),
        "2 engine declaration a"
    );
    assert_eq!(report_file("fixtures/pos-v01/source.pact").head(), None);
}

// ------------------------------------------------------------ light

/// H3 "no light at all", as far as R2 can tell without a run:
/// - no hole: nothing to tell;
/// - `Dark`: nothing is left to run that could give light (no fact, every
///   rule needs a positive premise, no `not`, no query that could be a
///   light falsehood);
/// - `Lit`: a fact is left whose predicate no hole and no declaration can
///   remove (H1's `D`, the general rule);
/// - `Open`: anything else, and every `not` beside a hole that adds (H1:
///   `unknown shadow under W` is R5's).
#[test]
fn how_much_light_is_left() {
    let cases: [(&str, Light); 16] = [
        ("fixtures/pos-v01/source.pact", Light::NoHole),
        ("48-source.pact-text", Light::Dark),
        ("22-source.pact-text", Light::Dark),
        ("24-source.pact-text", Light::Dark),
        ("fixtures/pos-dark2-engine/source.pact", Light::Dark),
        ("fixtures/pos-dark2-reader/source.pact", Light::Dark),
        ("fixtures/a1-record-dark-reader/source.pact", Light::Dark),
        ("fixtures/p8-swapped/source.pact", Light::Lit),
        ("fixtures/ct-a6-unsafe-with-model/source.pact", Light::Lit),
        ("71-source.pact-text", Light::Lit),
        ("77-source.pact-text", Light::Lit),
        ("80-source.pact-text", Light::Lit),
        ("81-source.pact-text", Light::Lit),
        ("90-source.pact-text", Light::Lit),
        ("91-source.pact-text", Light::Lit),
        ("fixtures/typed-corpus-unheard/source.pact", Light::NoHole),
    ];
    for (src, light) in cases {
        assert_eq!(report_file(src).light, light, "{src}");
    }
    let open: [&[u8]; 5] = [
        // `not` beside a `#` hole: W with an additive hole (H1, R5).
        b"fact p(a)\nrule r q(?x) requires p(?x), not s(?x)\nfact X\n",
        // A query that may be a light falsehood, and no `#` hole.
        b"rule r p(?x) requires q(?x)\nquery p(a)\nfunctional z(1) -> 2\n",
        // The only facts are on a predicate `#` can remove (arity >= 2).
        b"fact e(a, b)\nfact X\n",
        // ... or a declared one.
        b"fact o(k, a)\nfunctional o(1) -> 2\nfact X\n",
        // A rule with no positive premise fires.
        b"rule r p requires not q\nfact X\n",
    ];
    for text in open {
        assert_eq!(
            report_of(text).light,
            Light::Open,
            "{:?}",
            String::from_utf8_lossy(text)
        );
    }
    // `not` only in a line left out, and nothing left: dark (Taut's 48).
    assert_eq!(
        report_of(b"rule s r(?y) requires not t(?y)\nfact X\n").light,
        Light::Dark
    );
    // A `#` hole and a query, nothing else: R+ = *, so no light falsehood.
    assert_eq!(
        report_of(b"rule r p requires q\nquery p\nfact X\n").light,
        Light::Dark
    );
}
