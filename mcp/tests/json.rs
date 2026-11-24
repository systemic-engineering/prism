//! The wire by hand (docs/spec/mcp.md §3): RFC 8259 and I-JSON edge cases,
//! each refused with its kind at its byte offset, and the printer.

use std::collections::BTreeMap;

use prism_mcp::json::{print, read, Decimal, Kind, Refusal, Value, MAX_BYTES, MAX_DEPTH};

fn refused(bytes: &[u8], kind: Kind, at: usize) {
    assert_eq!(
        read(bytes),
        Err(Refusal { kind, at }),
        "{:?}",
        String::from_utf8_lossy(bytes)
    );
}

fn canon(text: &str) -> String {
    print(&read(text.as_bytes()).unwrap_or_else(|r| panic!("{text}: {r:?}")))
}

fn obj(members: &[(&str, Value)]) -> Value {
    Value::Object(
        members
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    )
}

fn s(text: &str) -> Value {
    Value::Str(text.into())
}

// ------------------------------------------------------------- values

/// RFC 8259 §3: the literals, and any value at the top (§2).
#[test]
fn scalars_read_at_the_top() {
    assert_eq!(read(b"null"), Ok(Value::Null));
    assert_eq!(read(b"true"), Ok(Value::Bool(true)));
    assert_eq!(read(b"false"), Ok(Value::Bool(false)));
    assert_eq!(read(b"0"), Ok(Value::Int(0)));
    assert_eq!(read(b"-12"), Ok(Value::Int(-12)));
    assert_eq!(read(b"\"a\""), Ok(s("a")));
    assert_eq!(read(b" \t\r\n[ ] \r\n"), Ok(Value::Array(vec![])));
    assert_eq!(
        read(br#"{"a":[1,{"b":null}],"c":"d"}"#),
        Ok(obj(&[
            (
                "a",
                Value::Array(vec![Value::Int(1), obj(&[("b", Value::Null)])])
            ),
            ("c", s("d")),
        ]))
    );
}

/// RFC 8259 §2: whitespace is space, tab, LF and CR; nothing else.
#[test]
fn whitespace_is_four_bytes() {
    refused(b"\x0c{}", Kind::Unexpected, 0);
    refused(b"[1,\x0b2]", Kind::Unexpected, 3);
    refused(b"", Kind::End, 0);
    refused(b"   ", Kind::End, 3);
}

/// RFC 8259 §2: nothing but whitespace after the value.
#[test]
fn trailing_garbage_is_refused_where_it_starts() {
    refused(b"{} x", Kind::Trailing, 3);
    refused(b"[1]]", Kind::Trailing, 3);
    refused(b"1 2", Kind::Trailing, 2);
    refused(b"\"a\"\"b\"", Kind::Trailing, 3);
    refused(b"null\x00", Kind::Trailing, 4);
}

/// RFC 8259 §2, §4, §5: structure, literals and the end of input.
#[test]
fn structure_is_refused_at_the_byte() {
    refused(b"[1,]", Kind::Unexpected, 3);
    refused(b"[,1]", Kind::Unexpected, 1);
    refused(br#"{"a" 1}"#, Kind::Unexpected, 5);
    refused(br#"{"a":1,}"#, Kind::Unexpected, 7);
    refused(br#"{a:1}"#, Kind::Unexpected, 1);
    refused(br#"{"a":1 "b":2}"#, Kind::Unexpected, 7);
    refused(b"[1 2]", Kind::Unexpected, 3);
    refused(b"trux", Kind::Unexpected, 3);
    refused(b"NaN", Kind::Unexpected, 0);
    refused(b"+1", Kind::Unexpected, 0);
    refused(b"'a'", Kind::Unexpected, 0);
    refused(b"tru", Kind::End, 3);
    refused(b"[1,", Kind::End, 3);
    refused(b"{\"a\":", Kind::End, 5);
    refused(b"\"abc", Kind::End, 4);
    refused(b"[", Kind::End, 1);
}

/// RFC 8259 §8.1: no byte-order mark, and this reader doesn't ignore one.
#[test]
fn a_byte_order_mark_is_refused() {
    refused(b"\xef\xbb\xbf{}", Kind::Unexpected, 0);
}

// ------------------------------------------------------------ strings

/// RFC 8259 §7: the escapes, and a pair for an astral code point.
#[test]
fn escapes_read() {
    assert_eq!(
        read(br#""\"\\\/\b\f\n\r\t\u0041\u00e9\uD834\uDD1E""#),
        Ok(s("\"\\/\u{8}\u{c}\n\r\tAé\u{1D11E}"))
    );
    assert_eq!(read("\"é€𝄞\"".as_bytes()), Ok(s("é€𝄞")));
    assert_eq!(read(b"\"\x7f\""), Ok(s("\u{7f}")));
}

/// RFC 8259 §7: a control byte must be escaped; an escape is one of nine.
#[test]
fn control_bytes_and_bad_escapes_are_refused() {
    refused(b"[\"a\tb\"]", Kind::Control, 3);
    refused(b"\"\x00\"", Kind::Control, 1);
    refused(b"\"\n\"", Kind::Control, 1);
    refused(br#"["\x"]"#, Kind::Escape, 2);
    refused(br#"["\u12"]"#, Kind::Escape, 2);
    refused(br#"["\u12G4"]"#, Kind::Escape, 2);
    refused(br#"["\U0041"]"#, Kind::Escape, 2);
    refused(br#"["ab\'"]"#, Kind::Escape, 4);
}

/// RFC 8259 §7 and §8.2, RFC 7493 §2.1: a surrogate escape stands only as a
/// high one followed at once by a low one.
#[test]
fn a_lone_surrogate_is_refused_at_its_escape() {
    refused(br#"["\uD800"]"#, Kind::Surrogate, 2);
    refused(br#"["\uDC00"]"#, Kind::Surrogate, 2);
    refused(br#"["ab\uDBFFx"]"#, Kind::Surrogate, 4);
    refused(br#"["\uD800\u0041"]"#, Kind::Surrogate, 2);
    refused(br#"["\uD800\uD800"]"#, Kind::Surrogate, 2);
    refused(br#"["\uDD1E\uD834"]"#, Kind::Surrogate, 2);
    refused(br#"["\uD800\n"]"#, Kind::Surrogate, 2);
    refused(br#"{"\uDFAA":0}"#, Kind::Surrogate, 2);
}

/// RFC 8259 §8.1, RFC 3629: strict UTF-8, refused at the sequence's first
/// byte.
#[test]
fn invalid_utf8_is_refused_where_it_starts() {
    refused(b"[\"\xff\"]", Kind::Utf8, 2);
    refused(b"[\"a\xc3\"]", Kind::Utf8, 3);
    refused(b"[\"\xc0\xaf\"]", Kind::Utf8, 2);
    refused(b"[\"\xe0\x80\xaf\"]", Kind::Utf8, 2);
    refused(b"[\"\xed\xa0\x80\"]", Kind::Utf8, 2);
    refused(b"[\"\xf4\x90\x80\x80\"]", Kind::Utf8, 2);
    refused(b"[\"ab\x80\"]", Kind::Utf8, 4);
    // the earliest offence wins: a truncated sequence before the end
    refused(b"\"\xe2\x82", Kind::Utf8, 1);
    refused(b"[\"\xff\x01\"]", Kind::Utf8, 2);
    // outside a string a non-ASCII byte is just not JSON
    refused(b"[\xc3\xa9]", Kind::Unexpected, 1);
}

/// RFC 7493 §2.1: no noncharacter, raw or escaped.
#[test]
fn noncharacters_are_refused() {
    refused(br#"["\uFFFF"]"#, Kind::Noncharacter, 2);
    refused(br#"["\uFDD0"]"#, Kind::Noncharacter, 2);
    refused(br#"["a\uFDEF"]"#, Kind::Noncharacter, 3);
    refused(br#"["\uDBFF\uDFFF"]"#, Kind::Noncharacter, 2);
    refused(br#"["\uD83F\uDFFE"]"#, Kind::Noncharacter, 2);
    refused("[\"x\u{FFFE}\"]".as_bytes(), Kind::Noncharacter, 3);
    refused("[\"\u{10FFFF}\"]".as_bytes(), Kind::Noncharacter, 2);
    assert_eq!(
        read(br#""\uFDCF\uFDF0\uFFFD""#),
        Ok(s("\u{FDCF}\u{FDF0}\u{FFFD}"))
    );
}

// ------------------------------------------------------------ objects

/// RFC 7493 §2.3: names are unique, compared after unescaping; the second
/// is refused at its opening quote.
#[test]
fn a_duplicate_name_is_refused_at_its_quote() {
    refused(br#"{"a":1,"a":2}"#, Kind::Duplicate, 7);
    refused(br#"{"a":1,"a":1}"#, Kind::Duplicate, 7);
    refused(br#"{"a":1,"\u0061":2}"#, Kind::Duplicate, 7);
    refused(br#"{"a":1, "b":{"c":0,"c":0}}"#, Kind::Duplicate, 19);
    refused(br#"[{"":1,"":2}]"#, Kind::Duplicate, 7);
    assert!(read(br#"{"a":{"a":1},"b":{"a":1}}"#).is_ok());
    assert!(read(br#"{"a":1,"A":2,"\u00e9":3,"e\u0301":4}"#).is_ok());
}

// ------------------------------------------------------------- limits

/// §3.3: 64 levels read; the 65th opening bracket is refused.
#[test]
fn depth_over_the_limit_is_refused_at_its_bracket() {
    assert_eq!(MAX_DEPTH, 64);
    let ok = format!("{}{}", "[".repeat(64), "]".repeat(64));
    assert!(read(ok.as_bytes()).is_ok());
    let deep = format!("{}{}", "[".repeat(65), "]".repeat(65));
    refused(deep.as_bytes(), Kind::Depth, 64);
    let mixed = format!("{}{}", "{\"a\":[".repeat(33), "]}".repeat(33));
    // `{` of the 33rd `{"a":[` is the 65th bracket
    refused(mixed.as_bytes(), Kind::Depth, 6 * 32);
    refused(&[b'['; 100_000], Kind::Depth, 64);
}

/// §3.3: 8 MiB read; one byte more is refused before anything is read.
#[test]
fn size_over_the_limit_is_refused() {
    assert_eq!(MAX_BYTES, 8 * 1024 * 1024);
    let mut text = vec![b'a'; MAX_BYTES];
    text[0] = b'"';
    text[MAX_BYTES - 1] = b'"';
    assert!(matches!(read(&text), Ok(Value::Str(x)) if x.len() == MAX_BYTES - 2));
    text.push(b' ');
    refused(&text, Kind::Size, MAX_BYTES);
}

// ------------------------------------------------------------ numbers

/// RFC 8259 §6: no leading zero, refused at the digit after it.
#[test]
fn leading_zeros_are_refused() {
    refused(b"[01]", Kind::LeadingZero, 2);
    refused(b"[-01]", Kind::LeadingZero, 3);
    refused(b"00", Kind::LeadingZero, 1);
    refused(b"012.5", Kind::LeadingZero, 1);
    assert_eq!(
        read(b"[0,10,0.5]").map(|v| print(&v)),
        Ok("[0,10,0.5]".into())
    );
}

/// §3.2: the integer domain has one zero, spelled `0`.
#[test]
fn negative_zero_is_refused() {
    refused(b"-0", Kind::NegativeZero, 0);
    refused(b"[1,-0]", Kind::NegativeZero, 3);
    refused(br#"{"id":-0}"#, Kind::NegativeZero, 6);
    // a decimal is an opaque lexeme
    assert!(matches!(read(b"-0.0"), Ok(Value::Decimal(_))));
}

/// RFC 8259 §6 grammar: digits after `.`, `e` and `-`.
#[test]
fn malformed_numbers_are_refused() {
    refused(b"1.", Kind::End, 2);
    refused(b"[1.]", Kind::Unexpected, 3);
    refused(b"[.5]", Kind::Unexpected, 1);
    refused(b"[1e]", Kind::Unexpected, 3);
    refused(b"[1e+]", Kind::Unexpected, 4);
    refused(b"[-]", Kind::Unexpected, 2);
    refused(b"-", Kind::End, 1);
    refused(b"[1.5.2]", Kind::Unexpected, 4);
    refused(b"[0x10]", Kind::Unexpected, 2);
}

/// §3.2: an integer is an `i64` or refused; never rounded.
#[test]
fn a_huge_integer_is_refused_at_its_first_byte() {
    assert_eq!(read(b"9223372036854775807"), Ok(Value::Int(i64::MAX)));
    assert_eq!(read(b"-9223372036854775808"), Ok(Value::Int(i64::MIN)));
    refused(b"9223372036854775808", Kind::IntegerRange, 0);
    refused(b"-9223372036854775809", Kind::IntegerRange, 0);
    refused(b"[1,123456789012345678901234567890]", Kind::IntegerRange, 3);
}

/// §3.2: a fraction or an exponent is kept as its lexeme, and refused
/// where an integer is expected, at the number's first byte.
#[test]
fn an_exponent_where_an_integer_is_expected_is_refused() {
    let v = read(b"[1e2, 1.0, -5E-0, 7]").expect("reads");
    let Value::Array(items) = &v else {
        panic!("{v:?}")
    };
    assert_eq!(
        items[0],
        Value::Decimal(Decimal {
            text: "1e2".into(),
            at: 1
        })
    );
    let integers: Vec<_> = items.iter().map(Value::integer).collect();
    assert_eq!(
        integers,
        vec![
            Some(Err(Refusal {
                kind: Kind::NotInteger,
                at: 1
            })),
            Some(Err(Refusal {
                kind: Kind::NotInteger,
                at: 6
            })),
            Some(Err(Refusal {
                kind: Kind::NotInteger,
                at: 11
            })),
            Some(Ok(7)),
        ]
    );
    assert_eq!(s("7").integer(), None);
    assert_eq!(Value::Null.integer(), None);
    assert_eq!(print(&v), "[1e2,1.0,-5E-0,7]");
}

// ------------------------------------------------------------ printer

/// §3.5: members in byte order of their names, no whitespace.
#[test]
fn the_printer_orders_members_by_name_bytes() {
    assert_eq!(
        canon(r#" { "b" : 1 , "a" : [ true , null , "x" ] , "c" : { } } "#),
        r#"{"a":[true,null,"x"],"b":1,"c":{}}"#
    );
    assert_eq!(
        canon(r#"{"\u00e9":1,"z":2,"A":3,"":4}"#),
        "{\"\":4,\"A\":3,\"z\":2,\"é\":1}"
    );
}

/// §3.5: the escapes the printer writes; LF never raw.
#[test]
fn the_printer_escapes_what_it_must() {
    let v = s("\"\\\u{1}\u{8}\u{c}\n\r\t\u{1f}\u{7f}é/\u{2028}");
    let p = print(&v);
    assert_eq!(
        p,
        "\"\\\"\\\\\\u0001\\b\\f\\n\\r\\t\\u001f\u{7f}é/\u{2028}\""
    );
    assert!(!p.contains('\n'));
    assert_eq!(read(p.as_bytes()), Ok(v));
    assert_eq!(canon(r#""\/\u001F""#), r#""/\u001f""#);
}

/// §3.5: identical values print identical bytes, whatever order they were
/// built in; printing is idempotent through the reader.
#[test]
fn the_printer_is_deterministic_and_canonical() {
    let mut a = BTreeMap::new();
    a.insert("y".to_string(), Value::Int(-1));
    a.insert("x".to_string(), Value::Bool(false));
    let mut b = BTreeMap::new();
    b.insert("x".to_string(), Value::Bool(false));
    b.insert("y".to_string(), Value::Int(-1));
    assert_eq!(print(&Value::Object(a.clone())), print(&Value::Object(b)));
    assert_eq!(print(&Value::Object(a.clone())), r#"{"x":false,"y":-1}"#);
    let v = Value::Array(vec![
        Value::Object(a),
        Value::Int(i64::MIN),
        Value::Decimal(Decimal {
            text: "6.02e23".into(),
            at: 0,
        }),
        Value::Null,
        s(""),
    ]);
    let once = print(&v);
    assert_eq!(once, print(&v));
    assert_eq!(
        once,
        r#"[{"x":false,"y":-1},-9223372036854775808,6.02e23,null,""]"#
    );
    assert_eq!(canon(&once), once);
}

/// The refusal words `data.refusal` carries (§3.4).
#[test]
fn every_kind_has_its_word() {
    let words = [
        (Kind::Size, "size"),
        (Kind::Depth, "depth"),
        (Kind::End, "end"),
        (Kind::Unexpected, "unexpected"),
        (Kind::Trailing, "trailing"),
        (Kind::Control, "control"),
        (Kind::Escape, "escape"),
        (Kind::Surrogate, "surrogate"),
        (Kind::Utf8, "utf8"),
        (Kind::Noncharacter, "noncharacter"),
        (Kind::Duplicate, "duplicate"),
        (Kind::LeadingZero, "leading-zero"),
        (Kind::NegativeZero, "negative-zero"),
        (Kind::IntegerRange, "integer-range"),
        (Kind::NotInteger, "not-integer"),
    ];
    for (kind, word) in words {
        assert_eq!(kind.word(), word);
    }
}
