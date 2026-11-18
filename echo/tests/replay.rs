//! Certificate replay, slice R1: the surface (docs/spec/echo.md §10.1).
//!
//! The lines and exits are Note v3.3.2 §6.3 and mirror's contract delta
//! (`docs/pin-batch/2026-10-30-tune-exit-contract-v3.2.md`); `heard` is
//! §4.2's B8. The data is mirror's, copied byte for byte into
//! docs/spec/echo-vectors/mirror (its README): the 38 checker fixtures,
//! vector 50 with 27's stdout and corpus 01's source.
//!
//! The floor (§10): nothing echo can't replay yet may ring.

use std::fs;
use std::path::PathBuf;

use echo::address::Address;
use echo::replay::{self, conclude, Answer, Input, Inputs, Verdict, BUDGET, STRATA};
use echo::Position;

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

/// One fixture's three (or four) inputs, following its `expect` file's
/// `source`/`certificate`/`record`/`score` redirections (paths relative to
/// the fixture; mirror's `../../vectors/x` is our `x`).
struct Fixture {
    source: Vec<u8>,
    certificate: Vec<u8>,
    record: Vec<u8>,
    score: Option<Vec<u8>>,
}

fn fixture(name: &str) -> Fixture {
    let expect = String::from_utf8(read(&format!("fixtures/{name}/expect"))).expect("utf-8");
    let path = |key: &str, default: &str| -> Option<String> {
        let given = expect
            .lines()
            .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(' ')))
            .map(str::trim);
        match given {
            Some(p) if p.starts_with("../../vectors/") => {
                Some(p["../../vectors/".len()..].to_string())
            }
            Some(p) => Some(format!("fixtures/{name}/{p}")),
            None if default.is_empty() => None,
            None => Some(format!("fixtures/{name}/{default}")),
        }
    };
    Fixture {
        source: read(&path("source", "source.pact").expect("source")),
        certificate: read(&path("certificate", "certificate").expect("certificate")),
        record: read(&path("record", "record").expect("record")),
        score: path("score", "").map(|p| read(&p)),
    }
}

fn run(f: &Fixture, budget: u64) -> Answer {
    replay::replay(
        &Inputs {
            source: Some(&f.source),
            certificate: Some(&f.certificate),
            record: Some(&f.record),
            score: f.score.as_deref().map(Some),
        },
        budget,
    )
}

fn lines(a: &Answer) -> Vec<String> {
    let text = a.print();
    assert!(text.ends_with('\n'), "every line ends in LF: {text:?}");
    text.lines().map(str::to_string).collect()
}

const FIXTURES: [&str; 38] = [
    "a1-record-dark-disagrees",
    "a1-record-dark-engine",
    "a1-record-dark-in-clear",
    "a1-record-dark-reader",
    "a8-budget-query-unknown",
    "ct-a1-stable-model-witness",
    "ct-a10-truncated-claims-fixed",
    "ct-a11-model-text-sorted",
    "ct-a12-budget-stop",
    "ct-a2-missing-derivation",
    "ct-a3-clear-hides-functional",
    "ct-a4-spurious-fact",
    "ct-a5-readable-dark",
    "ct-a6-unsafe-with-model",
    "ct-a8-layer-off-by-one",
    "ct-a9-circular-pair",
    "dark2-not-canonical",
    "e1-false-budget-clear",
    "e1-false-budget-opaque",
    "e4-dark-order",
    "p8-one-line",
    "p8-swapped",
    "pos-dark2-engine",
    "pos-dark2-reader",
    "pos-diamond",
    "pos-v01",
    "pos-v03",
    "pos-v04",
    "pos-v05",
    "pos-v05-control",
    "score-missing-note",
    "taut-20",
    "taut-21",
    "taut-22",
    "taut-23",
    "taut-24",
    "taut-48",
    "typed-corpus-unheard",
];

// ---------------------------------------------------------------- heard

/// B8: plain BLAKE3 of each input's raw bytes. The digests are b3sum
/// 1.8.7's, run on the fixture files by hand, not by echo.
#[test]
fn heard_names_each_input_by_blake3_of_its_raw_bytes() {
    let a = run(&fixture("pos-v01"), BUDGET);
    assert_eq!(
        lines(&a)[1],
        "heard source e997e229707a8e433ede111a8b8540e7adb9adddf7fcb10d3788de74bcb14536 \
         certificate 719be2de095b4c9376fa4d363b44077151b79d6b74c949d6bd7bbbb1034e5f7f \
         record 76442bc3c499e90cbd028564967efe1f44de22a0fca87f5c0fd43932549141d6 \
         score none"
    );
}

#[test]
fn heard_names_a_score_when_one_is_given() {
    let a = run(&fixture("pos-v04"), BUDGET);
    assert!(
        lines(&a)[1]
            .ends_with(" score 7425114d32c4f83891c34d24dd7ea150e0e28930bd1560b62f1ad5489ecded7f"),
        "{}",
        lines(&a)[1]
    );
}

/// Taut's vector 50: corpus 01's source, and 27's stdout split at its
/// `certificate` line (certificate §3.2 M1) into the record and the
/// certificate. echo's second line is the vector's, byte for byte.
#[test]
fn vector_50_heard_line_byte_for_byte() {
    let stdout = read("27-stdout-positive.stdout");
    let at = stdout
        .split_inclusive(|&b| b == b'\n')
        .take_while(|l| !l.starts_with(b"certificate "))
        .map(<[u8]>::len)
        .sum::<usize>();
    assert_eq!(at, 335, "27's split_at");
    let (record, certificate) = stdout.split_at(at);
    let source = read("01-determinism.pact-text");
    let a = replay::replay(
        &Inputs {
            source: Some(&source),
            certificate: Some(certificate),
            record: Some(record),
            score: None,
        },
        BUDGET,
    );
    let tune = String::from_utf8(read("50-heard-score-none.tune")).expect("utf-8");
    let want: Vec<&str> = tune.lines().collect();
    assert_eq!(lines(&a)[1], want[1]);
    assert_eq!(
        a.heard[0].map(|h| hex(&h)).as_deref(),
        Some("896d1b7bbedcdec8e1e55ab81393c17880e231787bf216cb9930610c2f8277b7")
    );
}

#[test]
fn heard_prints_none_for_an_input_that_could_not_be_read() {
    let f = fixture("pos-v01");
    let a = replay::replay(
        &Inputs {
            source: None,
            certificate: Some(&f.certificate),
            record: None,
            score: Some(None),
        },
        BUDGET,
    );
    let l = lines(&a);
    assert_eq!(l[0], "unknown io source record score");
    assert!(
        l[1].starts_with("heard source none certificate 719be2de"),
        "{}",
        l[1]
    );
    assert!(l[1].contains(" record none score none"), "{}", l[1]);
    assert_eq!(a.exit(), 2);
}

#[test]
fn every_input_unread_is_named_in_order() {
    let a = replay::replay(
        &Inputs {
            source: None,
            certificate: None,
            record: None,
            score: Some(None),
        },
        BUDGET,
    );
    assert_eq!(
        lines(&a)[..2],
        [
            "unknown io source certificate record score".to_string(),
            "heard source none certificate none record none score none".to_string()
        ]
    );
}

/// No score given is not an input that failed: `score none`, and no `io`.
#[test]
fn no_score_is_not_an_io_failure() {
    let a = run(&fixture("pos-v01"), BUDGET);
    assert!(!lines(&a)[0].starts_with("unknown io"));
    assert!(lines(&a)[1].ends_with(" score none"));
}

// ----------------------------------------------------------- implements

/// §6.3's third line: `implements spec <commit> vectors <tree> rig none`,
/// the ids full and lowercase (Q2: 40 hex in a SHA-1 repository).
#[test]
fn implements_line_names_a_spec_commit_and_a_vectors_tree() {
    let a = run(&fixture("pos-v01"), BUDGET);
    let l = lines(&a);
    let words: Vec<&str> = l[2].split(' ').collect();
    assert_eq!(words.len(), 7, "{}", l[2]);
    assert_eq!(
        (words[0], words[1], words[3], words[5], words[6]),
        ("implements", "spec", "vectors", "rig", "none")
    );
    for id in [words[2], words[4]] {
        assert_eq!(id.len(), 40, "{id}");
        assert!(
            id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
            "{id}"
        );
        assert_ne!(id, "0000000000000000000000000000000000000000");
    }
    assert_ne!(words[2], words[4]);
    assert_eq!(words[2], replay::SPEC);
    assert_eq!(words[4], replay::VECTORS);
}

/// §6.3's order, whatever the verdict: the verdict, `heard`, `implements`,
/// then one `interval` line per dissonance (none unless `dissonant`).
#[test]
fn the_verdict_then_heard_then_implements() {
    for budget in [BUDGET, 1] {
        let l = lines(&run(&fixture("pos-v01"), budget));
        assert_eq!(l.len(), 3, "{l:?}");
        assert!(l[1].starts_with("heard source "));
        assert!(l[2].starts_with("implements spec "));
    }
    let l = lines(&run(&fixture("a1-record-dark-disagrees"), BUDGET));
    assert_eq!(l.len(), 5, "{l:?}");
    assert_eq!(l[0], "dissonant 2");
    assert!(l[1].starts_with("heard source "));
    assert!(l[2].starts_with("implements spec "));
    assert!(l[3..].iter().all(|i| i.starts_with("interval ")));
}

// ------------------------------------------------------------ the floor

/// The key test: a certificate echo can't replay yet answers
/// `unknown unreplayed <stratum>`, never `rings`.
#[test]
fn what_echo_cannot_replay_is_unknown_never_rings() {
    let a = run(&fixture("pos-v01"), BUDGET);
    assert_eq!(a.verdict, Verdict::Unreplayed("contexts"));
    assert_eq!(lines(&a)[0], "unknown unreplayed contexts");
    assert_eq!(a.exit(), 2);
}

/// Every one of mirror's 38 checker fixtures, cheats (`ct-*`) and correct
/// ones alike: none rings, and each reads (typed-corpus-unheard's typed
/// constants included: S4). R2's answers are `r2::the_fixture_table`.
#[test]
fn no_fixture_rings() {
    for name in FIXTURES {
        let a = run(&fixture(name), BUDGET);
        assert_ne!(a.verdict, Verdict::Rings, "{name}");
        assert_ne!(a.exit(), 0, "{name}");
        assert!(!lines(&a)[0].starts_with("unknown unreadable"), "{name}");
    }
}

/// `rings` is reachable only through `conclude` with nothing left
/// unreplayed.
#[test]
fn rings_only_when_nothing_is_left_unreplayed() {
    assert_eq!(conclude(&[]), Verdict::Rings);
    assert_eq!(conclude(&STRATA), Verdict::Unreplayed("source"));
    assert_eq!(conclude(&STRATA[3..]), Verdict::Unreplayed("contexts"));
    assert_eq!(
        STRATA,
        ["source", "program", "model", "contexts", "verdict", "budget", "address", "score"]
    );
}

// ---------------------------------------------------------------- exits

fn answer(verdict: Verdict) -> Answer {
    Answer {
        verdict,
        heard: [None; 4],
    }
}

#[test]
fn exit_codes_and_first_lines() {
    let cases = [
        (Verdict::Rings, "rings", 0),
        (
            Verdict::Uncertain { budget: 7 },
            "uncertain budget 7 ground rule instances",
            1,
        ),
        (
            Verdict::Io(vec![Input::Certificate, Input::Score]),
            "unknown io certificate score",
            2,
        ),
        (
            Verdict::Unreadable {
                input: Input::Record,
                at: Position {
                    line: 3,
                    column: 13,
                },
            },
            "unknown unreadable record:3:13",
            2,
        ),
        (Verdict::Unreplayed("model"), "unknown unreplayed model", 2),
    ];
    for (verdict, first, exit) in cases {
        let a = answer(verdict);
        assert_eq!(lines(&a)[0], first);
        assert_eq!(a.exit(), exit, "{first}");
    }
}

// ----------------------------------------------------------- unreadable

fn with_line(text: &[u8], n: usize, new: &str) -> Vec<u8> {
    let text = String::from_utf8(text.to_vec()).expect("utf-8");
    let mut lines: Vec<&str> = text.lines().collect();
    lines[n - 1] = new;
    (lines.join("\n") + "\n").into_bytes()
}

fn unreadable(certificate: &[u8], record: &[u8]) -> Answer {
    let f = fixture("pos-v01");
    replay::replay(
        &Inputs {
            source: Some(&f.source),
            certificate: Some(certificate),
            record: Some(record),
            score: None,
        },
        BUDGET,
    )
}

/// pos-v01's certificate: line 9 is `layer        0 edge(a, b)`, line 23
/// the first `derive`. A refusal is at the least byte offset where the
/// line can't continue (certificate §3.1.1 J), so a separator cut short
/// is refused where it stops.
#[test]
fn an_unreadable_certificate_is_located() {
    let f = fixture("pos-v01");
    let cases = [
        (9, "layer        0 Edge(a, b)", "certificate:9:16"),
        (9, "layer        0 edge(a, b", "certificate:9:25"),
        (9, "layer        00 edge(a, b)", "certificate:9:14"),
        (9, "layer        sideways 0 edge(a, b)", "certificate:9:14"),
        (9, "layer", "certificate:9:6"),
        (9, "layers       0 edge(a, b)", "certificate:9:1"),
        (9, " layer       0 edge(a, b)", "certificate:9:1"),
        (9, "layer        0 edge(a, b)\r", "certificate:9:26"),
        (
            3,
            "result       C26727afd3de20d1ad4a0db758ce73a59882ff6a3dc149b3b12d2c50228fb0ee",
            "certificate:3:14",
        ),
        (3, "result       c26727af", "certificate:3:14"),
        (2, "certificate  2", "certificate:2:14"),
        (
            23,
            "derive       path(a, b) via path-base from edge(a, b)",
            "certificate:23:25",
        ),
        (
            23,
            "derive       path(a, b) by path-base from edge(a, b) &",
            "certificate:23:55",
        ),
        (
            23,
            "derive       path(a, b) by path-base from",
            "certificate:23:42",
        ),
        (
            23,
            "derive       path(a, b) by path-base from edge(a, b) ",
            "certificate:23:54",
        ),
    ];
    for (n, line, want) in cases {
        let a = unreadable(&with_line(&f.certificate, n, line), &f.record);
        assert_eq!(
            lines(&a)[0],
            format!("unknown unreadable {want}"),
            "{line:?}"
        );
        assert_eq!(a.exit(), 2);
    }
}

/// A header key is required once: dropping `facts` is refused after the
/// last line; repeating `result` is refused at the repeat.
#[test]
fn the_certificate_header_is_whole() {
    let f = fixture("pos-v01");
    let text = String::from_utf8(f.certificate.clone()).expect("utf-8");
    let dropped: String = text
        .lines()
        .filter(|l| !l.starts_with("facts "))
        .map(|l| format!("{l}\n"))
        .collect();
    let last = dropped.lines().count();
    let a = unreadable(dropped.as_bytes(), &f.record);
    assert_eq!(
        lines(&a)[0],
        format!("unknown unreadable certificate:{}:1", last + 1)
    );
    let repeated = with_line(&f.certificate, 4, "result       none");
    assert_eq!(
        lines(&unreadable(&repeated, &f.record))[0],
        "unknown unreadable certificate:4:1"
    );
}

/// The other line forms of §3.2, Note §6.5 R5 (`erasure`) and §6.7 H5
/// (`hole`) read; malformed ones are located.
#[test]
fn every_certificate_line_form() {
    let f = fixture("pos-v01");
    let reads = [
        "layer        under 0 p",
        "layer        over 12 q(a, nat 7)",
        "derive       under r by r",
        "derive       over t by t from p",
        "derive       under by by r",
        "context      under 0 = []",
        "context      over 1 = [p & q & r & t]",
        "dark         2 at 3:7",
        "dark         2 engine arity q",
        "dark         2 engine safety s ?x",
        "dark         2 engine declaration owner",
        "dark         255 functional owner(1) -> 2",
        "dark         255 functional p(1, 2) -> 3",
        "violating    owner(doc, alex) by functional owner(1) -> 2",
        "violating    owner(doc, alex)",
        "erasure      tick budget count 3 ticks",
        "erasure      over budget count 12 rounds",
        "hole         arity q",
        "hole         declaration p functional p(2) -> 1",
        "layer        0 span(bytes \"0a26\", addr 0a261ff8316b0156a71050625d4f9f10277f7364ab55253bd5de935b84e24e3c)",
    ];
    for line in reads {
        let a = unreadable(&with_line(&f.certificate, 9, line), &f.record);
        assert!(
            !lines(&a)[0].starts_with("unknown unreadable"),
            "{line}: {}",
            lines(&a)[0]
        );
        assert_ne!(a.verdict, Verdict::Rings, "{line}");
    }
    let refused = [
        ("context      under 0 = [p & ]", 29),
        ("context      under 0 = p", 24),
        ("context      sideways 0 = []", 14),
        ("dark         2 at 3", 20),
        ("dark         2 engine colour q", 23),
        ("dark         3 at 1:1", 14),
        ("dark         255 functional owner(1) => 2", 38),
        ("erasure      tick budget count 0 ticks", 32),
        ("erasure      tick budget count 3 beats", 34),
        ("erasure      sideways budget count 3 ticks", 14),
        ("hole", 5),
        ("violating    owner(doc, alex) by", 33),
    ];
    for (line, column) in refused {
        let a = unreadable(&with_line(&f.certificate, 9, line), &f.record);
        assert_eq!(
            lines(&a)[0],
            format!("unknown unreadable certificate:9:{column}"),
            "{line}"
        );
    }
}

/// pos-v01's record: line 1 is `verdict     clear`.
#[test]
fn an_unreadable_record_is_located() {
    let f = fixture("pos-v01");
    let cases = [
        (1, "verdict     maybe", "record:1:13"),
        (1, "verdict", "record:1:8"),
        (2, "colour      \"blue\"", "record:2:1"),
        (2, "file        \"source.pact\"\t", "record:2:26"),
        (8, "true        path(a, b)\u{e9}", "record:8:23"),
    ];
    for (n, line, want) in cases {
        let a = unreadable(&f.certificate, &with_line(&f.record, n, line));
        assert_eq!(
            lines(&a)[0],
            format!("unknown unreadable {want}"),
            "{line:?}"
        );
    }
    // No verdict at all: refused after the last line.
    let text = String::from_utf8(f.record.clone()).expect("utf-8");
    let rest: String = text.lines().skip(1).map(|l| format!("{l}\n")).collect();
    let n = rest.lines().count();
    let a = unreadable(&f.certificate, rest.as_bytes());
    assert_eq!(
        lines(&a)[0],
        format!("unknown unreadable record:{}:1", n + 1)
    );
}

/// A `hole`/`dark` value's text after its first ` #` is opaque bytes
/// (Note §6.4 (a)): non-ASCII there reads.
#[test]
fn record_hole_text_is_opaque() {
    let f = fixture("pos-v01");
    let a = unreadable(
        &f.certificate,
        &with_line(&f.record, 8, "hole        arity q #\u{201c}B\u{201d} \t \r"),
    );
    // It reads, and pos-v01's source has no `arity q` hole (R2).
    assert_eq!(lines(&a)[0], "dissonant 1");
    let a = unreadable(
        &f.certificate,
        &with_line(&f.record, 8, "hole        arity \u{e9} #q"),
    );
    assert_eq!(lines(&a)[0], "unknown unreadable record:8:19");
}

/// The order of judging: `io` before `unreadable` before the budget.
#[test]
fn precedence() {
    let f = fixture("pos-v01");
    let bad = with_line(&f.certificate, 9, "layer        0 Edge(a, b)");
    let a = replay::replay(
        &Inputs {
            source: Some(&f.source),
            certificate: Some(&bad),
            record: None,
            score: None,
        },
        BUDGET,
    );
    assert_eq!(lines(&a)[0], "unknown io record");
    let a = replay::replay(
        &Inputs {
            source: Some(&f.source),
            certificate: Some(&bad),
            record: Some(&f.record),
            score: None,
        },
        0,
    );
    assert_eq!(lines(&a)[0], "unknown unreadable certificate:9:16");
    // The certificate is read before the record.
    let bad_record = with_line(&f.record, 1, "verdict     maybe");
    assert_eq!(
        lines(&unreadable(&bad, &bad_record))[0],
        "unknown unreadable certificate:9:16"
    );
}

/// The source is never `unreadable`: a source that doesn't read is a
/// Dark-2 source (Note §5, Seam row 8).
#[test]
fn an_unreadable_source_is_not_unreadable() {
    let f = fixture("pos-v01");
    let a = replay::replay(
        &Inputs {
            source: Some(b"fact Edge(a\t"),
            certificate: Some(&f.certificate),
            record: Some(&f.record),
            score: None,
        },
        BUDGET,
    );
    // A reader-Dark source whose certificate claims P and F: check 1's
    // two `source` intervals (R3). Check 2 is still open (Q26).
    assert_eq!(lines(&a)[0], "dissonant 2");
    assert_eq!(a.exit(), 1);
}

// --------------------------------------------------------------- budget

#[test]
fn the_budget_is_pinned() {
    assert_eq!(BUDGET, 10_000_000);
}

/// From R4 the budget is charged where echo's own run forms a ground rule
/// instance (R1 charged one per printed `derive` line): pos-v01's naive
/// run forms 38. A budget of 38 is enough; 37, 9, 1 and 0 run out.
/// Counted, so the boundary is exact and every run gives the same bytes.
#[test]
fn the_budget_counts_ground_rule_instances() {
    let f = fixture("pos-v01");
    assert_eq!(lines(&run(&f, 38))[0], "unknown unreplayed contexts");
    for budget in [37, 9, 1, 0] {
        let a = run(&f, budget);
        assert_eq!(a.verdict, Verdict::Uncertain { budget });
        assert_eq!(
            lines(&a)[0],
            format!("uncertain budget {budget} ground rule instances")
        );
        assert_eq!(a.exit(), 1);
    }
    let first = run(&f, 37).print();
    for _ in 0..50 {
        assert_eq!(run(&f, 37).print(), first);
    }
}

/// A budget of 1 runs out on exactly the real inputs whose source's run
/// forms more than one ground rule instance, whatever their certificates
/// print.
#[test]
fn a_budget_of_one_runs_out_on_real_inputs() {
    let mut ran_out = 0;
    for name in FIXTURES {
        let f = fixture(name);
        let s = echo::source::read(&f.source);
        let report = echo::dark2::report(&s);
        let mut meter = echo::model::Meter::new(BUDGET);
        echo::model::replay(&s, &report, &mut meter).expect("budget");
        let a = run(&f, 1);
        if meter.spent() > 1 {
            assert_eq!(
                lines(&a)[0],
                "uncertain budget 1 ground rule instances",
                "{name}"
            );
            ran_out += 1;
        } else {
            assert!(!lines(&a)[0].starts_with("uncertain"), "{name}");
            assert_ne!(a.verdict, Verdict::Rings, "{name}");
        }
    }
    // The 22 positive sources whose run forms two or more instances
    // (`r4::the_budget_on_every_fixture`); W sources, Dark ones, ct-a6
    // (its rule unsafe, left out) and ct-a9 (no premise ever holds) form
    // fewer.
    assert_eq!(ran_out, 22);
}

// ---------------------------------------------------------- addresses

fn claimed_result(f: &Fixture) -> Option<String> {
    let text = String::from_utf8(f.certificate.clone()).expect("utf-8");
    text.lines()
        .find_map(|l| l.strip_prefix("result       "))
        .filter(|v| *v != "none")
        .map(str::to_string)
}

/// The model's address, recomputed from the certificate's layers under H
/// (`fragment(@pact/fact root, "model", sorted)`; certificate §3.1, §3.1.1
/// A), equals the claimed `result` on Taut's certificates and the correct
/// fixtures: positive, W (the under-run) and typed.
#[test]
fn the_model_address_recomputes_from_the_layers() {
    for name in [
        "pos-v01",
        "pos-diamond",
        "pos-v03",
        "pos-v04",
        "pos-v05",
        "pos-v05-control",
        "taut-20",
        "taut-21",
        "taut-23",
        "typed-corpus-unheard",
    ] {
        let f = fixture(name);
        let p = echo::certificate::read(&f.certificate).expect(name);
        assert_eq!(p.model().map(|m| hex(&m)), claimed_result(&f), "{name}");
    }
}

/// CT-A11: a model address made with a different sort order. echo's
/// address differs from the claim (R8 prints it as `address`).
#[test]
fn ct_a11_claims_another_model_address() {
    let f = fixture("ct-a11-model-text-sorted");
    let p = echo::certificate::read(&f.certificate).expect("reads");
    let mine = p.model().map(|m| hex(&m));
    assert!(mine.is_some());
    assert_ne!(mine, claimed_result(&f));
    assert_eq!(
        mine,
        claimed_result(&fixture("pos-v01")),
        "the same model as vector 01"
    );
}

/// A repeated layer atom makes no set (fixpoint §5.0.1 item 2): no model
/// address.
#[test]
fn a_repeated_model_atom_has_no_address() {
    let f = fixture("pos-v01");
    let twice = with_line(&f.certificate, 10, "layer        0 edge(a, b)");
    assert_eq!(
        echo::certificate::read(&twice).expect("reads").model(),
        None
    );
}
