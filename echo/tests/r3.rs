//! Certificate replay, slice R3: P, F and the interval (docs/spec/echo.md
//! §10.6).
//!
//! Check 1 (certificate §4.1): echo recomputes the program address P and
//! the facts address F from the source, under the address law `H`
//! (certificate §3.1, §3.1.1 A, B, G), and compares them with the
//! performance header's `program` and `facts`. Note §6.7 H5 says which
//! header is right: every line that read for a performance that isn't
//! `unknown`; for an `unknown` one, `none` after a line that doesn't read
//! or a reader fault, P and F after an engine fault (§3.1.1 F). A
//! difference is `interval source source <claimed> <recomputed> about none
//! at none 0 by 1` (Note §6.2).
//!
//! Lines print by stratum in its enumeration's order, then by bytes, and
//! `dissonant <n>` counts the lines of every stratum replayed so far. What
//! R3 agrees with answers `unknown unreplayed model`, the next stratum.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use echo::address::{self, Address, Tau};
use echo::certificate;
use echo::dark2;
use echo::interval::{self, Interval, Side};
use echo::program::{self, Expected};
use echo::replay::{self, Answer, Inputs, Verdict, BUDGET};
use echo::source::{self, Literal, Pattern, Record, Term};
use echo::value::{Atom, Value};

fn data(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/echo-vectors/mirror")
        .join(rel)
}

fn read(rel: &str) -> Vec<u8> {
    fs::read(data(rel)).expect(rel)
}

fn text(rel: &str) -> String {
    String::from_utf8(read(rel)).expect("utf-8")
}

fn hex(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(h: &str) -> Address {
    let mut a = [0u8; 32];
    for (i, b) in a.iter_mut().enumerate() {
        *b = u8::from_str_radix(&h[2 * i..2 * i + 2], 16).expect("hex");
    }
    a
}

/// A vector file's field: the rest of the first line with that key.
fn field(rel: &str, key: &str) -> String {
    text(rel)
        .lines()
        .find_map(|l| {
            l.strip_prefix(key)
                .filter(|r| r.starts_with(' '))
                .map(|r| r.trim().to_string())
        })
        .expect(key)
}

struct Triple {
    source: Vec<u8>,
    certificate: Vec<u8>,
    record: Vec<u8>,
}

/// A fixture's inputs, its `expect` redirections followed, and the tune's
/// answer.
fn fixture(name: &str) -> (Triple, String) {
    let expect = text(&format!("fixtures/{name}/expect"));
    let path = |key: &str, default: &str| -> String {
        let given = expect
            .lines()
            .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(' ')))
            .map(str::trim);
        match given {
            Some(p) if p.starts_with("../../vectors/") => p["../../vectors/".len()..].to_string(),
            Some(p) => format!("fixtures/{name}/{p}"),
            None => format!("fixtures/{name}/{default}"),
        }
    };
    let verdict = expect
        .lines()
        .find_map(|l| l.strip_prefix("expect "))
        .expect("an expect line")
        .to_string();
    (
        Triple {
            source: read(&path("source", "source.pact")),
            certificate: read(&path("certificate", "certificate")),
            record: read(&path("record", "record")),
        },
        verdict,
    )
}

fn fixtures() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(data("fixtures"))
        .expect("fixtures")
        .map(|e| e.expect("entry").file_name().into_string().expect("name"))
        .collect();
    names.sort();
    names
}

fn run(t: &Triple) -> Answer {
    replay::replay(
        &Inputs {
            source: Some(&t.source),
            certificate: Some(&t.certificate),
            record: Some(&t.record),
            score: None,
        },
        BUDGET,
    )
}

fn lines(a: &Answer) -> Vec<String> {
    a.print().lines().map(str::to_string).collect()
}

fn intervals(a: &Answer) -> Vec<String> {
    lines(a)[3..].to_vec()
}

/// Replace every line starting with `prefix` by `with` (nothing if empty).
fn replace(bytes: &[u8], prefix: &str, with: &str) -> Vec<u8> {
    String::from_utf8(bytes.to_vec())
        .expect("utf-8")
        .lines()
        .map(|l| {
            if l.starts_with(prefix) {
                if with.is_empty() {
                    String::new()
                } else {
                    format!("{with}\n")
                }
            } else {
                format!("{l}\n")
            }
        })
        .collect::<String>()
        .into_bytes()
}

const NEXT: &str = "unknown unreplayed model";

fn source_line(left: &str, right: &str) -> String {
    format!("interval source source {left} {right} about none at none 0 by 1")
}

// ------------------------------------------------- P and F, from Taut's data

/// The source a `NNa-program` vector names in its `note source:` line: the
/// records joined by ` | `, one per line.
fn noted_source(nn: &str) -> Vec<u8> {
    let note = field(&format!("{nn}a-program.vector"), "note");
    let records = note.strip_prefix("source: ").expect("a source note");
    let mut s: String = records.split(" | ").map(|r| format!("{r}\n")).collect();
    if s.is_empty() {
        s.push('\n');
    }
    s.into_bytes()
}

fn vector_numbers(suffix: &str) -> Vec<String> {
    let mut nns: Vec<String> = fs::read_dir(data(""))
        .expect("vectors")
        .filter_map(|e| {
            let name = e.expect("entry").file_name().into_string().expect("name");
            name.strip_suffix(suffix).map(str::to_string)
        })
        .collect();
    nns.sort();
    nns
}

/// Taut's `NNa-program` vectors (16): each canonical line's atom, from
/// echo's own re-print of the noted source, and the program fragment's
/// digest (certificate §3.1, §3.1.1 A, B, G).
#[test]
fn the_program_vectors() {
    let nns = vector_numbers("a-program.vector");
    assert_eq!(nns.len(), 16);
    for nn in &nns {
        let rel = format!("{nn}a-program.vector");
        let src = source::read(&noted_source(nn));
        let mine: BTreeSet<String> = src
            .lines
            .iter()
            .filter_map(|l| program::canonical(&l.record))
            .collect();
        let theirs: BTreeSet<String> = text(&rel)
            .lines()
            .filter_map(|l| l.strip_prefix("line       \""))
            .map(|l| {
                let (line, addr) = l.rsplit_once("\" ").expect("line and addr");
                let tau = Tau::root("@pact/program").expect("tau");
                let keyword = line.split(' ').next().expect("keyword");
                let atom = Atom {
                    pred: keyword.into(),
                    args: vec![Value::Sym(line.into())],
                };
                assert_eq!(hex(&address::atom(&tau, &atom)), addr, "{rel}: {line}");
                line.to_string()
            })
            .collect();
        assert_eq!(mine, theirs, "{rel}");
        assert_eq!(hex(&program::program(&src)), field(&rel, "blake3"), "{rel}");
    }
}

/// Taut's `NNb-facts` vectors (16), over the same noted sources.
#[test]
fn the_facts_vectors() {
    let nns = vector_numbers("b-facts.vector");
    assert_eq!(nns.len(), 16);
    for nn in &nns {
        let rel = format!("{nn}b-facts.vector");
        let src = source::read(&noted_source(nn));
        assert_eq!(hex(&program::facts(&src)), field(&rel, "blake3"), "{rel}");
    }
}

/// Taut's `NNg-header` vectors (17): the header's `program` and `facts`
/// arguments are what echo expects of the source (H5; §3.1.1 F: 22 is an
/// engine Dark, so P and F; 24 a reader Dark, so `none`), and the header
/// atom made with echo's P and F has the vector's digest.
///
/// 95 and 97 are W sources beside a line that doesn't read (`A ≠ ∅`):
/// `unknown` unless a certain paradox exists in the positive core (H3
/// v3.3), which is a run, R4's. R3 can't tell which header they call for;
/// their vectors are the paradox's, P and F, and echo's P and F are those.
#[test]
fn the_header_vectors() {
    let nns = vector_numbers("g-header.vector");
    assert_eq!(nns.len(), 17);
    let mut undecided = Vec::new();
    for nn in &nns {
        let rel = format!("{nn}g-header.vector");
        let bytes = if nn == "24" {
            read("24-source.pact-text")
        } else {
            noted_source(nn)
        };
        let src = source::read(&bytes);
        let report = dark2::report(&src);
        let want = program::expected(&src, &report).unwrap_or_else(|| {
            undecided.push(nn.clone());
            Expected {
                program: Some(program::program(&src)),
                facts: Some(program::facts(&src)),
            }
        });
        let atom = field(&rel, "atom");
        let args: Vec<&str> = atom
            .strip_prefix("header(")
            .and_then(|a| a.strip_suffix(')'))
            .expect("header(…)")
            .split(", ")
            .collect();
        let side = |a: Option<Address>| match a {
            Some(a) => format!("addr {}", hex(&a)),
            None => "sym \"none\"".to_string(),
        };
        assert_eq!(args[1], side(want.program), "{rel}: program");
        assert_eq!(args[2], side(want.facts), "{rel}: facts");
        let value = |a: &str| match a.strip_prefix("addr ") {
            Some(h) => Value::Addr(unhex(h)),
            None => Value::Sym("none".into()),
        };
        let header = Atom {
            pred: "header".into(),
            args: vec![
                value(args[0]),
                value(&side(want.program)),
                value(&side(want.facts)),
                value(args[3]),
                value(args[4]),
            ],
        };
        assert_eq!(
            hex(&address::atom(&Tau::mirror_certificate(), &header)),
            field(&rel, "blake3"),
            "{rel}"
        );
    }
    assert_eq!(undecided, ["95", "97"]);
}

/// Every correct fixture's certificate and Taut's thirteen certificates
/// claim exactly the P and F echo recomputes. Only `ct-a4` claims another
/// F (its spurious fact); no fixture claims another P.
#[test]
fn the_certificates_claim_the_recomputed_addresses() {
    let mut triples: Vec<(String, Vec<u8>, Vec<u8>)> = fixtures()
        .into_iter()
        .map(|n| {
            let (t, _) = fixture(&n);
            (n, t.source, t.certificate)
        })
        .collect();
    for nn in [
        "20", "21", "22", "23", "24", "71", "77", "80", "81", "90", "91",
    ] {
        triples.push((
            nn.into(),
            read(&format!("{nn}-source.pact-text")),
            read(&format!("{nn}-certificate.cert")),
        ));
    }
    triples.push((
        "55".into(),
        read("48-source.pact-text"),
        read("55-certificate.cert"),
    ));
    triples.push((
        "68".into(),
        read("01-determinism.pact-text"),
        read("68-certificate-stray.cert"),
    ));
    let mut decided = 0;
    for (name, source, cert) in &triples {
        let src = source::read(source);
        let report = dark2::report(&src);
        let header = certificate::read(cert).expect(name).header;
        let Some(want) = program::expected(&src, &report) else {
            continue;
        };
        decided += 1;
        assert_eq!(header.program, want.program, "{name}: program");
        if name == "ct-a4-spurious-fact" {
            assert_ne!(header.facts, want.facts, "{name}");
        } else {
            assert_eq!(header.facts, want.facts, "{name}: facts");
        }
    }
    // Every one: none of them has a reader hole with light R2 can't tell.
    assert_eq!(decided, triples.len());
    assert_eq!(decided, 38 + 13);
}

/// typed-corpus-unheard (S4): its facts are typed, and F is their typed
/// atoms' set, the certificate's claim.
#[test]
fn typed_facts_enter_f_with_their_types() {
    let (t, _) = fixture("typed-corpus-unheard");
    let src = source::read(&t.source);
    let header = certificate::read(&t.certificate).expect("reads").header;
    assert_eq!(
        hex(&program::facts(&src)),
        "c17a34eacedce160387e82129df7629cc551d0c72afbcf3dc647444edf3db6c6"
    );
    assert_eq!(header.facts, Some(program::facts(&src)));
}

// --------------------------------------------------- the canonical line

fn var(v: &str) -> Term {
    Term::Var(v.into())
}

fn sym(s: &str) -> Term {
    Term::Const(Value::Sym(s.into()))
}

/// Certificate §3.1, §3.1.1 B, G: the whole record line, the keyword
/// twice, single spaces, `, ` between arguments, literals and key
/// positions, `not ` before a negative literal, zero-ary atoms bare,
/// ` -> `; variables keep their `?`. Facts have no program atom.
#[test]
fn the_canonical_line() {
    let src = source::read(
        b"rule  r1   p(?x,a)   requires q(?x),not  s ,  t(b,?x)\n\
          functional  f(1,22)  ->   3\n\
          query   p(a,b)\n\
          query z\n\
          fact e(a)\n",
    );
    let canon: Vec<Option<String>> = src
        .lines
        .iter()
        .map(|l| program::canonical(&l.record))
        .collect();
    assert_eq!(
        canon,
        [
            Some("rule r1 p(?x, a) requires q(?x), not s, t(b, ?x)".to_string()),
            Some("functional f(1, 22) -> 3".to_string()),
            Some("query p(a, b)".to_string()),
            Some("query z".to_string()),
            None,
        ]
    );
    // A typed constant prints in its canonical text (it reads only in
    // facts, which have no program atom; Q24).
    let typed = Record::Query(Atom {
        pred: "p".into(),
        args: vec![Value::Nat(7)],
    });
    assert_eq!(
        program::canonical(&typed).as_deref(),
        Some("query p(nat 7)")
    );
    let rule = Record::Rule {
        id: "r".into(),
        head: Pattern {
            pred: "h".into(),
            args: vec![var("x"), sym("k")],
        },
        body: vec![
            Literal {
                negative: true,
                atom: Pattern {
                    pred: "n".into(),
                    args: vec![],
                },
            },
            Literal {
                negative: false,
                atom: Pattern {
                    pred: "b".into(),
                    args: vec![Term::Const(Value::Nat(2)), var("x")],
                },
            },
        ],
    };
    assert_eq!(
        program::canonical(&rule).as_deref(),
        Some("rule r h(?x, k) requires not n, b(nat 2, ?x)")
    );
}

/// P is a set of distinct records, F of distinct facts (§3.1: "one atom
/// per distinct record"; fixpoint §5.0.1 item 2): a line repeated, or
/// re-spaced, adds nothing. Queries and declarations are P's, never F's.
#[test]
fn p_and_f_are_sets_of_the_lines_that_read() {
    let one = source::read(b"rule r p requires q\nfact q\nfact e(a)\n");
    let twice = source::read(
        b"rule r p requires q\nrule  r  p  requires  q\nfact q\nfact e(a)\nfact e( a)\nfact q\n",
    );
    assert_eq!(program::program(&one), program::program(&twice));
    assert_eq!(program::facts(&one), program::facts(&twice));
    let queried = source::read(b"rule r p requires q\nfact q\nfact e(a)\nquery p\n");
    assert_eq!(program::facts(&one), program::facts(&queried));
    assert_ne!(program::program(&one), program::program(&queried));
    let declared = source::read(b"rule r p requires q\nfact q\nfact e(a)\nfunctional e(1) -> 1\n");
    assert_eq!(program::facts(&one), program::facts(&declared));
    assert_ne!(program::program(&one), program::program(&declared));
    // The empty program and the empty facts: fragments with k = 0 (35b,
    // 23b's digest).
    let empty = source::read(b"");
    assert_eq!(
        hex(&program::facts(&empty)),
        "b811692f0079dc1b370f3e7656d0c8bde2fb51a219a2f29da74c21222d8671c1"
    );
    // Every line that read, left out or not (H5): a rule with an unsafe
    // head is in P.
    let unsafe_rule = source::read(b"rule r p(?x) requires q\nfact q\n");
    let safe = source::read(b"fact q\n");
    assert_ne!(program::program(&unsafe_rule), program::program(&safe));
    // Lines that don't read are in neither (95a, 97a).
    let with_hole = source::read(b"rule r p requires q\nfact q\nfact e(a)\nfact Y\n");
    assert_eq!(program::program(&one), program::program(&with_hole));
    assert_eq!(program::facts(&one), program::facts(&with_hole));
}

/// Which header the source calls for (H5; §3.1.1 F).
#[test]
fn the_expected_header_follows_the_light() {
    let expect = |bytes: &[u8]| {
        let src = source::read(bytes);
        program::expected(&src, &dark2::report(&src))
    };
    let both = |bytes: &[u8]| {
        let src = source::read(bytes);
        Some(Expected {
            program: Some(program::program(&src)),
            facts: Some(program::facts(&src)),
        })
    };
    let none = Some(Expected {
        program: None,
        facts: None,
    });
    // No hole.
    let s = b"rule r p requires q\nfact q\n";
    assert_eq!(expect(s), both(s));
    // Total dark after a line that doesn't read: `none`.
    let s = b"fact Edge(a\n";
    assert_eq!(expect(s), none);
    // Total dark after a reader fault (a repeated rule id): `none`.
    let s = b"rule r p requires q\nrule r p requires s\n";
    assert_eq!(expect(s), none);
    // Total dark after an engine fault (22): P and F.
    let s = b"rule r p(?x) requires q(?x, ?x)\nfact q(a)\n";
    assert_eq!(expect(s), both(s));
    // Lit beside a line that doesn't read (95, 97): P and F.
    let s = b"fact s(k)\nfact Y\n";
    assert_eq!(expect(s), both(s));
    // Light R2 can't tell beside a line that doesn't read: `none` if the
    // run finds no light, P and F if it does. R3 can't tell.
    let s = b"fact e(a, b)\nfact X\n";
    assert_eq!(expect(s), None);
    // Light R2 can't tell, no reader hole: P and F either way.
    let s = b"fact p(a)\nrule r q(?x) requires p(?x), not s(?x)\nrule r2 t(?y) requires p(a)\n";
    assert_eq!(expect(s), both(s));
}

// ------------------------------------------------------------ check 1

/// The interval of check 1: claimed, then recomputed; `none` on either
/// side; nothing when they agree.
#[test]
fn check_1_judges_each_address() {
    let p = [1u8; 32];
    let f = [2u8; 32];
    let q = [3u8; 32];
    let want = Expected {
        program: Some(p),
        facts: Some(f),
    };
    let header = |program, facts| certificate::Header {
        program,
        facts,
        ..Default::default()
    };
    assert!(program::judge(&want, &header(Some(p), Some(f))).is_empty());
    assert_eq!(
        program::judge(&want, &header(Some(q), Some(f))),
        [Interval::source(Side::Addr(q), Side::Addr(p))]
    );
    assert_eq!(
        program::judge(&want, &header(Some(p), None)),
        [Interval::source(Side::None, Side::Addr(f))]
    );
    let dark = Expected {
        program: None,
        facts: None,
    };
    assert_eq!(
        program::judge(&dark, &header(Some(p), Some(f))),
        [
            Interval::source(Side::Addr(p), Side::None),
            Interval::source(Side::Addr(f), Side::None)
        ]
    );
    let i = Interval::source(Side::Addr(q), Side::None);
    assert_eq!(i.text(), source_line(&hex(&q), "none"));
}

/// `ct-a4-spurious-fact`, refused for its own reason: `dissonant source`.
/// The certificate claims F with `edge(a, z)`; echo's F is 20b's.
#[test]
fn ct_a4_is_refused_for_its_spurious_fact() {
    let (t, expect) = fixture("ct-a4-spurious-fact");
    assert_eq!(expect, "dissonant source");
    let a = run(&t);
    assert_eq!(lines(&a)[0], "dissonant 1");
    assert_eq!(
        intervals(&a),
        [source_line(
            "cdcde6e982f1790afd8d19bc0644cedb5c8898e663b7289ba12bf35e16f437aa",
            &field("20b-facts.vector", "blake3"),
        )]
    );
    assert_eq!(a.exit(), 1);
}

/// A program claimed for a reader-Dark source, and `none` claimed where
/// echo computes P (§3.2: "a `none` where the checker computes an address
/// fails check 1").
#[test]
fn dark_headers_are_judged() {
    // taut-24 is a reader Dark: its header is all `none`.
    let (t, _) = fixture("taut-24");
    let p = "3e705fddf91700b8adb16f2bf0eee3d78313c7428e308ef0d59367fd0f251edb";
    let t = Triple {
        certificate: replace(&t.certificate, "program ", &format!("program      {p}")),
        ..t
    };
    let a = run(&t);
    assert_eq!(lines(&a)[0], "dissonant 1");
    assert_eq!(intervals(&a), [source_line(p, "none")]);
    // taut-22 is an engine Dark: P and F (22a, 22b).
    let (t, _) = fixture("taut-22");
    let t = Triple {
        certificate: replace(&t.certificate, "program ", "program      none"),
        ..t
    };
    let a = run(&t);
    assert_eq!(
        intervals(&a),
        [source_line("none", &field("22a-program.vector", "blake3"))]
    );
}

/// A source with a reader fault and light R2 can't tell: whether its
/// header is `none` is the run's (R4), so check 1 waits, whatever the
/// claim.
#[test]
fn check_1_waits_for_the_light_it_needs() {
    let (v01, _) = fixture("pos-v01");
    for source in [
        &b"fact e(a, b)\nfact X\n"[..],
        &b"fact p(a)\nrule r q(?x) requires p(?x), not s(?x)\nfact X\n"[..],
    ] {
        let t = Triple {
            source: source.to_vec(),
            certificate: v01.certificate.clone(),
            record: v01.record.clone(),
        };
        assert_eq!(
            lines(&run(&t))[0],
            NEXT,
            "{}",
            String::from_utf8_lossy(source)
        );
    }
}

/// A Dark source printed with a model: check 1 judges its header; check
/// 2's own interval is still open (Q26), so where check 1 agrees the
/// answer is `unknown unreplayed source`, never `rings`.
#[test]
fn a_dark_source_with_a_model() {
    let (v01, _) = fixture("pos-v01");
    let source = b"fact Edge(a\t".to_vec();
    let t = Triple {
        source: source.clone(),
        certificate: v01.certificate.clone(),
        record: v01.record.clone(),
    };
    let a = run(&t);
    let header = certificate::read(&v01.certificate).expect("reads").header;
    let mut want = vec![
        source_line(&hex(&header.program.expect("P")), "none"),
        source_line(&hex(&header.facts.expect("F")), "none"),
    ];
    want.sort();
    assert_eq!(lines(&a)[0], "dissonant 2");
    assert_eq!(intervals(&a), want);
    let cleared = replace(&v01.certificate, "program ", "program      none");
    let cleared = replace(&cleared, "facts ", "facts        none");
    let t = Triple {
        source,
        certificate: cleared,
        record: v01.record.clone(),
    };
    assert_eq!(lines(&run(&t))[0], "unknown unreplayed source");
}

// ------------------------------------------------- the fixtures, under R3

/// Every fixture the tune refuses for a reason in R2's or R3's strata is
/// refused by echo for that reason; every other fixture answers `unknown
/// unreplayed model`, and none rings.
#[test]
fn the_fixture_table() {
    let refused = [
        "a1-record-dark-disagrees",
        "a1-record-dark-in-clear",
        "ct-a4-spurious-fact",
        "ct-a5-readable-dark",
        "ct-a6-unsafe-with-model",
        "dark2-not-canonical",
        "p8-one-line",
    ];
    let names = fixtures();
    assert_eq!(names.len(), 38);
    for name in &names {
        let a = run(&fixture(name).0);
        assert_ne!(a.verdict, Verdict::Rings, "{name}");
        if refused.contains(&name.as_str()) {
            assert!(lines(&a)[0].starts_with("dissonant "), "{name}");
            assert_eq!(a.exit(), 1, "{name}");
        } else {
            assert_eq!(lines(&a)[0], NEXT, "{name}");
            assert_eq!(lines(&a).len(), 3, "{name}");
            assert_eq!(a.exit(), 2, "{name}");
        }
    }
}

/// The kinds echo prints, R2's (`hole`, `dark`) and R3's (`source`):
/// wherever the tune's `expect` names one, echo prints it; echo prints no
/// kind the tune doesn't; no fixture the tune rings is dissonant in echo.
#[test]
fn the_kinds_agree_with_the_tune() {
    const OWNED: [&str; 4] = ["hole", "dark", "dark-source", "source"];
    let mut matched = Vec::new();
    for name in fixtures() {
        let (t, expect) = fixture(&name);
        let a = run(&t);
        let echo: BTreeSet<String> = intervals(&a)
            .iter()
            .map(|l| l.split(' ').nth(2).expect("kind").to_string())
            .collect();
        match expect.strip_prefix("dissonant ") {
            Some(kinds) => {
                let tune: BTreeSet<String> = kinds.split(' ').map(str::to_string).collect();
                assert!(echo.is_subset(&tune), "{name}: {echo:?} not in {tune:?}");
                let owned: BTreeSet<String> = tune
                    .iter()
                    .filter(|k| OWNED.contains(&k.as_str()))
                    .cloned()
                    .collect();
                assert!(owned.is_subset(&echo), "{name}: {owned:?} not in {echo:?}");
                if !owned.is_empty() {
                    matched.push(name);
                }
            }
            None => assert!(echo.is_empty(), "{name}: {echo:?}"),
        }
    }
    assert_eq!(
        matched,
        [
            "a1-record-dark-disagrees",
            "a1-record-dark-in-clear",
            "ct-a4-spurious-fact",
            "ct-a5-readable-dark",
            "ct-a6-unsafe-with-model",
            "dark2-not-canonical",
            "p8-one-line",
        ]
    );
}

/// Taut's triples (55, 56, 71, 77, 80, 81, 90, 91; 20–24 with their own
/// certificates): R2 and R3 agree with each.
#[test]
fn taut_certificates_agree_in_r3() {
    for nn in ["20", "21", "22", "23", "24"] {
        let t = Triple {
            source: read(&format!("{nn}-source.pact-text")),
            certificate: read(&format!("{nn}-certificate.cert")),
            record: Vec::new(),
        };
        let src = source::read(&t.source);
        let cert = certificate::read(&t.certificate).expect(nn);
        let want = program::expected(&src, &dark2::report(&src)).expect(nn);
        assert!(program::judge(&want, &cert.header).is_empty(), "{nn}");
    }
}

/// Vector 68 (A5, dissonant; the stray is R4's): corpus 01's source, 68's
/// certificate, 27's record half. Under R3 echo agrees with P and F and
/// waits for the model; its `heard` line is the vector's, byte for byte.
#[test]
fn vector_68_waits_for_the_model() {
    let stdout = read("27-stdout-positive.stdout");
    let at = stdout
        .split_inclusive(|&b| b == b'\n')
        .take_while(|l| !l.starts_with(b"certificate "))
        .map(<[u8]>::len)
        .sum::<usize>();
    let t = Triple {
        source: read("01-determinism.pact-text"),
        certificate: read("68-certificate-stray.cert"),
        record: stdout[..at].to_vec(),
    };
    let a = run(&t);
    assert_eq!(lines(&a)[0], NEXT);
    assert_eq!(lines(&a)[1], field("68-a5-dissonant.song", "line2"));
}

// ------------------------------------------------- the interval's bytes

/// Note §6.2's text and atom, for a stratum other than R2's: Taut's 45
/// (`structure depth`) and 68's `stray` line and atom, byte for byte; 46
/// and 46a, the same atom at another `τ`, differ.
#[test]
fn interval_text_and_atom_vectors() {
    let structure = |kind: &'static str, left: &str, tick: u64| Interval {
        stratum: "structure",
        kind,
        left: Side::Addr(unhex(left)),
        right: Side::None,
        about: "none",
        run: "tick",
        tick,
        distance: 1,
    };
    let i45 = structure(
        "depth",
        "9e16b48f74793d3951020ef760b9365d2fbec6c5d27764decae47cbebc4887a8",
        2,
    );
    assert_eq!(
        i45.text(),
        field("45-interval-song-interval.vector", "text")
    );
    assert_eq!(
        hex(&address::atom(&Tau::song_interval(), &i45.atom())),
        field("45-interval-song-interval.vector", "blake3")
    );
    let distance = Tau::root("@song/distance").expect("tau");
    assert_eq!(
        hex(&address::atom(&distance, &i45.atom())),
        field("46-interval-song-distance.vector", "blake3")
    );
    assert_ne!(
        field("46-interval-song-distance.vector", "blake3"),
        field("45-interval-song-interval.vector", "blake3")
    );
    // 68: the stray's left side is `path(a, a)` at the store's τ.
    let stray_atom = Atom {
        pred: "path".into(),
        args: vec![Value::Sym("a".into()), Value::Sym("a".into())],
    };
    let left = hex(&address::atom(&Tau::pact_fact(), &stray_atom));
    let i68 = structure("stray", &left, 1);
    assert_eq!(i68.text(), field("68-a5-dissonant.song", "line4"));
    assert!(field("68-a5-dissonant.song", "atom")
        .starts_with(&hex(&address::atom(&Tau::song_interval(), &i68.atom()))));
}

/// The order (Note §6.2): by stratum in its enumeration's order (`source`,
/// `structure`, `verdict`, then `grammar`, `reader`, `engine`, then
/// `piece`), not by name, then by the line's bytes.
#[test]
fn intervals_sort_by_stratum_then_bytes() {
    let at = |stratum: &'static str, b: u8| Interval {
        stratum,
        kind: "k",
        left: Side::Addr([b; 32]),
        right: Side::None,
        about: "none",
        run: "none",
        tick: 0,
        distance: 1,
    };
    let mut all = vec![
        at("piece", 0),
        at("engine", 0),
        at("verdict", 2),
        at("grammar", 0),
        at("reader", 0),
        at("verdict", 1),
        at("structure", 0),
        at("source", 9),
    ];
    interval::sort(&mut all);
    let order: Vec<(&str, u8)> = all
        .iter()
        .map(|i| match i.left {
            Side::Addr(a) => (i.stratum, a[0]),
            Side::None => (i.stratum, 255),
        })
        .collect();
    assert_eq!(
        order,
        [
            ("source", 9),
            ("structure", 0),
            ("verdict", 1),
            ("verdict", 2),
            ("grammar", 0),
            ("reader", 0),
            ("engine", 0),
            ("piece", 0),
        ]
    );
}

/// Across slices: ct-a4's source interval and a `hole` interval R2 finds
/// in the same record print in one list, `source` first, and `dissonant
/// <n>` counts both.
#[test]
fn strata_compose_in_one_list() {
    let (t, _) = fixture("ct-a4-spurious-fact");
    let mut record = t.record.clone();
    record.extend_from_slice(b"hole        # 9:9 #\n");
    let t = Triple { record, ..t };
    let a = run(&t);
    assert_eq!(lines(&a)[0], "dissonant 2");
    let l = intervals(&a);
    assert!(l[0].starts_with("interval source source "), "{l:?}");
    assert!(l[1].starts_with("interval verdict hole "), "{l:?}");
    let Verdict::Dissonant(found) = &a.verdict else {
        panic!("dissonant")
    };
    assert_eq!(found.iter().map(Interval::text).collect::<Vec<_>>(), l);
}
