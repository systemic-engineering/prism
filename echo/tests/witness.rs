//! The witness (docs/spec/echo.md §11; prism-025 Addendum item 3): Mara's
//! address + key + pin + re-derivation + coverage (052e5a13 §1.1, on Seam
//! SF §5), the answers {accept, refuse, not now} apart from the roll's
//! {true, maybe, false} (§1.2), beside the roll (mirror-029 §6).
//!
//! Without `--witness` the parity surface is byte for byte what it was
//! before §11 (prism-022 S2): [`PARITY`] pins every fixture's stdout and
//! exit, as BLAKE3 of the stdout followed by `exit <code>\n`, taken from
//! the binary at `982d945` (the §11 spec commit, R4's code).

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command, Output};

use echo::address::{self, Address};
use echo::replay::{self, Input, Inputs, Verdict, BUDGET, STRATA};
use echo::witness::{self, Reason, Response};
use echo::Position;

fn data(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/echo-vectors/mirror")
        .join(rel)
}

fn hex(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

/// A fixture's paths, its `expect` redirections followed (as `replay.rs`).
struct Paths {
    source: PathBuf,
    certificate: PathBuf,
    record: PathBuf,
    score: Option<PathBuf>,
}

impl Paths {
    fn list(&self) -> Vec<&PathBuf> {
        let mut v = vec![&self.source, &self.certificate, &self.record];
        v.extend(self.score.as_ref());
        v
    }
}

fn paths(name: &str) -> Paths {
    let expect = std::fs::read_to_string(data(&format!("fixtures/{name}/expect"))).expect("expect");
    let path = |key: &str, default: &str| -> Option<PathBuf> {
        let given = expect
            .lines()
            .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(' ')))
            .map(str::trim);
        match given {
            Some(p) if p.starts_with("../../vectors/") => Some(data(&p["../../vectors/".len()..])),
            Some(p) => Some(data(&format!("fixtures/{name}/{p}"))),
            None if default.is_empty() => None,
            None => Some(data(&format!("fixtures/{name}/{default}"))),
        }
    };
    Paths {
        source: path("source", "source.pact").expect("source"),
        certificate: path("certificate", "certificate").expect("certificate"),
        record: path("record", "record").expect("record"),
        score: path("score", ""),
    }
}

fn prism(flag: bool, files: &[&PathBuf]) -> Output {
    let mut args: Vec<OsString> = vec!["echo".into()];
    if flag {
        args.push("--witness".into());
    }
    args.extend(files.iter().map(|p| p.as_os_str().to_os_string()));
    Command::new(env!("CARGO_BIN_EXE_prism"))
        .args(&args)
        .output()
        .expect("runs")
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).expect("utf-8")
}

/// The witness record of a `--witness` run: the lines after the seam.
fn record(o: &Output) -> Vec<String> {
    let text = stdout(o);
    let (_, r) = text.split_once("\n---\n").expect("a seam");
    assert!(r.ends_with('\n'), "every line ends in LF: {r:?}");
    r.lines().map(str::to_string).collect()
}

fn line<'a>(r: &'a [String], key: &str) -> &'a str {
    let found: Vec<&String> = r.iter().filter(|l| l.starts_with(key)).collect();
    assert_eq!(found.len(), 1, "one {key:?} line in {r:#?}");
    &found[0][key.len()..]
}

fn starting<'a>(r: &'a [String], key: &str) -> Vec<&'a str> {
    r.iter().filter_map(|l| l.strip_prefix(key)).collect()
}

fn b3(p: &PathBuf) -> String {
    hex(&address::hash(&std::fs::read(p).expect("fixture")))
}

/// Every fixture's parity output before §11: BLAKE3 of stdout ‖ `exit N\n`.
const PARITY: [(&str, &str); 38] = [
    (
        "a1-record-dark-disagrees",
        "bebcf4aa3cd6031de76e53158c0829fae9d58eec5f7a35928005cd218e6ce2f8",
    ),
    (
        "a1-record-dark-engine",
        "aa9486b2e93228aa29eb2c775d5abe04b6bfcd422d96f1368a15f435d3902c20",
    ),
    (
        "a1-record-dark-in-clear",
        "34f9a90597ba4666742916e04613571a6eb69284d19d8ce4a76e5fafbf5907de",
    ),
    (
        "a1-record-dark-reader",
        "ebe96bbd0d6cca95275495863e781ec5d52c16ccbec100c964ab92f5b7cbd798",
    ),
    (
        "a8-budget-query-unknown",
        "6e88b1e7a372d3ab5b960f06d78c4f01e1e0663c128048a7ffe651704750a682",
    ),
    (
        "ct-a1-stable-model-witness",
        "f1e4d489e66a75f0892fc874590c102a366eb92c68537b0a98f6c2460c19e477",
    ),
    (
        "ct-a10-truncated-claims-fixed",
        "0577663d93edddac337a40aacc5cd02f4a9c96d6cb452ef1974652b18aa8000f",
    ),
    (
        "ct-a11-model-text-sorted",
        "93bc683a11e6d270313e0859d7af9747203b6a19ad044f687e6dc1e694fd8b24",
    ),
    (
        "ct-a12-budget-stop",
        "535e75547bd5fc5049652f63374493f7ad9e0b90f0c7a7f326e8d8df8a36411d",
    ),
    (
        "ct-a2-missing-derivation",
        "bd3586ef5ecdc2a89c15c15912c902bdac41bffc8ae7ae9b7d8c050c3a651344",
    ),
    (
        "ct-a3-clear-hides-functional",
        "3aaff1e3fabae7800a6dd978a5339eb84c5ba572dccbe85caf6b3891aeed88ec",
    ),
    (
        "ct-a4-spurious-fact",
        "77c20e08c01ce01194c5aaec61b7bbd8ea88fbe36d5f4806f4a7f829be39dc0b",
    ),
    (
        "ct-a5-readable-dark",
        "1336954f046b3864deea54b7b328246643ab7b3c08da05ddaabb97a5c7ffabeb",
    ),
    (
        "ct-a6-unsafe-with-model",
        "fa8a738260e7e6b4821297f910eefd70b911c8f6f5fee7e55459250a12c9229b",
    ),
    (
        "ct-a8-layer-off-by-one",
        "0b5d933b07521d9b39cee470189faa258625165c9f185998649edc8a8f5ffc69",
    ),
    (
        "ct-a9-circular-pair",
        "cda383e505aa54f6651ddc5c4103ff16cfd0194de0124db03a37adaa0327e236",
    ),
    (
        "dark2-not-canonical",
        "6bb7c4dba500e2b566c0833e9660ce0ad40430d7df4bdf2f89bb1a8019c0b01f",
    ),
    (
        "e1-false-budget-clear",
        "f8acaed9d4c26f05691abe957ffeeca7e13fc4aaffdf783085c58f59c94c94ab",
    ),
    (
        "e1-false-budget-opaque",
        "4a780d7a24565c4e29948e039ba1f6556fc5c3f01bfeea9fc319f84eae9fa409",
    ),
    (
        "e4-dark-order",
        "b23bfb0b29e42cb4ce1e173271549f3de3753a922d553e3b26cf8580cc4e9989",
    ),
    (
        "p8-one-line",
        "19ae80a68c16491f5b37baee5cc4e22f271589394c5f12886fab6b8bf237f133",
    ),
    (
        "p8-swapped",
        "59ee18ee71bacfdb89bf131d7832a838529a230668d4a0debc3aad0083177fa2",
    ),
    (
        "pos-dark2-engine",
        "3fb4cbfa9ddd4267ecdc512f17445677c1f3917ee24ededeea13bfdccfbbbd73",
    ),
    (
        "pos-dark2-reader",
        "37311ff70398a94e91e96b52e76203ed81d73d8f349d52ac948cc41c53d053c9",
    ),
    (
        "pos-diamond",
        "c63bc20d1b859b9467edfea987acd4db42b28b1672af1ae79f16e95709a639ef",
    ),
    (
        "pos-v01",
        "075207c28d373de99c0fc14108ddd1e9f68d2c07c0663ee4d1cf1414c10f5415",
    ),
    (
        "pos-v03",
        "6d154cbecbc255259b569c6bfc8337c011e4690c7faa06b6fad1ba1fefa81243",
    ),
    (
        "pos-v04",
        "980b856482e850f3fd134b7513df1a3093a6dad17c1c73da16e57ecd42e0acd2",
    ),
    (
        "pos-v05",
        "560420861132e2bd36ac8f0ce33f9dbdb5eef10f24e38bc518c2a420ed65ced7",
    ),
    (
        "pos-v05-control",
        "778ee4a289db531fa62515bebcc59cdbcb286b5af1a240d1fcba797452b1776c",
    ),
    (
        "score-missing-note",
        "1b7130a4561c3fcb70851defb5676cc7d86903969c4db7edc72ee8b15b88851d",
    ),
    (
        "taut-20",
        "632daf9584cb2b4d90e2c3474dad2bda91413703af377b454931a1ec3674734f",
    ),
    (
        "taut-21",
        "9413304b0200843c780225b206b6140af1ddee1d0b0421bb0c475c9f1479a10f",
    ),
    (
        "taut-22",
        "18ba1e9e4b21e5fbc84e5b751a65399c06884525bc13c6e517cbf570843b499c",
    ),
    (
        "taut-23",
        "994b4d8f748aaf51fbc16c872c00a0fb99220588eb9c1b14167b74ee1df465d9",
    ),
    (
        "taut-24",
        "c89a3c2b5b7fb582a93eb50c19f90cf61649fd2b81becfdacdd40a8da903954c",
    ),
    (
        "taut-48",
        "9907981459b11ad410e57a72436b5e3e102535444e5302b07b5c28405ec0788b",
    ),
    (
        "typed-corpus-unheard",
        "bf1e391e13e0f6e4ec60d359a9efe4c571ecbcda79fbf1be651a2a54406a454f",
    ),
];

/// The 14 fixtures R2–R4 refuse (§10.4, §10.6, §10.8); the other 24 answer
/// `unknown unreplayed contexts`.
const REFUSED: [&str; 14] = [
    "a1-record-dark-disagrees",
    "a1-record-dark-in-clear",
    "ct-a10-truncated-claims-fixed",
    "ct-a12-budget-stop",
    "ct-a2-missing-derivation",
    "ct-a4-spurious-fact",
    "ct-a5-readable-dark",
    "ct-a6-unsafe-with-model",
    "ct-a8-layer-off-by-one",
    "ct-a9-circular-pair",
    "dark2-not-canonical",
    "e1-false-budget-clear",
    "e1-false-budget-opaque",
    "p8-one-line",
];

fn parity_digest(o: &Output) -> String {
    let mut bytes = o.stdout.clone();
    bytes.extend_from_slice(format!("exit {}\n", o.status.code().expect("exit")).as_bytes());
    hex(&address::hash(&bytes))
}

// ------------------------------------------------------------- parity

/// Without the flag, every fixture's stdout and exit are byte for byte
/// what they were before §11 (the regression guard), and the library
/// prints the same.
#[test]
fn without_the_flag_every_fixture_is_byte_identical() {
    for (name, want) in PARITY {
        let p = paths(name);
        let o = prism(false, &p.list());
        assert_eq!(parity_digest(&o), want, "{name}: {}", stdout(&o));
        assert!(o.stderr.is_empty(), "{name}");
    }
}

/// With the flag, the parity lines come first, unchanged, then the seam,
/// then the record; the exit is the verdict's.
#[test]
fn with_the_flag_the_parity_lines_come_first_then_the_seam() {
    for (name, _) in PARITY {
        let p = paths(name);
        let plain = prism(false, &p.list());
        let o = prism(true, &p.list());
        let text = stdout(&o);
        let (a, r) = text.split_at(plain.stdout.len());
        assert_eq!(a.as_bytes(), &plain.stdout[..], "{name}");
        assert!(r.starts_with("---\nwitness 1\n"), "{name}: {r}");
        assert_eq!(o.status.code(), plain.status.code(), "{name}");
        assert!(!stdout(&plain).lines().any(|l| l == "---"), "{name}");
    }
}

/// The witness sits beside the roll (mirror-029 §6): the library's answer
/// with a witness is the answer without one, the `heard` certificate
/// address is the file's BLAKE3 either way, and the record's address is
/// that same address.
#[test]
fn the_witness_never_enters_the_certificate_address() {
    for (name, _) in PARITY {
        let p = paths(name);
        let bytes: Vec<Vec<u8>> = p
            .list()
            .iter()
            .map(|f| std::fs::read(f).expect("f"))
            .collect();
        let inputs = Inputs {
            source: Some(&bytes[0]),
            certificate: Some(&bytes[1]),
            record: Some(&bytes[2]),
            score: bytes.get(3).map(|b| Some(&b[..])),
        };
        let plain = replay::replay(&inputs, BUDGET);
        let (answer, w) = witness::witness(&inputs, BUDGET);
        assert_eq!(answer, plain, "{name}");
        assert_eq!(answer.heard[1], Some(address::hash(&bytes[1])), "{name}");
        let cert = b3(&p.certificate);
        let o = prism(true, &p.list());
        let r = record(&o);
        assert_eq!(line(&r, "address certificate "), cert, "{name}");
        let heard = stdout(&o).lines().nth(1).expect("heard").to_string();
        assert!(heard.contains(&format!(" certificate {cert} ")), "{name}");
        assert_eq!(w.print(), format!("---\n{}\n", r.join("\n")), "{name}");
    }
}

// ------------------------------------------------------------- the answer

/// rings → accept, dissonant → refuse, uncertain and unknown → not now,
/// keeping the verdict's words as the reason (§11.3).
#[test]
fn respond_maps_every_verdict() {
    let at = Position { line: 3, column: 4 };
    let cases: [(Verdict, bool, Response, &str); 8] = [
        (Verdict::Rings, true, Response::Accept, "accept"),
        (
            Verdict::Rings,
            false,
            Response::NotNow(Reason::Uncovered),
            "not now uncovered",
        ),
        (
            Verdict::Dissonant(Vec::new()),
            true,
            Response::Refuse,
            "refuse",
        ),
        (
            Verdict::Uncertain { budget: 7 },
            true,
            Response::NotNow(Reason::Budget(7)),
            "not now budget 7 ground rule instances",
        ),
        (
            Verdict::Io(vec![Input::Source, Input::Score]),
            true,
            Response::NotNow(Reason::Io(vec![Input::Source, Input::Score])),
            "not now io source score",
        ),
        (
            Verdict::Unreadable {
                input: Input::Record,
                at,
            },
            true,
            Response::NotNow(Reason::Unreadable {
                input: Input::Record,
                at,
            }),
            "not now unreadable record:3:4",
        ),
        (
            Verdict::Unreplayed("contexts"),
            true,
            Response::NotNow(Reason::Unreplayed("contexts")),
            "not now unreplayed contexts",
        ),
        (
            Verdict::Unreplayed("contexts"),
            false,
            Response::NotNow(Reason::Unreplayed("contexts")),
            "not now unreplayed contexts",
        ),
    ];
    for (verdict, covered, want, text) in cases {
        let got = witness::respond(&verdict, covered);
        assert_eq!(got, want, "{verdict:?}");
        assert_eq!(got.text(), text);
    }
}

/// Per fixture: the 14 R2–R4 refuse are `refuse`, the 24 others `not now
/// unreplayed contexts`; none is `accept` (the floor of echo).
#[test]
fn every_fixture_answers_by_the_mapping() {
    for (name, _) in PARITY {
        let p = paths(name);
        let o = prism(true, &p.list());
        let r = record(&o);
        let verdict = stdout(&o).lines().next().expect("verdict").to_string();
        let answer = line(&r, "answer ");
        if REFUSED.contains(&name) {
            assert!(verdict.starts_with("dissonant "), "{name}");
            assert_eq!(answer, "refuse", "{name}");
        } else {
            assert_eq!(verdict, "unknown unreplayed contexts", "{name}");
            assert_eq!(answer, "not now unreplayed contexts", "{name}");
        }
        assert_ne!(answer, "accept", "{name}");
    }
}

// ------------------------------------------------------------- the record

/// The record's lines, in §11.4's order, with the pin the `implements`
/// line's content word for word.
#[test]
fn the_record_is_in_order() {
    let p = paths("pos-v01");
    let o = prism(true, &p.list());
    let r = record(&o);
    let keys: Vec<&str> = r
        .iter()
        .map(|l| {
            let mut w = l.split(' ');
            match w.next().expect("key") {
                k @ ("covered" | "uncovered") if l != "uncovered rest" => {
                    if w.next() == Some("input") {
                        if k == "covered" {
                            "ci"
                        } else {
                            "ui"
                        }
                    } else if k == "covered" {
                        "cs"
                    } else {
                        "us"
                    }
                }
                k => k,
            }
        })
        .collect();
    let mut want = vec!["witness", "answer", "address", "key", "pin"];
    want.extend(["derived"; 3]);
    want.extend(["ci"; 3]);
    want.push("ui");
    want.extend(["cs"; 3]);
    want.extend(["us"; 5]);
    want.push("uncovered");
    assert_eq!(keys, want, "{r:#?}");
    let implements = stdout(&o).lines().nth(2).expect("implements").to_string();
    assert_eq!(
        format!("pin {}", line(&r, "pin ")),
        implements.replacen("implements", "pin", 1)
    );
    assert_eq!(r.last().map(String::as_str), Some("uncovered rest"));
}

/// The key is echo's own P and F (§10.6), which pos-v01's certificate
/// claims, and `scope none`.
#[test]
fn the_key_is_echo_s_program_and_facts() {
    let p = paths("pos-v01");
    let cert = std::fs::read_to_string(&p.certificate).expect("cert");
    let claim = |k: &str| {
        cert.lines()
            .find_map(|l| l.strip_prefix(k))
            .map(str::trim)
            .expect(k)
            .to_string()
    };
    let r = record(&prism(true, &p.list()));
    assert_eq!(
        line(&r, "key "),
        format!(
            "program {} facts {} scope none",
            claim("program "),
            claim("facts ")
        )
    );
}

// ------------------------------------------------------------- coverage

/// Coverage names exactly the inputs read, by their `heard` address, and
/// the strata replayed and decided; every later stratum is `unreplayed`;
/// a score is heard and never read; a stratum not decided is uncovered
/// with its outcome; the dissonant counts add up to the verdict's.
#[test]
fn coverage_names_the_inputs_read_and_the_strata_replayed() {
    for (name, _) in PARITY {
        let p = paths(name);
        let o = prism(true, &p.list());
        let r = record(&o);
        assert_eq!(
            starting(&r, "covered input "),
            vec![
                format!("source {}", b3(&p.source)),
                format!("certificate {}", b3(&p.certificate)),
                format!("record {}", b3(&p.record)),
            ],
            "{name}"
        );
        let score = match &p.score {
            Some(s) => format!("score {} unread", b3(s)),
            None => "score none absent".to_string(),
        };
        assert_eq!(starting(&r, "uncovered input "), vec![score], "{name}");
        let derived = starting(&r, "derived ");
        assert_eq!(derived.len(), 3, "{name}");
        let mut covered = Vec::new();
        let mut uncovered = Vec::new();
        let mut lines = 0;
        for (d, stratum) in derived.iter().zip(&STRATA[..3]) {
            let outcome = d.strip_prefix(&format!("{stratum} ")).expect("in order");
            if outcome == "agreed" {
                covered.push(stratum.to_string());
            } else if let Some(n) = outcome.strip_prefix("dissonant ") {
                match n.split_once(' ') {
                    Some((n, why)) => {
                        lines += n.parse::<usize>().expect("n");
                        uncovered.push(format!("{stratum} {why}"));
                    }
                    None => {
                        lines += n.parse::<usize>().expect("n");
                        covered.push(stratum.to_string());
                    }
                }
            } else {
                assert!(["waits", "open"].contains(&outcome), "{name}: {outcome}");
                uncovered.push(format!("{stratum} {outcome}"));
            }
        }
        for s in &STRATA[3..] {
            uncovered.push(format!("{s} unreplayed"));
        }
        assert_eq!(starting(&r, "covered stratum "), covered, "{name}");
        assert_eq!(starting(&r, "uncovered stratum "), uncovered, "{name}");
        let verdict = stdout(&o).lines().next().expect("v").to_string();
        let printed = verdict
            .strip_prefix("dissonant ")
            .map_or(0, |n| n.parse().expect("n"));
        assert_eq!(lines, printed, "{name}");
    }
}

/// A fixture that needs R5 or later is `not now`, and its coverage leaves
/// the contexts stratum out: pos-v01 (positive: R2–R4 agree) and pos-v04
/// (W, with a score: R4 waits, and the score is heard, not read).
#[test]
fn a_fixture_needing_r5_is_not_now_and_contexts_is_not_covered() {
    let r = record(&prism(true, &paths("pos-v01").list()));
    assert_eq!(line(&r, "answer "), "not now unreplayed contexts");
    assert_eq!(
        starting(&r, "derived "),
        vec!["source agreed", "program agreed", "model agreed"]
    );
    assert_eq!(
        starting(&r, "covered stratum "),
        vec!["source", "program", "model"]
    );
    assert!(r.contains(&"uncovered stratum contexts unreplayed".to_string()));
    assert!(!r.iter().any(|l| l.starts_with("covered stratum contexts")));

    let p = paths("pos-v04");
    let r = record(&prism(true, &p.list()));
    assert_eq!(line(&r, "answer "), "not now unreplayed contexts");
    assert_eq!(starting(&r, "derived ")[2], "model waits");
    assert!(!starting(&r, "covered stratum ").contains(&"model"));
    assert!(r.contains(&"uncovered stratum model waits".to_string()));
    assert!(r.contains(&"uncovered stratum contexts unreplayed".to_string()));
    let score = p.score.as_ref().expect("pos-v04 has a score");
    assert!(r.contains(&format!("uncovered input score {} unread", b3(score))));
}

/// An input echo didn't read is not covered: no score given is `absent`;
/// an input that can't be read is `io`, and nothing is read or replayed;
/// an unreadable certificate is refused at its byte and the rest stops; a
/// budget that runs out reads every input and decides no stratum.
#[test]
fn an_input_not_read_is_not_covered() {
    let p = paths("pos-v01");
    let r = record(&prism(true, &p.list()));
    assert!(r.contains(&"uncovered input score none absent".to_string()));

    let missing = data("fixtures/pos-v01/no-such-certificate");
    let o = prism(true, &[&p.source, &missing, &p.record]);
    let r = record(&o);
    assert_eq!(line(&r, "answer "), "not now io certificate");
    assert_eq!(line(&r, "address certificate "), "none");
    assert_eq!(line(&r, "key "), "program none facts none scope none");
    assert!(starting(&r, "covered ").is_empty(), "{r:#?}");
    assert!(starting(&r, "derived ").is_empty(), "{r:#?}");
    assert_eq!(
        starting(&r, "uncovered input "),
        vec![
            format!("source {} stopped", b3(&p.source)),
            "certificate none io".to_string(),
            format!("record {} stopped", b3(&p.record)),
            "score none absent".to_string(),
        ]
    );
    let mut stopped: Vec<String> = STRATA[..3].iter().map(|s| format!("{s} stopped")).collect();
    stopped.extend(STRATA[3..].iter().map(|s| format!("{s} unreplayed")));
    assert_eq!(starting(&r, "uncovered stratum "), stopped);

    let bad = b"certificate 2\n".to_vec();
    let src = std::fs::read(&p.source).expect("s");
    let rec = std::fs::read(&p.record).expect("r");
    let inputs = Inputs {
        source: Some(&src),
        certificate: Some(&bad),
        record: Some(&rec),
        score: None,
    };
    let (answer, w) = witness::witness(&inputs, BUDGET);
    let at = match &answer.verdict {
        Verdict::Unreadable {
            input: Input::Certificate,
            at,
        } => at.to_string(),
        other => format!("not unreadable: {other:?}"),
    };
    assert_eq!(answer.exit(), 2, "{at}");
    let r: Vec<String> = w.print().lines().skip(1).map(str::to_string).collect();
    assert_eq!(
        line(&r, "answer "),
        format!("not now unreadable certificate:{at}")
    );
    assert_eq!(
        starting(&r, "uncovered input "),
        vec![
            format!("source {} stopped", hex(&address::hash(&src))),
            format!("certificate {} unreadable {at}", hex(&address::hash(&bad))),
            format!("record {} stopped", hex(&address::hash(&rec))),
            "score none absent".to_string(),
        ]
    );
    assert_eq!(starting(&r, "uncovered stratum "), stopped);

    let cert = std::fs::read(&p.certificate).expect("c");
    let inputs = Inputs {
        source: Some(&src),
        certificate: Some(&cert),
        record: Some(&rec),
        score: None,
    };
    let (answer, w) = witness::witness(&inputs, 1);
    assert_eq!(answer.verdict, Verdict::Uncertain { budget: 1 });
    let r: Vec<String> = w.print().lines().skip(1).map(str::to_string).collect();
    assert_eq!(
        line(&r, "answer "),
        "not now budget 1 ground rule instances"
    );
    assert_eq!(starting(&r, "covered input ").len(), 3);
    assert!(starting(&r, "covered stratum ").is_empty());
    assert!(starting(&r, "derived ").is_empty());
    assert_eq!(starting(&r, "uncovered stratum "), stopped);
}

/// The witness is Mara's five clauses and nothing about the roll: no
/// line reads `true`, `maybe` or `false`, and the answer is never one of
/// them.
#[test]
fn the_answer_is_never_a_roll_value() {
    for (name, _) in PARITY {
        let r = record(&prism(true, &paths(name).list()));
        let answer = line(&r, "answer ");
        for roll in ["true", "maybe", "false"] {
            assert!(!answer.starts_with(roll), "{name}");
        }
    }
}

/// The flag is the second word only; anywhere else it is a path, and a
/// flag with too few paths is usage.
#[test]
fn the_flag_is_the_second_word_only() {
    let p = paths("pos-v01");
    let o = Command::new(env!("CARGO_BIN_EXE_prism"))
        .args([OsStr::new("--witness"), OsStr::new("echo")])
        .args(p.list())
        .output()
        .expect("runs");
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    let o = prism(true, &[&p.source, &p.certificate]);
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    assert_eq!(
        String::from_utf8(o.stderr).expect("utf-8"),
        "usage: prism echo <source> <certificate> <record> [<score>]\n"
    );
}
