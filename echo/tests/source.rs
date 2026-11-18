//! Certificate replay, slice R2: the pact source reader
//! (docs/spec/echo.md §10.4).
//!
//! The grammar is Seam's pact-eval audit §1.2 (frozen), with the typed
//! constants `pact eval` reads in `fact` lines since trace-layer C6 (S4).
//! A line that doesn't read is located at the least byte offset where the
//! grammar can't continue (certificate §3.1.1 J): a byte outside 0x20–0x7E
//! is an error where it stands, and is never reported ahead of an earlier
//! grammar error on its line. Lines count comment and blank lines, and
//! columns count bytes.

use echo::source::{self, Literal, Pattern, Record, Term};
use echo::value::{Atom, Value};
use echo::Position;

/// The `L:C` of every line that doesn't read.
fn unread(text: &str) -> Vec<String> {
    source::read(text.as_bytes())
        .unread
        .iter()
        .map(|u| u.at.to_string())
        .collect()
}

/// One line that doesn't read, at column `c` of line 1.
fn refused_at(line: &str, c: usize) {
    assert_eq!(unread(line), vec![format!("1:{c}")], "{line:?}");
}

fn sym(s: &str) -> Value {
    Value::Sym(s.into())
}

// ------------------------------------------------------------ locations

/// Certificate §3.1.1 J and the fixtures' sources: the least offset where
/// the grammar can't continue.
#[test]
fn a_line_that_does_not_read_is_located_at_its_least_failing_offset() {
    refused_at("fact edge(a, B)", 14);
    refused_at("fact Q", 6);
    refused_at("fact p(a, B)", 11);
    refused_at("fact\tx", 5);
    // J: the TAB after `B` is not reported ahead of `B`.
    refused_at("fact edge(a, B)\t", 14);
}

/// pact-eval §1.5 item 5: one case per grammar rule broken.
#[test]
fn every_grammar_rule_broken_is_located() {
    let cases = [
        ("fact p\r", 7),       // CR
        ("fact p(a)\t", 10),   // TAB
        ("fact p(\u{e9})", 8), // non-ASCII
        ("fact p # c", 8),     // a trailing comment
        ("fact p()", 8),       // `p()`
        ("fact p(1)", 8),      // a numeric constant
        ("fact not", 9),       // `not` as a predicate
        ("fact requires(a)", 14),
        ("query p(?x)", 9),        // a non-ground query
        ("rule r p requires", 18), // an empty `requires`
        ("rule r p requires q,", 21),
        ("rule r p(?x) requires not", 26),
        ("rule r not requires q", 11),
        ("rule R p requires q", 6),
        ("rule r p need q", 10),
        ("rule r p(?X) requires q", 11),
        ("factx p", 5),
        ("facts p", 5),
        ("fac p", 4),
        ("q p", 2), // `q` could still be `query`
        ("x p", 1),
        (" fact p", 2),
        ("   x", 4),
        ("\tfact p", 1),
        ("# caf\u{e9}", 6), // a comment is printable ASCII
        ("functional p(0) -> 1", 14),
        ("functional p(1) -> 01", 20),
        ("functional p(1) => 2", 17),
        ("functional p -> 2", 13),
        ("functional p(1)->2", 16),
        ("fact p(a,b", 11),
    ];
    for (line, c) in cases {
        refused_at(line, c);
    }
}

/// Lines count comments and blanks; every line that doesn't read is its
/// own hole, and the rest still read (Note §6.4: composed).
#[test]
fn lines_count_comments_and_blanks_and_every_unread_line_is_kept() {
    let s = source::read(b"# c\n\n   \nfact Q\nfact p(a)\nfact\tx");
    assert_eq!(
        s.unread.iter().map(|u| u.at).collect::<Vec<_>>(),
        vec![
            Position { line: 4, column: 6 },
            Position { line: 6, column: 5 }
        ]
    );
    // Byte offsets in the whole text: `fact Q` starts at 9, `fact\tx` at 26.
    assert_eq!(
        s.unread.iter().map(|u| u.offset).collect::<Vec<_>>(),
        vec![14, 30]
    );
    assert_eq!(s.lines.len(), 1);
    assert_eq!(s.lines[0].line, 5);
    assert_eq!(s.lines[0].offset, 16);
}

// ---------------------------------------------------------- what reads

#[test]
fn the_four_record_forms_read() {
    let s = source::read(
        b"rule path-step path(?x, ?z) requires path(?x, ?y),edge(?y, ?z)  \n\
          rule r s(?x) requires q(?x), not t(?x)\n\
          fact  edge(a, b)\n\
          query p\n\
          functional owner(1, 2) -> 3\n",
    );
    assert!(s.unread.is_empty(), "{:?}", s.unread);
    let records: Vec<&Record> = s.lines.iter().map(|l| &l.record).collect();
    let var = |v: &str| Term::Var(v.into());
    assert_eq!(
        records[0],
        &Record::Rule {
            id: "path-step".into(),
            head: Pattern {
                pred: "path".into(),
                args: vec![var("x"), var("z")]
            },
            body: vec![
                Literal {
                    negative: false,
                    atom: Pattern {
                        pred: "path".into(),
                        args: vec![var("x"), var("y")]
                    }
                },
                Literal {
                    negative: false,
                    atom: Pattern {
                        pred: "edge".into(),
                        args: vec![var("y"), var("z")]
                    }
                },
            ],
        }
    );
    match records[1] {
        Record::Rule { body, .. } => {
            assert_eq!(
                body.iter().map(|l| l.negative).collect::<Vec<_>>(),
                vec![false, true]
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        records[2],
        &Record::Fact(Atom {
            pred: "edge".into(),
            args: vec![sym("a"), sym("b")]
        })
    );
    assert_eq!(
        records[3],
        &Record::Query(Atom {
            pred: "p".into(),
            args: vec![]
        })
    );
    assert_eq!(
        records[4],
        &Record::Functional {
            pred: "owner".into(),
            key: vec!["1".into(), "2".into()],
            value: "3".into(),
        }
    );
    assert_eq!(
        s.lines.iter().map(|l| l.line).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
}

/// A name that is a type word is a name (the tune reads it so): `p(nat)` is
/// `sym "nat"`. `not` is a negative literal only when a space follows.
#[test]
fn names_that_look_like_keywords() {
    let s = source::read(b"fact p(nat, bytes, int)\nrule r notary requires nothing\n");
    assert!(s.unread.is_empty(), "{:?}", s.unread);
    assert_eq!(
        s.lines[0].record,
        Record::Fact(Atom {
            pred: "p".into(),
            args: vec![sym("nat"), sym("bytes"), sym("int")]
        })
    );
    match &s.lines[1].record {
        Record::Rule { head, body, .. } => {
            assert_eq!(head.pred, "notary");
            assert!(!body[0].negative);
            assert_eq!(body[0].atom.pred, "nothing");
        }
        other => panic!("{other:?}"),
    }
}

// --------------------------------------------------------- typed facts

/// S4, beyond the tune: `fact` lines read typed constants (since
/// trace-layer C6), each kept with its type (fixpoint §5.0.1 item 6,
/// §16.10 item 1).
#[test]
fn typed_constants_read_in_facts() {
    let a64 = "0a261ff8316b0156a71050625d4f9f10277f7364ab55253bd5de935b84e24e3c";
    let line = format!(
        "fact k(bytes \"0a26\", addr {a64}, nat 7, sym \"x \\\"y\\\\\", nat 0, bytes \"\")"
    );
    let s = source::read(line.as_bytes());
    assert!(s.unread.is_empty(), "{:?}", s.unread);
    let mut addr = [0u8; 32];
    for (i, b) in addr.iter_mut().enumerate() {
        *b = u8::from_str_radix(&a64[2 * i..2 * i + 2], 16).expect("hex");
    }
    assert_eq!(
        s.lines[0].record,
        Record::Fact(Atom {
            pred: "k".into(),
            args: vec![
                Value::Bytes(vec![0x0a, 0x26]),
                Value::Addr(addr),
                Value::Nat(7),
                Value::Sym("x \"y\\".into()),
                Value::Nat(0),
                Value::Bytes(vec![]),
            ]
        })
    );
    // `sym "a"` is the bare name `a`.
    let s = source::read(b"fact p(sym \"a\")");
    assert_eq!(
        s.lines[0].record,
        Record::Fact(Atom {
            pred: "p".into(),
            args: vec![sym("a")]
        })
    );
}

/// A typed constant that doesn't read is located where it can't continue.
/// Outside `fact` lines no typed constant reads (Q24).
#[test]
fn typed_constants_that_do_not_read() {
    let cases = [
        ("fact p(nat 07)", 13),
        ("fact p(nat 18446744073709551616)", 31),
        ("fact p(nat x)", 12),
        ("fact p(addr abc)", 16),
        ("fact p(addr ABC)", 13),
        ("fact p(bytes \"abc\")", 18),
        ("fact p(bytes \"AB\")", 15),
        ("fact p(bytes ab)", 14),
        ("fact p(sym \"a\\n\")", 15),
        ("fact p(sym \"caf\u{e9}\")", 16),
        ("fact p(sym \"a)", 15),
        ("fact p(int 5)", 11),
        ("query p(nat 7)", 12),
        ("rule r p(nat 7) requires q", 13),
    ];
    for (line, c) in cases {
        refused_at(line, c);
    }
    // 2^64 - 1 reads.
    assert!(unread("fact p(nat 18446744073709551615)").is_empty());
}

/// The fixtures' sources: what reads and where the rest stops.
#[test]
fn the_fixture_sources() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/echo-vectors/mirror");
    let read = |p: &str| std::fs::read(dir.join(p)).expect(p);
    let at = |p: &str| -> Vec<String> {
        source::read(&read(p))
            .unread
            .iter()
            .map(|u| u.at.to_string())
            .collect()
    };
    assert_eq!(at("48-source.pact-text"), ["2:6", "7:11"]);
    assert_eq!(at("fixtures/pos-dark2-reader/source.pact"), ["3:14", "5:5"]);
    assert_eq!(at("fixtures/p8-swapped/source.pact"), ["4:6"]);
    assert_eq!(at("77-source.pact-text"), ["2:6"]);
    assert_eq!(at("81-source.pact-text"), ["3:6"]);
    for p in [
        "fixtures/pos-v01/source.pact",
        "fixtures/pos-v04/source.pact",
        "fixtures/pos-v05/source.pact",
        "fixtures/typed-corpus-unheard/source.pact",
        "20-source.pact-text",
        "21-source.pact-text",
        "22-source.pact-text",
        "23-source.pact-text",
        "24-source.pact-text",
        "71-source.pact-text",
        "01-determinism.pact-text",
    ] {
        assert_eq!(at(p), Vec::<String>::new(), "{p}");
    }
    let typed = source::read(&read("fixtures/typed-corpus-unheard/source.pact"));
    assert_eq!(typed.lines.len(), 9);
}
