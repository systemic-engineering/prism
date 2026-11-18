//! Certificate replay, slice R4: the positive run (docs/spec/echo.md
//! §10.8).
//!
//! echo evaluates the positive program itself, from certificate §1 and
//! §2.1: the least model by naive evaluation, each atom's depth (its naive
//! stage, facts at 0), and every first-tick derivation. Checks 3, 4, 5 and
//! 8 (§4.1) compare the certificate's `layer` and `derive` lines with them,
//! as Note §6.2's `structure` intervals (`stray`, `depth`, `unlayered`,
//! `missing`, `extra`). Every ground rule instance echo forms is charged to
//! the budget. What R4 agrees with answers `unknown unreplayed contexts`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use echo::address::{self, Address, Tau};
use echo::certificate;
use echo::dark2;
use echo::model::{self, Meter, Positive};
use echo::replay::{self, Answer, Inputs, Verdict, BUDGET};
use echo::source;
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

/// Hex to bytes, up to the first space (a vector field's trailing note).
fn unhex(h: &str) -> Vec<u8> {
    let h = h.split(' ').next().unwrap_or_default();
    (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
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

/// A fixture's inputs, its `expect` redirections followed.
fn fixture(name: &str) -> Triple {
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
    Triple {
        source: read(&path("source", "source.pact")),
        certificate: read(&path("certificate", "certificate")),
        record: read(&path("record", "record")),
    }
}

fn fixtures() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(data("fixtures"))
        .expect("fixtures")
        .map(|e| e.expect("entry").file_name().into_string().expect("name"))
        .collect();
    names.sort();
    names
}

fn run_with(t: &Triple, budget: u64) -> Answer {
    replay::replay(
        &Inputs {
            source: Some(&t.source),
            certificate: Some(&t.certificate),
            record: Some(&t.record),
            score: None,
        },
        budget,
    )
}

fn run(t: &Triple) -> Answer {
    run_with(t, BUDGET)
}

fn lines(a: &Answer) -> Vec<String> {
    a.print().lines().map(str::to_string).collect()
}

const NEXT: &str = "unknown unreplayed contexts";

/// echo's own run of a source's eval program, and what it charged.
fn positive_run(src: &[u8]) -> (Positive, u64) {
    let s = source::read(src);
    let report = dark2::report(&s);
    let mut meter = Meter::new(u64::MAX);
    let (facts, rules) = model::program(&s, &report, false);
    let run = model::evaluate(&facts, &rules, &mut meter).expect("no budget");
    (run, meter.spent())
}

fn atom(text: &str) -> Atom {
    let src = source::read(format!("fact {text}\n").as_bytes());
    match &src.lines[0].record {
        source::Record::Fact(a) => a.clone(),
        other => panic!("not a fact: {other:?}"),
    }
}

fn fact_addr(text: &str) -> String {
    hex(&address::atom(&Tau::pact_fact(), &atom(text)))
}

/// The source a `NNa-program` vector names in its `note source:` line.
fn noted_source(nn: &str) -> Vec<u8> {
    let note = field(&format!("{nn}a-program.vector"), "note");
    let records = note.strip_prefix("source: ").expect("a source note");
    let mut s: String = records.split(" | ").map(|r| format!("{r}\n")).collect();
    if s.is_empty() {
        s.push('\n');
    }
    s.into_bytes()
}

// ------------------------------------------------------- the definitions

/// Certificate §1: facts at depth 0, a derived atom at its naive stage
/// (`1 + max` over its first-tick derivations' premises); §2.1: all
/// first-tick derivations, the diamond's `d` by both `b` and `c`.
#[test]
fn the_diamond_has_two_first_tick_derivations() {
    let (run, _) = positive_run(&fixture("pos-diamond").source);
    let depth: BTreeMap<String, u64> = run.depth.iter().map(|(a, d)| (a.to_string(), *d)).collect();
    assert_eq!(
        depth,
        BTreeMap::from([
            ("a".to_string(), 0),
            ("b".to_string(), 1),
            ("c".to_string(), 1),
            ("d".to_string(), 2),
        ])
    );
    let heads: Vec<(Atom, Vec<Atom>)> = run
        .derivations
        .iter()
        .map(|i| (i.head.clone(), i.premises.clone()))
        .collect();
    assert_eq!(
        heads,
        [
            (atom("b"), vec![atom("a")]),
            (atom("c"), vec![atom("a")]),
            (atom("d"), vec![atom("b")]),
            (atom("d"), vec![atom("c")]),
        ]
    );
}

/// A derivation whose premise is deeper than the shallowest one is not
/// first-tick: vector 01's `path(b, c)` (depth 1) also follows from
/// `path(b, b)` (depth 2) and `edge(b, c)`, and that instance is not
/// carried. A head that is a fact has no derivation (Note §6.5 G5).
#[test]
fn only_first_tick_derivations_and_no_fact_heads() {
    let (run, _) = positive_run(
        b"rule r p(?x) requires q(?x)\nrule s q(?x) requires p(?x)\nfact q(a)\nfact p(b)\n",
    );
    let printed: Vec<(Atom, Atom)> = run
        .derivations
        .iter()
        .map(|i| (i.head.clone(), i.premises[0].clone()))
        .collect();
    assert_eq!(
        printed,
        [(atom("p(a)"), atom("q(a)")), (atom("q(b)"), atom("p(b)"))]
    );
    let (run, _) = positive_run(&fixture("pos-v01").source);
    assert_eq!(run.derivations.len(), 9);
    assert!(run
        .derivations
        .iter()
        .all(|i| i.premises.iter().all(|p| run.depth[p] < run.depth[&i.head])));
    assert!(run.derivations.iter().all(|i| i
        .premises
        .iter()
        .any(|p| run.depth[p] + 1 == run.depth[&i.head])));
}

/// Taut's Notes (38, 44a; negs 41): echo's Note address of a derivation
/// it computes, from the head, the rule's canonical line and the premises.
#[test]
fn the_note_vectors() {
    assert_eq!(
        hex(&address::fragment(&Tau::pact_fact(), "negs", &[])),
        field("41-negs-empty.song", "blake3")
    );
    for (rel, src) in [
        ("38-note-positive.song", read("20-source.pact-text")),
        ("44a-note-step.song", noted_source("31")),
    ] {
        let (run, _) = positive_run(&src);
        let s = source::read(&src);
        let report = dark2::report(&s);
        let (_, rules) = model::program(&s, &report, false);
        let head = atom(&field(rel, "head"));
        let premises: Vec<Atom> = field(rel, "premises").split(" & ").map(atom).collect();
        let found = run
            .derivations
            .iter()
            .find(|i| i.head == head && i.premises == premises)
            .expect(rel);
        let line = &rules[found.rule].line;
        assert_eq!(
            format!("\"{line}\""),
            field(rel, "rule").rsplit_once(' ').expect("rule").0
        );
        assert_eq!(
            hex(&model::note(&head, line, &premises)),
            field(rel, "blake3"),
            "{rel}"
        );
    }
}

/// S4, beyond the tune: a typed fact keeps its type through the run. A
/// variable binds `nat 7`, and the head carries it; `nat 7` and `sym "7"`
/// are two constants, so two heads.
#[test]
fn typed_constants_run_with_their_types() {
    let (run, n) = positive_run(
        b"fact n(nat 7)\nfact n(sym \"7\")\nfact k(addr 0000000000000000000000000000000000000000000000000000000000000001)\nrule r m(?x) requires n(?x)\nrule s j(?y) requires k(?y)\n",
    );
    let m: Vec<&Atom> = run.depth.keys().filter(|a| a.pred == "m").collect();
    assert_eq!(
        m,
        [
            &Atom {
                pred: "m".into(),
                args: vec![Value::Sym("7".into())]
            },
            &Atom {
                pred: "m".into(),
                args: vec![Value::Nat(7)]
            },
        ]
    );
    let mut one = [0u8; 32];
    one[31] = 1;
    assert!(run.depth.contains_key(&Atom {
        pred: "j".into(),
        args: vec![Value::Addr(one)]
    }));
    // Two rounds of three instances: the second adds nothing.
    assert_eq!(n, 6);
}

// ------------------------------------------------ the models, from Taut's

/// `layers`: one `tick` fragment per tick, 0 to the deepest, each its
/// atoms' addresses sorted (certificate §3.1, §3.1.1 A).
fn layers(run: &Positive) -> Address {
    let tau = Tau::mirror_certificate();
    let fact = Tau::pact_fact();
    let ticks: Vec<Address> = match run.top() {
        None => Vec::new(),
        Some(top) => (0..=top)
            .map(|t| {
                let mut atoms: Vec<Address> = run
                    .depth
                    .iter()
                    .filter(|(_, d)| **d == t)
                    .map(|(a, _)| address::atom(&fact, a))
                    .collect();
                atoms.sort();
                address::fragment(&tau, "tick", &atoms)
            })
            .collect(),
    };
    address::fragment(&tau, "layers", &ticks)
}

/// `derivations`: `derive(addr head, sym rule-id, addr p₁, …)` atoms,
/// sorted (§3.1).
fn derivations(src: &[u8], run: &Positive) -> Address {
    let s = source::read(src);
    let report = dark2::report(&s);
    let (_, rules) = model::program(&s, &report, false);
    let tau = Tau::mirror_certificate();
    let fact = Tau::pact_fact();
    let mut atoms: Vec<Address> = run
        .derivations
        .iter()
        .map(|i| {
            let mut args = vec![
                Value::Addr(address::atom(&fact, &i.head)),
                Value::Sym(rules[i.rule].id.to_string()),
            ];
            args.extend(
                i.premises
                    .iter()
                    .map(|p| Value::Addr(address::atom(&fact, p))),
            );
            address::atom(
                &tau,
                &Atom {
                    pred: "derive".into(),
                    args,
                },
            )
        })
        .collect();
    atoms.sort();
    address::fragment(&tau, "derivations", &atoms)
}

/// The positive sources among Taut's: echo's model, layers and
/// derivations equal his `NNc`, `NNd` and `NNe` digests. (The W ones, 23,
/// 30, 88, 94, 95 and 97, are R5's.)
#[test]
fn the_model_layer_and_derivation_vectors() {
    let mut checked = Vec::new();
    for nn in ["20", "21", "22", "31", "35", "82", "83", "86", "87", "89"] {
        let src = if nn == "22" {
            read("22-source.pact-text")
        } else {
            noted_source(nn)
        };
        assert!(model::positive(&source::read(&src)), "{nn}");
        let (run, _) = positive_run(&src);
        if nn != "22" {
            assert_eq!(
                hex(&run.model()),
                field(&format!("{nn}c-result-model.vector"), "blake3"),
                "{nn}c"
            );
        }
        assert_eq!(
            hex(&layers(&run)),
            field(&format!("{nn}d-layers.vector"), "blake3"),
            "{nn}d"
        );
        assert_eq!(
            hex(&derivations(&src, &run)),
            field(&format!("{nn}e-derivations.vector"), "blake3"),
            "{nn}e"
        );
        checked.push(nn);
    }
    assert_eq!(checked.len(), 10);
}

/// echo's model address equals the certificate's `result` on every
/// correct positive fixture, and on Taut's positive certificates. 22 is an
/// engine Dark (`result none`; echo's eval program is empty).
#[test]
fn echo_s_model_is_the_certificates_result() {
    let claimed = |cert: &[u8]| certificate::read(cert).expect("reads").header.result;
    let mut names = Vec::new();
    for name in fixtures() {
        let t = fixture(&name);
        let expect = text(&format!("fixtures/{name}/expect"));
        if !expect.lines().any(|l| l == "expect rings")
            || !model::positive(&source::read(&t.source))
        {
            continue;
        }
        let Some(result) = claimed(&t.certificate) else {
            continue;
        };
        let (mut run, _) = positive_run(&t.source);
        // A budget stop's `result` is the stopped set: every atom at depth
        // <= n (Note §6.5 S1). Judging the claim is R7's.
        if let Some(e) = certificate::read(&t.certificate)
            .expect("reads")
            .erasures
            .first()
        {
            run.depth.retain(|_, d| *d <= e.count);
        }
        assert_eq!(run.model(), result, "{name}");
        names.push(name);
    }
    assert_eq!(
        names,
        [
            "a8-budget-query-unknown",
            "p8-swapped",
            "pos-diamond",
            "pos-v01",
            "pos-v05",
            "pos-v05-control",
            "taut-20",
            "taut-21",
        ]
    );
    for nn in ["20", "21", "71", "77", "80", "81", "90", "91"] {
        let src = read(&format!("{nn}-source.pact-text"));
        assert!(model::positive(&source::read(&src)), "{nn}");
        let (run, _) = positive_run(&src);
        assert_eq!(
            Some(run.model()),
            claimed(&read(&format!("{nn}-certificate.cert"))),
            "{nn}"
        );
    }
}

// ------------------------------------------------------------- fixtures

fn structure(kind: &str, left: &str, right: &str, tick: u64, by: u64) -> String {
    format!("interval structure {kind} {left} {right} about none at tick {tick} by {by}")
}

fn note(head: &str, line: &str, premises: &[&str]) -> String {
    let ps: Vec<Atom> = premises.iter().map(|p| atom(p)).collect();
    hex(&model::note(&atom(head), line, &ps))
}

const BASE: &str = "rule path-base path(?x, ?y) requires edge(?x, ?y)";
const STEP: &str = "rule path-step path(?x, ?z) requires path(?x, ?y), edge(?y, ?z)";

/// The cheats whose cheat lives in the model, its depths or its
/// derivations, refused for their own reason, line by line.
#[test]
fn the_structure_cheats() {
    // CT-A2: d's first-tick derivation via c is omitted (check 8).
    let a = run(&fixture("ct-a2-missing-derivation"));
    assert_eq!(
        lines(&a)[3..],
        [structure(
            "missing",
            "none",
            &note("d", "rule cd d requires c", &["c"]),
            2,
            1
        )]
    );
    assert_eq!(lines(&a)[0], "dissonant 1");
    // CT-A8: path(a, d) at 4, its depth 3 (check 4).
    let a = run(&fixture("ct-a8-layer-off-by-one"));
    assert_eq!(
        lines(&a)[3..],
        [structure("depth", &fact_addr("path(a, d)"), "none", 4, 1)]
    );
    // CT-A9: p and q support only each other; neither is in the model
    // (check 3). Their derivations are not named again (§6.2: one cause,
    // named once; `missing` / `extra` for heads in the model only).
    let a = run(&fixture("ct-a9-circular-pair"));
    let mut want = vec![
        structure("stray", &fact_addr("p"), "none", 1, 1),
        structure("stray", &fact_addr("q"), "none", 1, 1),
    ];
    want.sort();
    assert_eq!(lines(&a)[3..], want);
    // CT-A10: the naive stage 2 presented as the fixed point: path(a, d)
    // is unlayered (checks 4, 5), and its first-tick derivation missing
    // (check 8; Q44).
    let a = run(&fixture("ct-a10-truncated-claims-fixed"));
    assert_eq!(
        lines(&a)[3..],
        [
            structure(
                "missing",
                "none",
                &note("path(a, d)", STEP, &["path(a, c)", "edge(c, d)"]),
                3,
                1
            ),
            structure("unlayered", "none", &fact_addr("path(a, d)"), 3, 1),
        ]
    );
}

/// Vector 68, Taut's whole A5 output for a stray, byte for byte: lines
/// 1–2 and line 4, and the interval's atom.
#[test]
fn vector_68_byte_for_byte() {
    let t = Triple {
        source: read("01-determinism.pact-text"),
        certificate: read("68-certificate-stray.cert"),
        record: {
            // M1: the record ends where the first `certificate` line starts.
            let all = read("27-stdout-positive.stdout");
            let cut = all
                .windows(13)
                .position(|w| w == b"\ncertificate ")
                .expect("split");
            all[..=cut].to_vec()
        },
    };
    let a = run(&t);
    let out = a.print();
    let l: Vec<&str> = out.lines().collect();
    assert_eq!(l.len(), 4);
    let head = format!("{}\n{}\n", l[0], l[1]);
    assert_eq!(
        head.as_bytes(),
        unhex(&field("68-a5-dissonant.song", "exact_hex"))
    );
    assert_eq!(
        format!("{}\n", l[3]).as_bytes(),
        unhex(&field("68-a5-dissonant.song", "tail_hex"))
    );
    assert_eq!(l[3], field("68-a5-dissonant.song", "line4"));
    assert_eq!(a.exit(), 1);
    let Verdict::Dissonant(found) = &a.verdict else {
        panic!("dissonant");
    };
    assert_eq!(
        hex(&address::atom(&Tau::song_interval(), &found[0].atom())),
        field("68-a5-dissonant.song", "atom")
            .split(' ')
            .next()
            .expect("atom")
    );
}

/// Every fixture under R4: the first line and the interval kinds, in
/// print order. The tune's `expect` is a lower bound (each kind it names
/// must appear); echo's kinds beyond it are listed and asked (Q35, Q44,
/// Q45). Nothing rings.
#[test]
fn the_fixture_table() {
    let table: &[(&str, &str, &[&str])] = &[
        ("a1-record-dark-disagrees", "dissonant 2", &["hole", "hole"]),
        ("a1-record-dark-engine", NEXT, &[]),
        ("a1-record-dark-in-clear", "dissonant 1", &["hole"]),
        ("a1-record-dark-reader", NEXT, &[]),
        ("a8-budget-query-unknown", NEXT, &[]),
        ("ct-a1-stable-model-witness", NEXT, &[]),
        (
            "ct-a10-truncated-claims-fixed",
            "dissonant 2",
            &["missing", "unlayered"],
        ),
        ("ct-a11-model-text-sorted", NEXT, &[]),
        ("ct-a12-budget-stop", "dissonant 2", &["source", "source"]),
        ("ct-a2-missing-derivation", "dissonant 1", &["missing"]),
        ("ct-a3-clear-hides-functional", NEXT, &[]),
        (
            "ct-a4-spurious-fact",
            "dissonant 3",
            &["source", "stray", "stray"],
        ),
        (
            "ct-a5-readable-dark",
            "dissonant 3",
            &["source", "source", "dark"],
        ),
        ("ct-a6-unsafe-with-model", "dissonant 2", &["hole", "hole"]),
        ("ct-a8-layer-off-by-one", "dissonant 1", &["depth"]),
        ("ct-a9-circular-pair", "dissonant 2", &["stray", "stray"]),
        ("dark2-not-canonical", "dissonant 1", &["dark"]),
        (
            "e1-false-budget-clear",
            "dissonant 2",
            &["source", "source"],
        ),
        (
            "e1-false-budget-opaque",
            "dissonant 2",
            &["source", "source"],
        ),
        ("e4-dark-order", NEXT, &[]),
        ("p8-one-line", "dissonant 1", &["hole"]),
        ("p8-swapped", NEXT, &[]),
        ("pos-dark2-engine", NEXT, &[]),
        ("pos-dark2-reader", NEXT, &[]),
        ("pos-diamond", NEXT, &[]),
        ("pos-v01", NEXT, &[]),
        ("pos-v03", NEXT, &[]),
        ("pos-v04", NEXT, &[]),
        ("pos-v05", NEXT, &[]),
        ("pos-v05-control", NEXT, &[]),
        ("score-missing-note", NEXT, &[]),
        ("taut-20", NEXT, &[]),
        ("taut-21", NEXT, &[]),
        ("taut-22", NEXT, &[]),
        ("taut-23", NEXT, &[]),
        ("taut-24", NEXT, &[]),
        ("taut-48", NEXT, &[]),
        ("typed-corpus-unheard", NEXT, &[]),
    ];
    assert_eq!(
        table
            .iter()
            .map(|(n, ..)| n.to_string())
            .collect::<Vec<_>>(),
        fixtures()
    );
    for (name, first, kinds) in table {
        let a = run(&fixture(name));
        let l = lines(&a);
        assert_eq!(l[0], *first, "{name}");
        let got: Vec<&str> = l[3..]
            .iter()
            .map(|i| i.split(' ').nth(2).expect("kind"))
            .collect();
        assert_eq!(got, *kinds, "{name}");
        assert_ne!(a.verdict, Verdict::Rings, "{name}");
        assert!(!l[0].contains("unreplayed model"), "{name}");
    }
}

/// The cheats R4 owns are refused for the tune's named kind: each kind the
/// tune's `expect` lists is among echo's.
#[test]
fn the_kinds_agree_with_the_tune() {
    for name in [
        "ct-a2-missing-derivation",
        "ct-a8-layer-off-by-one",
        "ct-a9-circular-pair",
        "ct-a10-truncated-claims-fixed",
    ] {
        let a = run(&fixture(name));
        let expect = text(&format!("fixtures/{name}/expect"));
        let kinds: Vec<&str> = expect
            .lines()
            .find_map(|l| l.strip_prefix("expect dissonant "))
            .expect("dissonant")
            .split(' ')
            .collect();
        let l = lines(&a);
        for k in kinds {
            assert!(
                l[3..].iter().any(|i| i.split(' ').nth(2) == Some(k)),
                "{name}: {k}"
            );
        }
    }
}

/// A derivation that is a valid instance of its rule but not first-tick is
/// `extra` (check 8): vector 01's `path(b, c)` (depth 1) by path-step from
/// `path(b, b)` (depth 2) and `edge(b, c)`. So is one that is no instance
/// at all, and a first-tick one printed twice (a list, never collapsed).
#[test]
fn an_extra_derivation_is_refused() {
    let f = fixture("pos-v01");
    let with = |line: &str| {
        let mut c = f.certificate.clone();
        c.extend_from_slice(line.as_bytes());
        c.push(b'\n');
        Triple {
            source: f.source.clone(),
            certificate: c,
            record: f.record.clone(),
        }
    };
    let cases = [
        (
            "derive       path(b, c) by path-step from path(b, b) & edge(b, c)",
            note("path(b, c)", STEP, &["path(b, b)", "edge(b, c)"]),
            1,
        ),
        (
            "derive       path(a, d) by path-base from edge(a, d)",
            note("path(a, d)", BASE, &["edge(a, d)"]),
            3,
        ),
        (
            "derive       path(a, b) by path-base from edge(a, b)",
            note("path(a, b)", BASE, &["edge(a, b)"]),
            1,
        ),
    ];
    for (line, n, tick) in cases {
        let a = run(&with(line));
        assert_eq!(lines(&a)[0], "dissonant 1", "{line}");
        assert_eq!(
            lines(&a)[3],
            structure("extra", &n, "none", tick, 1),
            "{line}"
        );
    }
}

/// A model atom layered at the wrong tick prints `depth` by the distance;
/// a fact left unlayered prints `unlayered` at 0.
#[test]
fn depth_by_distance_and_an_unlayered_fact() {
    let f = fixture("pos-v01");
    let swap = |from: &str, to: &str| Triple {
        source: f.source.clone(),
        certificate: String::from_utf8(f.certificate.clone())
            .expect("utf-8")
            .replace(from, to)
            .into_bytes(),
        record: f.record.clone(),
    };
    let a = run(&swap(
        "layer        1 path(a, b)",
        "layer        5 path(a, b)",
    ));
    assert_eq!(
        lines(&a)[3..],
        [structure("depth", &fact_addr("path(a, b)"), "none", 5, 4)]
    );
    let a = run(&swap("layer        0 edge(c, d)\n", ""));
    assert_eq!(
        lines(&a)[3..],
        [structure(
            "unlayered",
            "none",
            &fact_addr("edge(c, d)"),
            0,
            1
        )]
    );
}

/// What R4 can't print bytes for stays unreplayed, never accepted: a
/// layer line repeated (Q42), a derivation of an atom neither in the model
/// nor layered (Q40), a rule id the source doesn't have (Q41).
#[test]
fn what_r4_cannot_print_stays_unreplayed() {
    let f = fixture("pos-v01");
    for line in [
        "layer        0 edge(a, b)",
        "derive       path(d, a) by path-base from edge(d, a)",
        "derive       path(a, b) by nowhere from edge(a, b)",
    ] {
        let mut c = f.certificate.clone();
        c.extend_from_slice(line.as_bytes());
        c.push(b'\n');
        let t = Triple {
            source: f.source.clone(),
            certificate: c,
            record: f.record.clone(),
        };
        let a = run(&t);
        assert_eq!(lines(&a)[0], NEXT, "{line}");
        assert_eq!(a.exit(), 2);
    }
}

/// Taut's 95 and 97: W beside a line that doesn't read, so `unknown`
/// unless a certain paradox exists in the positive core (H3 v3.3). R4 runs
/// the core, finds one, and so the header is P and F, his `NNg`. Without
/// the declaration the core has no paradox: `unknown`, `none` (Q37).
#[test]
fn the_positive_core_decides_95_and_97() {
    for nn in ["95", "97"] {
        let src = noted_source(nn);
        let s = source::read(&src);
        let report = dark2::report(&s);
        assert!(!model::positive(&s));
        let mut meter = Meter::new(BUDGET);
        let r4 = model::replay(&s, &report, &mut meter).expect("budget");
        assert!(r4.paradox, "{nn}");
        let header = r4.header.expect("decided");
        let atom = field(&format!("{nn}g-header.vector"), "atom");
        assert!(atom.contains(&format!("addr {}", hex(&header.program.expect("P")))));
        assert!(atom.contains(&format!("addr {}", hex(&header.facts.expect("F")))));
        assert!(meter.spent() > 0);
    }
    let src = String::from_utf8(noted_source("95"))
        .expect("utf-8")
        .replace("functional p(1) -> 2\n", "");
    let s = source::read(src.as_bytes());
    let report = dark2::report(&s);
    let r4 = model::replay(&s, &report, &mut Meter::new(BUDGET)).expect("budget");
    assert!(!r4.paradox);
    let header = r4.header.expect("decided");
    assert_eq!((header.program, header.facts), (None, None));
}

// ---------------------------------------------------------------- budget

/// Vector 01 by naive evaluation: round 1 forms 4 instances (path-base
/// only), round 2 8, round 3 13, and the closure round 13: 38 in all. A
/// budget of exactly 38 finishes; 37 runs out. The budget is charged per
/// instance formed, not per `derive` line (9).
#[test]
fn the_budget_is_charged_per_instance() {
    let f = fixture("pos-v01");
    assert_eq!(positive_run(&f.source).1, 38);
    assert_eq!(lines(&run_with(&f, 38))[0], NEXT);
    let a = run_with(&f, 37);
    assert_eq!(a.verdict, Verdict::Uncertain { budget: 37 });
    assert_eq!(lines(&a)[0], "uncertain budget 37 ground rule instances");
    assert_eq!(a.exit(), 1);
    // Before any dissonance: ct-a8 runs out the same way.
    let a = run_with(&fixture("ct-a8-layer-off-by-one"), 37);
    assert_eq!(lines(&a)[0], "uncertain budget 37 ground rule instances");
    assert_eq!(
        lines(&run_with(&fixture("ct-a8-layer-off-by-one"), 38))[0],
        "dissonant 1"
    );
}

/// The instances each fixture's source charges (W sources run nothing in
/// R4 unless their header turns on the positive core).
#[test]
fn the_budget_on_every_fixture() {
    let charged: Vec<(String, u64)> = fixtures()
        .into_iter()
        .map(|name| {
            let t = fixture(&name);
            let s = source::read(&t.source);
            let report = dark2::report(&s);
            let mut meter = Meter::new(BUDGET);
            model::replay(&s, &report, &mut meter).expect("budget");
            let n = meter.spent();
            // The boundary is exact: n fits, n - 1 doesn't (when n > 0).
            assert_ne!(
                run_with(&t, n).verdict,
                Verdict::Uncertain { budget: n },
                "{name}"
            );
            if n > 0 {
                assert_eq!(
                    run_with(&t, n - 1).verdict,
                    Verdict::Uncertain { budget: n - 1 },
                    "{name}"
                );
            }
            (name, n)
        })
        .collect();
    let charged: BTreeMap<&str, u64> = charged.iter().map(|(n, c)| (n.as_str(), *c)).collect();
    assert_eq!(charged["pos-v01"], 38);
    assert_eq!(charged["ct-a8-layer-off-by-one"], 38);
    // The diamond: round 1 forms b and c (2), round 2 adds d by both and
    // re-forms b and c (4), the closure round 4: 10.
    assert_eq!(charged["pos-diamond"], 10);
    assert_eq!(charged["ct-a2-missing-derivation"], 10);
    // ct-a9: neither rule's premise is ever in a stage; one empty round.
    assert_eq!(charged["ct-a9-circular-pair"], 0);
}

// ----------------------------------------------------------- determinism

/// Two runs of the same input give the same bytes, on every fixture.
#[test]
fn the_same_bytes_twice() {
    for name in fixtures() {
        let t = fixture(&name);
        assert_eq!(run(&t).print(), run(&t).print(), "{name}");
    }
}

/// No hash-ordered collection anywhere in echo's library: every map and
/// set is ordered, so no hash order can reach the output.
#[test]
fn no_hash_order_in_the_library() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    for e in fs::read_dir(&dir).expect("src") {
        let p = e.expect("entry").path();
        if p.extension().is_some_and(|x| x == "rs") {
            files.push(p);
        }
    }
    assert!(files.len() >= 14);
    for p in files {
        let code = fs::read_to_string(&p).expect("source");
        for word in ["HashMap", "HashSet", "hashbrown", "RandomState"] {
            assert!(!code.contains(word), "{}: {word}", p.display());
        }
    }
}
