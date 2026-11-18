//! P1, what reads. Reflection seam A1.4 (the format: headers, `fact` lines,
//! comments), Seam's pact-eval audit §1.2 (the line shapes), fixpoint
//! §5.0.1 item 6 and §16.10 item 1 (typed constants).

use echo::{read, read_reflection, Atom, Header, Value};

fn sym(s: &str) -> Value {
    Value::Sym(s.into())
}

fn atom(pred: &str, args: Vec<Value>) -> Atom {
    Atom {
        pred: pred.into(),
        args,
    }
}

fn atoms(src: &[u8]) -> Vec<Atom> {
    read(src)
        .expect("reads")
        .facts
        .into_iter()
        .map(|f| f.atom)
        .collect()
}

/// mirror.facts' header shape (fixpoint §16.10 item 1): `# key value` lines
/// at the top.
#[test]
fn header_lines_at_the_top_are_headers() {
    let f = read(b"# format-grammar 145e\n# root 97c6\n# grammar 9092\nfact p\n").expect("reads");
    let h = |k: &str, v: &str, line| Header {
        key: k.into(),
        value: v.into(),
        line,
    };
    assert_eq!(
        f.headers,
        vec![
            h("format-grammar", "145e", 1),
            h("root", "97c6", 2),
            h("grammar", "9092", 3)
        ]
    );
    assert_eq!(f.facts.len(), 1);
    assert_eq!(f.facts[0].line, 4);
}

/// Headers vs comments is an open question (docs/spec/echo.md §7 Q3); the
/// reading here: only the leading block of `#` lines holds headers, and only
/// a line shaped `# <key> <value>` is one.
#[test]
fn other_hash_lines_are_comments() {
    let f = read(b"# a header\n# lonely\n#\n#x y\n#  two spaces\nfact p\n# root 97c6\n")
        .expect("reads");
    assert_eq!(
        f.headers,
        vec![Header {
            key: "a".into(),
            value: "header".into(),
            line: 1
        }]
    );
    assert_eq!(f.facts.len(), 1);
    // a blank line ends the leading block too
    assert!(read(b"\n# root 97c6\n").expect("reads").headers.is_empty());
}

/// pact-eval §1.2: `blank = { " " }`, and the last LF may be missing.
#[test]
fn blank_lines_and_a_missing_final_lf() {
    assert_eq!(
        atoms(b"\n   \nfact p\n\nfact q"),
        vec![atom("p", vec![]), atom("q", vec![])]
    );
    assert_eq!(atoms(b""), vec![]);
    assert_eq!(atoms(b"\n"), vec![]);
}

/// pact-eval §1.2: `record = key , sp , body , { " " }`, `sp = " " , { " " }`,
/// arguments separated by `"," , sp0`.
#[test]
fn spacing_the_grammar_allows() {
    let want = vec![atom("p", vec![sym("a"), sym("b")])];
    assert_eq!(atoms(b"fact p(a, b)\n"), want);
    assert_eq!(atoms(b"fact   p(a,b)   \n"), want);
    assert_eq!(atoms(b"fact p(a,    b)\n"), want);
}

/// pact-eval §1.2: a zero-ary atom has no parentheses; names are
/// `lower { lower | digit | "_" }`.
#[test]
fn names_and_zero_ary_atoms() {
    assert_eq!(atoms(b"fact p\n"), vec![atom("p", vec![])]);
    assert_eq!(
        atoms(b"fact term_lit2(seq_0, x9)\n"),
        vec![atom("term_lit2", vec![sym("seq_0"), sym("x9")])]
    );
}

/// Every typed form of §5.0.1 item 6 and §16.10 item 1, at its edges.
#[test]
fn typed_constants() {
    let addr = "0976ca1179bfa9425af4522e8796cd7d1bef52992c68ce6a5a1e31c123c91bfa";
    let src = format!(
        "fact t(sym \"a\\\"b\\\\c\", sym \"\", sym \"λ # ( ) , \", nat 0, nat 18446744073709551615, addr {addr}, bytes \"\", bytes \"00ff2f\")\n"
    );
    let mut a = [0u8; 32];
    for (i, b) in a.iter_mut().enumerate() {
        *b = u8::from_str_radix(&addr[2 * i..2 * i + 2], 16).unwrap();
    }
    assert_eq!(
        atoms(src.as_bytes()),
        vec![atom(
            "t",
            vec![
                sym("a\"b\\c"),
                sym(""),
                sym("λ # ( ) , "),
                Value::Nat(0),
                Value::Nat(u64::MAX),
                Value::Addr(a),
                Value::Bytes(vec![]),
                Value::Bytes(vec![0x00, 0xff, 0x2f]),
            ]
        )]
    );
}

/// The canonical text prints back, and reads again to the same atom.
#[test]
fn the_canonical_text_round_trips() {
    let src = b"fact t(seq, sym \"a\\\"b\\\\\", nat 7, bytes \"\", bytes \"2f\", addr 0976ca1179bfa9425af4522e8796cd7d1bef52992c68ce6a5a1e31c123c91bfa)\n";
    let a = atoms(src);
    let printed = a[0].to_string();
    assert_eq!(
        printed,
        "t(sym \"seq\", sym \"a\\\"b\\\\\", nat 7, bytes \"\", bytes \"2f\", addr 0976ca1179bfa9425af4522e8796cd7d1bef52992c68ce6a5a1e31c123c91bfa)"
    );
    assert_eq!(atoms(format!("fact {printed}\n").as_bytes()), a);
    assert_eq!(atom("p", vec![]).to_string(), "p");
}

/// Duplicates are kept in input order with their lines (the set is the
/// caller's: fixpoint §5.0.1 item 2).
#[test]
fn duplicate_facts_are_kept_with_their_lines() {
    let f = read(b"fact p(a)\nfact p(a)\n").expect("reads");
    assert_eq!(f.facts.len(), 2);
    assert_eq!(f.facts[0].atom, f.facts[1].atom);
    assert_eq!((f.facts[0].line, f.facts[1].line), (1, 2));
}

/// The reflection is read below the seam only, in the file's coordinates.
#[test]
fn a_reflection_is_read_below_the_seam() {
    // above: a rule line, which a facts reader refuses, and it is never read
    let f = read_reflection(b"rule r p requires q\n---\n# root 97c6\nfact p(a)\n")
        .expect("reads")
        .expect("reflected");
    assert_eq!(f.headers[0].line, 3);
    assert_eq!(f.facts[0].line, 4);
    assert_eq!(f.facts[0].atom, atom("p", vec![sym("a")]));
    assert_eq!(read_reflection(b"fact p\n"), Ok(None));
    assert_eq!(
        read_reflection(b"x\n---\n")
            .expect("reads")
            .expect("reflected")
            .facts,
        vec![]
    );
}
