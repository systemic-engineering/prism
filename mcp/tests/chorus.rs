//! The chorus tool (docs/spec/mcp.md §5.2) on chorus's AND checkpoint
//! (docs/spec/mcp-vectors/checkpoint.json, checked against chorus's own
//! seeded signers by chorus/tests/checkpoint.rs), read with this crate's
//! own reader.

use prism_mcp::json::{print, read, Value};

use crate::common::{b3, call, code, get, is_error, lit, ready, repo, str_of, texts};

struct Vector {
    note: String,
    ed25519: String,
    mldsa44: String,
    inclusion: String,
}

fn vector() -> Vector {
    let bytes = std::fs::read(repo("docs/spec/mcp-vectors/checkpoint.json")).expect("vector");
    let v = read(&bytes).expect("reads");
    Vector {
        note: str_of(get(&v, &["note"])).into(),
        ed25519: str_of(get(&v, &["ed25519"])).into(),
        mldsa44: str_of(get(&v, &["mldsa44"])).into(),
        inclusion: print(get(&v, &["inclusion"])),
    }
}

fn arguments(note: &str, ed: &str, ml: &str, inclusion: Option<&str>) -> String {
    let mut a = format!(
        r#"{{"note":{},"ed25519":{},"mldsa44":{}"#,
        lit(note),
        lit(ed),
        lit(ml)
    );
    if let Some(i) = inclusion {
        a.push_str(&format!(r#","inclusion":{i}"#));
    }
    a.push('}');
    a
}

/// The one text of a chorus result, as lines.
fn lines(a: &Value) -> Vec<String> {
    let t = texts(a);
    assert_eq!(t.len(), 1, "{t:?}");
    assert!(t[0].ends_with('\n'));
    t[0].lines().map(str::to_string).collect()
}

fn run(args: &str) -> Value {
    ready(&[call(1, "chorus", args)]).remove(0)
}

/// Flip one base64 character of a signature line, past the key ID and (for
/// ML-DSA-44) the timestamp, keeping the base64 canonical.
fn corrupt(note: &str, which: usize) -> String {
    let mut lines: Vec<String> = note.split_inclusive('\n').map(str::to_string).collect();
    let sig: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].starts_with('\u{2014}'))
        .collect();
    let line = &mut lines[sig[which]];
    let at = line.rfind(' ').unwrap() + 40;
    let c = line.as_bytes()[at];
    let to = if c == b'A' { "B" } else { "A" };
    line.replace_range(at..at + 1, to);
    lines.concat()
}

const ROOT: &str = "XcnaeacGWamtVZy3Ad7ZoqudgjqtL0lgz+Nw7/RgQyg=";

#[test]
fn the_and_checkpoint_accepts_with_its_inclusion() {
    let v = vector();
    let a = run(&arguments(
        &v.note,
        &v.ed25519,
        &v.mldsa44,
        Some(&v.inclusion),
    ));
    assert!(!is_error(&a));
    let l = lines(&a);
    let leaf = [0x40, 0x41, 0x42, 0x43];
    let path: Vec<u8> = [
        "bc1a0643b12e4d2d7c77918f44e0f4f79a838b6cf9ec5b5c283e1f4d88599e6b",
        "ca854ea128ed050b41b35ffc1b87b8eb2bde461e9e3b5596ece6b9d5975a0ae0",
        "d37ee418976dd95753c1c73862b9398fa2a2cf9b4ff0fdfe8b30cd95209614b7",
    ]
    .iter()
    .flat_map(|h| {
        (0..64)
            .step_by(2)
            .map(move |i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
    })
    .collect();
    assert_eq!(
        l,
        [
            "accept".to_string(),
            format!(
                "heard note {} ed25519 {} mldsa44 {} leaf {} path {}",
                b3(v.note.as_bytes()),
                b3(v.ed25519.as_bytes()),
                b3(v.mldsa44.as_bytes()),
                b3(&leaf),
                b3(&path)
            ),
            format!("checkpoint example.com/prism-ledger 8 {ROOT}"),
            "timestamp 1763400000".into(),
            "inclusion 5 accept".into(),
        ]
    );
}

#[test]
fn the_and_checkpoint_accepts_without_an_inclusion() {
    let v = vector();
    let l = lines(&run(&arguments(&v.note, &v.ed25519, &v.mldsa44, None)));
    assert_eq!(l[0], "accept");
    assert!(l[1].ends_with(" leaf none path none"));
    assert_eq!(l[4], "inclusion none");
}

/// The AND rule: one corrupted signature, either one, rejects.
#[test]
fn one_corrupted_signature_rejects() {
    let v = vector();
    for (which, want) in [
        (0, "reject unsigned ed25519 false mldsa44 true"),
        (1, "reject unsigned ed25519 true mldsa44 false"),
    ] {
        let note = corrupt(&v.note, which);
        assert_ne!(note, v.note);
        let a = run(&arguments(
            &note,
            &v.ed25519,
            &v.mldsa44,
            Some(&v.inclusion),
        ));
        assert!(!is_error(&a), "a rejected subject is an answer");
        let l = lines(&a);
        assert_eq!(l[0], want);
        assert_eq!(l.len(), 3, "{l:?}");
        assert_eq!(l[2], "inclusion 5 unchecked");
    }
}

#[test]
fn a_note_that_is_not_a_checkpoint_rejects() {
    let v = vector();
    let cases = [
        ("".to_string(), "reject note malformed"),
        (v.note.replacen("\n8\n", "\n08\n", 1), "reject body"),
        (v.note.replacen("\n\n", "\next\n\n", 1), "reject extensions"),
    ];
    for (note, want) in cases {
        let l = lines(&run(&arguments(&note, &v.ed25519, &v.mldsa44, None)));
        assert_eq!(l[0], want, "{note:?}");
    }
}

#[test]
fn a_wrong_inclusion_rejects() {
    let v = vector();
    let wrong = v.inclusion.replace(r#""index":5"#, r#""index":4"#);
    let l = lines(&run(&arguments(
        &v.note,
        &v.ed25519,
        &v.mldsa44,
        Some(&wrong),
    )));
    assert_eq!(l[0], "reject inclusion");
    assert_eq!(
        l[2],
        format!("checkpoint example.com/prism-ledger 8 {ROOT}")
    );
    assert_eq!(l[4], "inclusion 4 reject root-mismatch");
    let far = v.inclusion.replace(r#""index":5"#, r#""index":8"#);
    let l = lines(&run(&arguments(
        &v.note,
        &v.ed25519,
        &v.mldsa44,
        Some(&far),
    )));
    assert_eq!(l[4], "inclusion 8 reject index-out-of-range");
}

/// A broken ruler gives no verdict: `isError`, not `reject`.
#[test]
fn a_bad_key_or_bytes_is_a_tool_error() {
    let v = vector();
    let short_path = r#"{"index":5,"leaf":"QEFCQw==","path":["AAAA"]}"#;
    let cases = [
        (
            arguments(&v.note, "nope", &v.mldsa44, None),
            "error ed25519 vkey",
        ),
        (
            arguments(&v.note, &v.mldsa44, &v.ed25519, None),
            "error ed25519 key",
        ),
        (
            arguments(&v.note, &v.ed25519, &v.ed25519, None),
            "error mldsa44 key",
        ),
        (
            arguments(
                &v.note,
                &v.ed25519,
                &v.mldsa44,
                Some(r#"{"index":5,"leaf":"QEFCQw=","path":[]}"#),
            ),
            "error inclusion.leaf base64",
        ),
        (
            arguments(&v.note, &v.ed25519, &v.mldsa44, Some(short_path)),
            "error inclusion.path 0 length",
        ),
    ];
    for (args, want) in cases {
        let a = run(&args);
        assert!(is_error(&a), "{want}");
        assert_eq!(texts(&a), [format!("{want}\n")]);
    }
}

/// §3.2: the index is an integer; an exponent there is refused, located.
#[test]
fn the_index_is_an_integer() {
    let v = vector();
    for (index, data) in [
        ("5e0", Some("not-integer")),
        ("5.0", Some("not-integer")),
        ("-1", None),
        ("\"5\"", None),
    ] {
        let inc = v
            .inclusion
            .replace(r#""index":5"#, &format!(r#""index":{index}"#));
        let line = call(
            1,
            "chorus",
            &arguments(&v.note, &v.ed25519, &v.mldsa44, Some(&inc)),
        );
        let a = ready(std::slice::from_ref(&line)).remove(0);
        assert_eq!(code(&a), -32602, "{index}");
        let d = get(&a, &["error", "data"]);
        assert_eq!(str_of(get(d, &["argument"])), "inclusion.index");
        match data {
            Some(word) => {
                assert_eq!(str_of(get(d, &["refusal"])), word);
                let at = line.find(&format!(r#""index":{index}"#)).unwrap() + 8;
                assert_eq!(get(d, &["at"]), &Value::Int(at as i64));
            }
            None => assert_eq!(str_of(get(d, &["expected"])), "integer ≥ 0"),
        }
    }
    for bad in [
        r#"{"index":5,"leaf":"QEFCQw=="}"#,
        r#"{"index":5,"leaf":"QEFCQw==","path":[1]}"#,
        r#"{"index":5,"leaf":"QEFCQw==","path":[],"x":1}"#,
        r#"[]"#,
    ] {
        let a = ready(&[call(
            1,
            "chorus",
            &arguments(&v.note, &v.ed25519, &v.mldsa44, Some(bad)),
        )])
        .remove(0);
        assert_eq!(code(&a), -32602, "{bad}");
    }
}
