//! Certificate replay, slice R2, end to end: the source and the Dark-2
//! report against the performance and the record (docs/spec/echo.md
//! §10.4).
//!
//! What R2 judges (Note v3.3.2 §6.4, §6.7 H4, H5, H7; certificate §2.4,
//! check 10): the record's `hole` lines, as a multiset of hole texts,
//! against the source's holes (`hole … about record`); the performance's
//! `hole` lines, and its `dark 2` atom against the head of the source's
//! report (`hole … about performance`, `dark … about performance`). Where
//! R2 can't tell, and where nothing differs, R3 judges P and F (check 1;
//! `crate::r3`), and then the answer is the next stratum's: `unknown
//! unreplayed model`. Nothing rings.
//!
//! The fixtures' `expect` (tune's `test_checker.py` docstring): `expect
//! dissonant <kind>…`, each kind among the printed intervals.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use echo::address::{self, Address, Tau};
use echo::interval::Interval;
use echo::replay::{self, Answer, Inputs, Verdict, BUDGET};
use echo::value::{Atom, Value};

fn data(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/echo-vectors/mirror")
        .join(rel)
}

fn read(rel: &str) -> Vec<u8> {
    fs::read(data(rel)).expect(rel)
}

fn hex(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

struct Triple {
    source: Vec<u8>,
    certificate: Vec<u8>,
    record: Vec<u8>,
}

/// A fixture's inputs, its `expect` redirections followed.
fn fixture(name: &str) -> (Triple, String) {
    let expect = String::from_utf8(read(&format!("fixtures/{name}/expect"))).expect("utf-8");
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

fn certificate_atom(a: &Atom) -> String {
    hex(&address::atom(&Tau::mirror_certificate(), a))
}

fn dark_at(line: u64, column: u64) -> String {
    certificate_atom(&Atom {
        pred: "dark".into(),
        args: vec![
            Value::Nat(2),
            Value::Sym("at".into()),
            Value::Nat(line),
            Value::Nat(column),
        ],
    })
}

fn hole_atom(words: &[&str]) -> String {
    certificate_atom(&Atom {
        pred: "hole".into(),
        args: words.iter().map(|w| Value::Sym((*w).into())).collect(),
    })
}

const NEXT: &str = "unknown unreplayed model";

/// R2's lines (stratum `verdict`) on all 38 fixtures. The interval lines
/// are exact: the hole texts' digests are b3sum 1.8.7's, run by hand; the
/// atoms' addresses are echo's encoder (checked against Taut's 48a, 71c,
/// 73–77, 81, 90, 91). The whole answer, R3's `source` lines and the
/// count included, is `r3::the_fixture_table`.
#[test]
fn the_fixture_table() {
    let hole = |left: &str, right: &str, about: &str| {
        format!("interval verdict hole {left} {right} about {about} at none 0 by 1")
    };
    let dark = |left: &str, right: &str| {
        format!("interval verdict dark {left} {right} about performance at none 0 by 1")
    };
    let mut ct_a6 = vec![
        hole(
            "none",
            "356f962f9a253189e45af963f7de03c6b9f6a235b80f8d6512698713e0f969dc",
            "record",
        ),
        hole("none", &hole_atom(&["safety", "bad", "?x"]), "performance"),
    ];
    ct_a6.sort();
    let dissonant: Vec<(&str, Vec<String>)> = vec![
        (
            "a1-record-dark-disagrees",
            vec![
                hole(
                    "61cdf14c4a179f38edcdbcfe6a024696771d87e57cd0c2ab20284e9d34c76efe",
                    "none",
                    "record",
                ),
                hole(
                    "none",
                    "f9b90e3f8908b8d08ae7df29a199bd56e7e7e732f020d2c87c4e20a59606d457",
                    "record",
                ),
            ],
        ),
        (
            "a1-record-dark-in-clear",
            vec![hole(
                "3491ee284e5db6aaa2e4382187830569d2b34acc458f545b5502f556e890eb5e",
                "none",
                "record",
            )],
        ),
        ("ct-a5-readable-dark", vec![dark(&dark_at(3, 5), "none")]),
        ("ct-a6-unsafe-with-model", ct_a6),
        (
            "dark2-not-canonical",
            vec![dark(&dark_at(5, 5), &dark_at(3, 14))],
        ),
        (
            "p8-one-line",
            vec![hole(
                "none",
                "f210bb24d7906dfff2927b08eb9f63d80cda165c039ae7c6acf5d719d8e73744",
                "record",
            )],
        ),
    ];
    let r2_lines = |a: &Answer| -> Vec<String> {
        intervals(a)
            .into_iter()
            .filter(|l| l.starts_with("interval verdict "))
            .collect()
    };
    let names: BTreeSet<&str> = dissonant.iter().map(|(n, _)| *n).collect();
    for (name, want) in &dissonant {
        let a = run(&fixture(name).0);
        assert!(lines(&a)[0].starts_with("dissonant "), "{name}");
        assert_eq!(&r2_lines(&a), want, "{name}");
        assert_eq!(a.exit(), 1, "{name}");
    }
    for entry in fs::read_dir(data("fixtures")).expect("fixtures") {
        let name = entry
            .expect("entry")
            .file_name()
            .into_string()
            .expect("name");
        if names.contains(name.as_str()) {
            continue;
        }
        let a = run(&fixture(&name).0);
        assert!(r2_lines(&a).is_empty(), "{name}");
    }
}

/// The kinds R2 prints: `hole`, `dark`. Wherever the tune's `expect` names
/// one of them, echo prints it; echo prints none of them where the tune's
/// `expect` doesn't; and no fixture the tune rings is dissonant in echo.
/// (R3's `source` is `r3::the_kinds_agree_with_the_tune`'s.)
#[test]
fn r2_kinds_agree_with_the_tune() {
    const R2: [&str; 3] = ["hole", "dark", "dark-source"];
    let mut matched = 0;
    for entry in fs::read_dir(data("fixtures")).expect("fixtures") {
        let name = entry
            .expect("entry")
            .file_name()
            .into_string()
            .expect("name");
        let (t, expect) = fixture(&name);
        let a = run(&t);
        let all: BTreeSet<String> = intervals(&a)
            .iter()
            .map(|l| l.split(' ').nth(2).expect("kind").to_string())
            .collect();
        let echo: BTreeSet<String> = all
            .iter()
            .filter(|k| R2.contains(&k.as_str()))
            .cloned()
            .collect();
        match expect.strip_prefix("dissonant ") {
            Some(kinds) => {
                let tune: BTreeSet<String> = kinds.split(' ').map(str::to_string).collect();
                assert!(echo.is_subset(&tune), "{name}: {echo:?} not in {tune:?}");
                let owned: BTreeSet<String> = tune
                    .iter()
                    .filter(|k| R2.contains(&k.as_str()))
                    .cloned()
                    .collect();
                assert!(owned.is_subset(&echo), "{name}: {owned:?} not in {echo:?}");
                if !owned.is_empty() {
                    matched += 1;
                }
            }
            None => assert!(all.is_empty(), "{name}: {all:?}"),
        }
    }
    // a1-record-dark-disagrees, a1-record-dark-in-clear, ct-a5, ct-a6,
    // dark2-not-canonical, p8-one-line.
    assert_eq!(matched, 6);
}

// ------------------------------------------------------ Taut's triples

/// A `.record` vector's whole bytes (its `hex` field; the said text empty).
fn record_vector(rel: &str) -> Vec<u8> {
    let text = String::from_utf8(read(rel)).expect("utf-8");
    let h = text
        .lines()
        .find_map(|l| l.strip_prefix("hex        "))
        .expect("hex");
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).expect("hex"))
        .collect()
}

fn triple(source: &str, certificate: &str, record: &str) -> Triple {
    Triple {
        source: read(source),
        certificate: read(certificate),
        record: record_vector(record),
    }
}

fn vector_triples() -> Vec<(&'static str, Triple)> {
    vec![
        (
            "55",
            triple(
                "48-source.pact-text",
                "55-certificate.cert",
                "55-record-dark2-strata.record",
            ),
        ),
        (
            "56",
            triple(
                "24-source.pact-text",
                "24-certificate.cert",
                "56-record-dark2-reader.record",
            ),
        ),
        (
            "71",
            triple(
                "71-source.pact-text",
                "71-certificate.cert",
                "71-declaration-where.record",
            ),
        ),
        (
            "77",
            triple(
                "77-source.pact-text",
                "77-certificate.cert",
                "77-record-hash-hole.record",
            ),
        ),
        (
            "80",
            triple(
                "80-source.pact-text",
                "80-certificate.cert",
                "80-record-declarations-light.record",
            ),
        ),
        (
            "81",
            triple(
                "81-source.pact-text",
                "81-certificate.cert",
                "81-record-unary-light.record",
            ),
        ),
        (
            "90",
            triple(
                "90-source.pact-text",
                "90-certificate.cert",
                "90-record-declaration-rule-3.record",
            ),
        ),
        (
            "91",
            triple(
                "91-source.pact-text",
                "91-certificate.cert",
                "91-record-declaration-rule-5.record",
            ),
        ),
    ]
}

/// Taut's Dark-2 and hole records, with their performances: R2 agrees
/// with every one (total dark 55, 56; uncertain 71, 77, 80, 81, 90, 91).
#[test]
fn taut_triples_agree_in_r2() {
    for (name, t) in vector_triples() {
        let a = run(&t);
        assert_eq!(lines(&a)[0], NEXT, "{name}");
    }
}

/// Taut's 76 and 76a: 80's record, then 80's performance, without the
/// `declaration p functional p(3) -> 1` hole. echo prints exactly the
/// vector's interval line, and its atom has the vector's address.
#[test]
fn vectors_76_and_76a() {
    let drop = |bytes: &[u8], prefix: &str| -> Vec<u8> {
        let text = String::from_utf8(bytes.to_vec()).expect("utf-8");
        text.lines()
            .filter(|l| !l.starts_with(prefix))
            .map(|l| format!("{l}\n"))
            .collect::<String>()
            .into_bytes()
    };
    let field = |rel: &str, key: &str| -> String {
        String::from_utf8(read(rel))
            .expect("utf-8")
            .lines()
            .find_map(|l| l.strip_prefix(key).map(|r| r.trim().to_string()))
            .expect(key)
    };
    let base = || {
        vector_triples()
            .into_iter()
            .find(|(n, _)| *n == "80")
            .expect("80")
            .1
    };
    let mut t = base();
    t.record = drop(&t.record, "hole        declaration p functional p(3) -> 1");
    let mut u = base();
    u.certificate = drop(
        &u.certificate,
        "hole         declaration p functional p(3) -> 1",
    );
    for (t, vector) in [
        (t, "76-interval-hole-record.vector"),
        (u, "76a-interval-hole-performance.vector"),
    ] {
        let a = run(&t);
        assert_eq!(lines(&a)[0], "dissonant 1", "{vector}");
        assert_eq!(intervals(&a), [field(vector, "text ")], "{vector}");
        let Verdict::Dissonant(found) = &a.verdict else {
            panic!("{vector}")
        };
        assert_eq!(
            hex(&address::atom(&Tau::song_interval(), &found[0].atom())),
            field(vector, "blake3 "),
            "{vector}"
        );
    }
}

/// Check 10, composed (Note §6.4): the record's holes as a multiset. A
/// repeat is an extra line; a hole text never decides by its said text;
/// lines that aren't the source's kind of hole (`budget`, `capacity`,
/// `io`) are R7's and the record's, not R2's.
#[test]
fn the_record_holes_are_a_multiset() {
    let (base, _) = fixture("pos-dark2-reader");
    let with = |extra: &str| -> Triple {
        let mut record = base.record.clone();
        record.extend_from_slice(extra.as_bytes());
        Triple {
            source: base.source.clone(),
            certificate: base.certificate.clone(),
            record,
        }
    };
    let a = run(&with("hole        # 3:14 #again\n"));
    assert_eq!(lines(&a)[0], "dissonant 1");
    assert!(
        intervals(&a)[0].contains(" none about record "),
        "{:?}",
        intervals(&a)
    );
    for extra in [
        "hole        budget count 3 ticks #\n",
        "hole        capacity tick 2 #\n",
        "hole        io #no such file\n",
    ] {
        assert_eq!(lines(&run(&with(extra)))[0], NEXT, "{extra}");
    }
    // The said text is opaque: a different one changes nothing.
    let mut swapped = base.record.clone();
    let text = String::from_utf8(swapped.clone()).expect("utf-8");
    swapped = text
        .replace("#a sym is a lowercase name", "#everything is fine")
        .into_bytes();
    let t = Triple {
        source: base.source.clone(),
        certificate: base.certificate.clone(),
        record: swapped,
    };
    assert_eq!(lines(&run(&t))[0], NEXT);
}

/// H5 (Taut V2): an `unknown` performance carries no `hole` lines; one
/// added is a `hole` interval about the performance. An uncertain one
/// carries every hole and no `dark 2` atom.
#[test]
fn the_performance_holes_follow_the_light() {
    let (dark, _) = fixture("taut-24");
    let mut t = Triple {
        source: dark.source.clone(),
        certificate: dark.certificate.clone(),
        record: dark.record.clone(),
    };
    t.certificate
        .extend_from_slice(b"hole         repeat 2:1\n");
    let a = run(&t);
    assert_eq!(lines(&a)[0], "dissonant 1");
    assert_eq!(
        intervals(&a)[0],
        format!(
            "interval verdict hole {} none about performance at none 0 by 1",
            hole_atom(&["repeat", "2:1"])
        )
    );
    // p8-swapped is uncertain: a `dark 2` atom there is surplus.
    let (lit, _) = fixture("p8-swapped");
    let mut t = Triple {
        source: lit.source.clone(),
        certificate: lit.certificate.clone(),
        record: lit.record.clone(),
    };
    t.certificate.extend_from_slice(b"dark         2 at 1:1\n");
    let a = run(&t);
    assert_eq!(
        intervals(&a),
        [format!(
            "interval verdict dark {} none about performance at none 0 by 1",
            dark_at(1, 1)
        )]
    );
}

/// The `dark` kind pairs only one claimed-only with one derived-only
/// (§6.5 R2); otherwise each surplus atom is its own line.
#[test]
fn dark_atoms_pair_only_one_for_one() {
    let (t0, _) = fixture("pos-dark2-reader");
    let cert = |darks: &str| -> Triple {
        let text = String::from_utf8(t0.certificate.clone()).expect("utf-8");
        let kept: String = text
            .lines()
            .filter(|l| !l.starts_with("dark "))
            .map(|l| format!("{l}\n"))
            .collect();
        Triple {
            source: t0.source.clone(),
            certificate: (kept + darks).into_bytes(),
            record: t0.record.clone(),
        }
    };
    let a = run(&cert(""));
    assert_eq!(
        intervals(&a),
        [format!(
            "interval verdict dark none {} about performance at none 0 by 1",
            dark_at(3, 14)
        )]
    );
    let a = run(&cert("dark         2 at 3:14\ndark         2 at 3:14\n"));
    assert_eq!(
        intervals(&a),
        [format!(
            "interval verdict dark {} none about performance at none 0 by 1",
            dark_at(3, 14)
        )]
    );
    let a = run(&cert("dark         2 at 1:1\ndark         2 at 2:1\n"));
    assert_eq!(lines(&a)[0], "dissonant 3");
    let a = run(&cert("dark         2 engine safety s ?y\n"));
    let safety = certificate_atom(&Atom {
        pred: "dark".into(),
        args: ["engine", "safety", "s", "?y"]
            .iter()
            .fold(vec![Value::Nat(2)], |mut v, w| {
                v.push(Value::Sym((*w).into()));
                v
            }),
    });
    assert_eq!(
        intervals(&a),
        [format!(
            "interval verdict dark {safety} {} about performance at none 0 by 1",
            dark_at(3, 14)
        )]
    );
}

/// Where R2 can't tell, it doesn't: W with a hole that adds (R5's
/// `unknown shadow under W`); light it can't decide without a run (R4).
/// Both `unknown unreplayed model`, whatever the record says: their
/// header waits for the light too (`r3::check_1_waits_for_the_light_it_needs`).
/// A Dark source printed with a model (check 2, Q26) is
/// `r3::a_dark_source_with_a_model`.
#[test]
fn what_r2_cannot_tell_is_left_to_the_next_stratum() {
    let (v01, _) = fixture("pos-v01");
    let bogus = b"verdict     clear\nfile        \"x\"\nshaped_by   none\nhole        # 9:9 #\n";
    for source in [
        &b"fact p(a)\nrule r q(?x) requires p(?x), not s(?x)\nfact X\n"[..],
        &b"fact e(a, b)\nfact X\n"[..],
    ] {
        let t = Triple {
            source: source.to_vec(),
            certificate: v01.certificate.clone(),
            record: bogus.to_vec(),
        };
        assert_eq!(
            lines(&run(&t))[0],
            NEXT,
            "{}",
            String::from_utf8_lossy(source)
        );
    }
}

/// Lines are sorted by stratum, then by the printed line's bytes (Note
/// §6.2); `dissonant <n>` counts lines.
#[test]
fn intervals_are_sorted_and_counted() {
    let a = run(&fixture("ct-a6-unsafe-with-model").0);
    let l = intervals(&a);
    let mut sorted = l.clone();
    sorted.sort();
    assert_eq!(l, sorted);
    let Verdict::Dissonant(found) = &a.verdict else {
        panic!("dissonant")
    };
    assert_eq!(found.len(), 2);
    assert_eq!(found.iter().map(Interval::text).collect::<Vec<_>>(), l);
}
