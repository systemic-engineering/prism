//! mirror's data, read by prism's reader.
//!
//! docs/spec/listen-vectors/mirror holds, byte for byte from mirror
//! `reed/ship` (its README names the commit): the 46 `kind atom*` vectors of
//! `tests/checker/vectors` (Taut's §5.0 vectors, written from the spec text)
//! and `mirror.facts`, the pinned F_G (charter row `9092fca1…`).

use std::fs;
use std::path::PathBuf;

use listen::{read, Value};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/spec/listen-vectors/mirror")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One vector file's `key  value` lines.
fn fields(text: &str) -> Vec<(&str, &str)> {
    text.lines()
        .filter_map(|l| l.split_once(' ').map(|(k, v)| (k, v.trim_start())))
        .collect()
}

/// Every atom vector's `atom` reads as a `fact` line, prints back to the
/// vector's own canonical text, and, where the vector carries it, encodes
/// to its `content` bytes (fixpoint §5.0, §5.0.1 items 6, 10, 11).
#[test]
fn mirror_atom_vectors_read_print_back_and_encode() {
    let mut files: Vec<PathBuf> = fs::read_dir(dir())
        .expect("vectors")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "vector"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 46);
    let mut with_content = 0;
    for path in &files {
        let text = fs::read_to_string(path).expect("vector");
        let f = fields(&text);
        let atom_text = f.iter().find(|(k, _)| *k == "atom").expect("atom").1;
        let read = read(format!("fact {atom_text}\n").as_bytes())
            .unwrap_or_else(|r| panic!("{}: {r}", path.display()));
        assert_eq!(read.facts.len(), 1);
        let atom = &read.facts[0].atom;
        assert_eq!(atom.to_string(), atom_text, "{}", path.display());
        if let Some((_, content)) = f.iter().find(|(k, _)| *k == "content") {
            assert_eq!(hex(&atom.content()), *content, "{}", path.display());
            with_content += 1;
        }
    }
    assert_eq!(with_content, 7);
}

/// The pinned F_G reads whole: three headers, 539 facts, every argument
/// typed as written, and the canonical print reads back to the same atoms.
#[test]
fn mirror_facts_reads_whole() {
    let src = fs::read(dir().join("mirror.facts")).expect("mirror.facts");
    let f = read(&src).unwrap_or_else(|r| panic!("mirror.facts: {r}"));
    let keys: Vec<(&str, &str)> = f
        .headers
        .iter()
        .map(|h| (h.key.as_str(), h.value.as_str()))
        .collect();
    assert_eq!(
        keys,
        vec![
            ("format-grammar", "145e0f7c84579d400c894f74ef2d2e64d230185a"),
            (
                "root",
                "97c6e069da2f7a1656fbe0fa57fcba7d7fe8da5150201bf04a81ef7fb985d528"
            ),
            (
                "grammar",
                "9092fca18d8bc728f7f3467329c4a01d4073de2194fb7bce939ca739938857ee"
            ),
        ]
    );
    assert_eq!(f.facts.len(), 539);
    let (mut addr, mut nat, mut bytes, mut sym) = (0, 0, 0, 0);
    for v in f.facts.iter().flat_map(|x| &x.atom.args) {
        match v {
            Value::Addr(_) => addr += 1,
            Value::Nat(_) => nat += 1,
            Value::Bytes(_) => bytes += 1,
            Value::Sym(_) => sym += 1,
        }
    }
    // counted in the file by its spelling: `addr ` 739, `nat ` 229, `bytes ` 33
    assert_eq!((addr, nat, bytes), (739, 229, 33));
    assert!(sym > 0, "bare names read as syms");
    // the first fact, as written
    assert_eq!(f.facts[0].line, 4);
    assert_eq!(
        f.facts[0].atom.to_string(),
        "child(addr d0cbd0eb9646486f87456df3d3bbe24987c0798de9f497ee4ec8b6bb0d4f61f9, nat 6, addr 553fdbdce708153cc9fb3176839191e985aee6cdf021efa9b00e7399d279c845)"
    );
    let printed: String = f
        .facts
        .iter()
        .map(|x| format!("fact {}\n", x.atom))
        .collect();
    let again = read(printed.as_bytes()).expect("canonical print reads");
    assert!(f
        .facts
        .iter()
        .zip(&again.facts)
        .all(|(a, b)| a.atom == b.atom));
    assert_eq!(again.facts.len(), 539);
}
