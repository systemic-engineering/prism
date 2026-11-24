//! JSONTestSuite as data (docs/spec/mcp.md §3.6): nst/JSONTestSuite at
//! `1ef36fa01286573e846ac449e8683f8833c5b26a` (2024-11-22), `test_parsing/`
//! byte for byte in docs/spec/mcp-vectors/JSONTestSuite.
//!
//! - every `n_` case is refused;
//! - every `y_` case is accepted, except [`Y_REFUSED`]: what I-JSON (RFC
//!   7493 §2.1 noncharacters, §2.3 duplicate names) or the integer rule
//!   (§3.2, `-0`) refuses, each with its kind;
//! - every `i_` case's outcome is [`I_OUTCOMES`];
//! - every accepted case prints canonically: read(print(v)) prints the same.

use std::path::PathBuf;

use prism_mcp::json::{print, read, Kind};

const Y_REFUSED: [(&str, Kind); 12] = [
    ("y_number_minus_zero.json", Kind::NegativeZero),
    ("y_number_negative_zero.json", Kind::NegativeZero),
    ("y_object_duplicated_key.json", Kind::Duplicate),
    ("y_object_duplicated_key_and_value.json", Kind::Duplicate),
    ("y_string_escaped_noncharacter.json", Kind::Noncharacter),
    ("y_string_last_surrogates_1_and_2.json", Kind::Noncharacter),
    (
        "y_string_nonCharacterInUTF-8_U+10FFFF.json",
        Kind::Noncharacter,
    ),
    (
        "y_string_nonCharacterInUTF-8_U+FFFF.json",
        Kind::Noncharacter,
    ),
    ("y_string_unicode_U+10FFFE_nonchar.json", Kind::Noncharacter),
    ("y_string_unicode_U+1FFFE_nonchar.json", Kind::Noncharacter),
    ("y_string_unicode_U+FDD0_nonchar.json", Kind::Noncharacter),
    ("y_string_unicode_U+FFFE_nonchar.json", Kind::Noncharacter),
];

/// `None`: accepted (a decimal lexeme, never converted).
const I_OUTCOMES: [(&str, Option<Kind>); 35] = [
    ("i_number_double_huge_neg_exp.json", None),
    ("i_number_huge_exp.json", None),
    ("i_number_neg_int_huge_exp.json", None),
    ("i_number_pos_double_huge_exp.json", None),
    ("i_number_real_neg_overflow.json", None),
    ("i_number_real_pos_overflow.json", None),
    ("i_number_real_underflow.json", None),
    ("i_number_too_big_neg_int.json", Some(Kind::IntegerRange)),
    ("i_number_too_big_pos_int.json", Some(Kind::IntegerRange)),
    (
        "i_number_very_big_negative_int.json",
        Some(Kind::IntegerRange),
    ),
    (
        "i_object_key_lone_2nd_surrogate.json",
        Some(Kind::Surrogate),
    ),
    (
        "i_string_1st_surrogate_but_2nd_missing.json",
        Some(Kind::Surrogate),
    ),
    (
        "i_string_1st_valid_surrogate_2nd_invalid.json",
        Some(Kind::Surrogate),
    ),
    (
        "i_string_incomplete_surrogate_and_escape_valid.json",
        Some(Kind::Surrogate),
    ),
    (
        "i_string_incomplete_surrogate_pair.json",
        Some(Kind::Surrogate),
    ),
    (
        "i_string_incomplete_surrogates_escape_valid.json",
        Some(Kind::Surrogate),
    ),
    (
        "i_string_invalid_lonely_surrogate.json",
        Some(Kind::Surrogate),
    ),
    ("i_string_invalid_surrogate.json", Some(Kind::Surrogate)),
    ("i_string_invalid_utf-8.json", Some(Kind::Utf8)),
    (
        "i_string_inverted_surrogates_U+1D11E.json",
        Some(Kind::Surrogate),
    ),
    ("i_string_iso_latin_1.json", Some(Kind::Utf8)),
    ("i_string_lone_second_surrogate.json", Some(Kind::Surrogate)),
    (
        "i_string_lone_utf8_continuation_byte.json",
        Some(Kind::Utf8),
    ),
    ("i_string_not_in_unicode_range.json", Some(Kind::Utf8)),
    ("i_string_overlong_sequence_2_bytes.json", Some(Kind::Utf8)),
    ("i_string_overlong_sequence_6_bytes.json", Some(Kind::Utf8)),
    (
        "i_string_overlong_sequence_6_bytes_null.json",
        Some(Kind::Utf8),
    ),
    ("i_string_truncated-utf-8.json", Some(Kind::Utf8)),
    ("i_string_UTF-16LE_with_BOM.json", Some(Kind::Unexpected)),
    ("i_string_UTF-8_invalid_sequence.json", Some(Kind::Utf8)),
    ("i_string_utf16BE_no_BOM.json", Some(Kind::Unexpected)),
    ("i_string_utf16LE_no_BOM.json", Some(Kind::Unexpected)),
    ("i_string_UTF8_surrogate_U+D800.json", Some(Kind::Utf8)),
    ("i_structure_500_nested_arrays.json", Some(Kind::Depth)),
    (
        "i_structure_UTF-8_BOM_empty_object.json",
        Some(Kind::Unexpected),
    ),
];

fn cases() -> Vec<(String, Vec<u8>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/mcp-vectors/JSONTestSuite/test_parsing");
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .expect("the suite is there")
        .map(|e| {
            let p = e.expect("entry").path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read(&p).expect("case"))
        })
        .collect();
    out.sort();
    out
}

fn canonical(name: &str, bytes: &[u8]) {
    let v = read(bytes).unwrap_or_else(|r| panic!("{name}: {r:?}"));
    let once = print(&v);
    let again = read(once.as_bytes()).unwrap_or_else(|r| panic!("{name} printed: {r:?}"));
    assert_eq!(print(&again), once, "{name}");
    assert!(!once.contains('\n'), "{name}");
}

#[test]
fn the_suite_is_all_there() {
    let all = cases();
    let count = |p: &str| all.iter().filter(|(n, _)| n.starts_with(p)).count();
    assert_eq!((count("y_"), count("n_"), count("i_")), (95, 188, 35));
    assert_eq!(all.len(), 318);
}

#[test]
fn every_n_case_is_refused() {
    let accepted: Vec<String> = cases()
        .into_iter()
        .filter(|(n, b)| n.starts_with("n_") && read(b).is_ok())
        .map(|(n, _)| n)
        .collect();
    assert!(accepted.is_empty(), "n_ accepted: {accepted:#?}");
}

#[test]
fn every_y_case_is_accepted_but_what_i_json_refuses() {
    for (name, bytes) in cases().iter().filter(|(n, _)| n.starts_with("y_")) {
        match Y_REFUSED.iter().find(|(n, _)| n == name) {
            Some((_, kind)) => {
                assert_eq!(read(bytes).map_err(|r| r.kind), Err(*kind), "{name}")
            }
            None => canonical(name, bytes),
        }
    }
}

#[test]
fn every_i_case_has_its_pinned_outcome() {
    let all = cases();
    let is: Vec<&(String, Vec<u8>)> = all.iter().filter(|(n, _)| n.starts_with("i_")).collect();
    assert_eq!(is.len(), I_OUTCOMES.len());
    for (name, bytes) in is {
        let (_, want) = I_OUTCOMES
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} unpinned"));
        match want {
            None => canonical(name, bytes),
            Some(kind) => assert_eq!(read(bytes).map_err(|r| r.kind), Err(*kind), "{name}"),
        }
    }
}

/// The refusal offsets stay inside the message (or at its end).
#[test]
fn every_refusal_is_located_inside_the_message() {
    for (name, bytes) in cases() {
        if let Err(r) = read(&bytes) {
            assert!(r.at <= bytes.len(), "{name}: {r:?}");
        }
    }
}
