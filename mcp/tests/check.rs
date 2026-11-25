//! The check tool (docs/spec/mcp.md §5.3): prism-laws's `agreement::check`
//! over the wire, on `prismqueer/tests/agreement.rs`'s known values. The
//! addresses come from the library itself: the tool reports what it says.

use prism_laws::agreement::{Consistent, Relations, SelectionDraft};
use prism_laws::room::{Modulus, QModOne, ZMod, Z};
use prism_mcp::json::Value;

use crate::common::{call, code, get, is_error, ready, str_of, texts};

/// The adjudicator `by` names: a label beside the address, never a key.
pub const BY: &str = "abababababababababababababababababababababababababababababababab";

fn run(args: &str) -> Value {
    ready(&[call(1, "check", args)]).remove(0)
}

/// The one text of a result.
fn text(a: &Value) -> String {
    let t = texts(a);
    assert_eq!(t.len(), 1, "{t:?}");
    t[0].clone()
}

fn z_args(relations: &str, selection: &str) -> String {
    format!(r#"{{"ring":"Z","relations":{relations},"selection":{selection},"by":"{BY}"}}"#)
}

const TRIANGLE: &str = "[[0,1,3],[1,2,4],[0,2,7]]";

fn z_oid(readings: &[(u64, i64)]) -> String {
    SelectionDraft::new(readings.iter().map(|&(v, x)| (v, Z::from(x))))
        .oid()
        .as_str()
        .to_string()
}

fn head(verdict: &str, ring: &str, oid: &str) -> String {
    format!("{verdict}\nring {ring}\nselection {oid} by {BY}\n")
}

// --- In, Out, NeedsRuling ------------------------------------------------------

#[test]
fn a_consistent_triangle_is_in_with_its_certificate() {
    let a = run(&z_args(TRIANGLE, "[[0,10],[1,13],[2,17]]"));
    assert!(!is_error(&a));
    assert_eq!(
        text(&a),
        head("in", "Z", &z_oid(&[(0, 10), (1, 13), (2, 17)]))
            + "edge 0-1#0 required 3 found 3\n\
               edge 0-2#0 required 7 found 7\n\
               edge 1-2#0 required 4 found 4\n\
               coordinate 0 10\n"
    );
}

#[test]
fn off_by_one_is_out_with_exactly_its_edges_and_is_a_verdict() {
    let a = run(&z_args(TRIANGLE, "[[0,10],[1,14],[2,17]]"));
    assert!(!is_error(&a), "Out is an answer about the subject");
    assert_eq!(
        text(&a),
        head("out", "Z", &z_oid(&[(0, 10), (1, 14), (2, 17)]))
            + "violated 0-1#0 required 3 found 4\n\
               violated 1-2#0 required 4 found 3\n"
    );
}

/// The cycle lines as the library's cycles give them.
fn cycles<V: prism_laws::agreement::Reading>(
    relations: &Relations<V>,
    word: fn(&V) -> String,
) -> String {
    let Consistent::Empty(cycles) = relations.consistent().expect("solvable") else {
        panic!("curved");
    };
    cycles
        .iter()
        .map(|c| {
            let vertices: Vec<String> = c.vertices().iter().map(u64::to_string).collect();
            let edges: Vec<String> = c
                .edges()
                .iter()
                .map(|e| format!("{}-{}#{}", e.ends().0, e.ends().1, e.nth()))
                .collect();
            format!(
                "cycle twist {} vertices {} edges {}\n",
                word(c.pairing()),
                vertices.join(" "),
                edges.join(" ")
            )
        })
        .collect()
}

fn z_word(z: &Z) -> String {
    z.value().to_string()
}

#[test]
fn curved_relations_need_a_ruling_and_it_is_a_verdict() {
    let relations = Relations::new(
        &(),
        [],
        [(0, 1, Z::from(3)), (1, 2, Z::from(4)), (0, 2, Z::from(8))],
    )
    .unwrap();
    let a = run(&z_args("[[0,1,3],[1,2,4],[0,2,8]]", "[[0,0],[1,3],[2,7]]"));
    assert!(!is_error(&a));
    let t = text(&a);
    assert_eq!(
        t,
        head("needs-ruling", "Z", &z_oid(&[(0, 0), (1, 3), (2, 7)])) + &cycles(&relations, z_word)
    );
    assert_eq!(t.lines().filter(|l| l.starts_with("cycle ")).count(), 1);
    let twist = t.lines().last().unwrap();
    assert!(
        twist.starts_with("cycle twist 1 ") || twist.starts_with("cycle twist -1 "),
        "{twist}"
    );
}

// --- The ring decides: the tritone pair ------------------------------------------

#[test]
fn the_tritone_pair_needs_a_ruling_over_z() {
    let relations = Relations::new(&(), [], [(0, 1, Z::from(6)), (1, 0, Z::from(6))]).unwrap();
    let a = run(&z_args("[[0,1,6],[1,0,6]]", "[[0,0],[1,6]]"));
    assert!(!is_error(&a));
    let t = text(&a);
    assert_eq!(
        t,
        head("needs-ruling", "Z", &z_oid(&[(0, 0), (1, 6)])) + &cycles(&relations, z_word)
    );
    assert!(t.contains("cycle twist 12 "), "the octave: {t}");
    assert!(t.ends_with("edges 0-1#0 0-1#1\n"), "{t}");
}

#[test]
fn the_tritone_pair_has_readings_mod_12() {
    let twelve = Modulus::new(12).unwrap();
    let oid = |a: i64, b: i64| {
        SelectionDraft::new([(0, ZMod::new(a, &twelve)), (1, ZMod::new(b, &twelve))])
            .oid()
            .as_str()
            .to_string()
    };
    let args = |selection: &str| {
        format!(
            r#"{{"ring":"ZMod","modulus":12,"relations":[[0,1,6],[1,0,6]],"selection":{selection},"by":"{BY}"}}"#
        )
    };
    let a = run(&args("[[0,3],[1,9]]"));
    assert!(!is_error(&a));
    assert_eq!(
        text(&a),
        head("in", "ZMod 12", &oid(3, 9))
            + "edge 0-1#0 required 6 found 6\n\
               edge 0-1#1 required 6 found 6\n\
               coordinate 0 3\n"
    );
    // A residue is read mod 12: 21 is 9.
    let b = run(&args("[[0,3],[1,21]]"));
    assert_eq!(text(&b), text(&a));
    let c = run(&args("[[0,0],[1,7]]"));
    assert!(!is_error(&c));
    assert_eq!(
        text(&c),
        head("out", "ZMod 12", &oid(0, 7))
            + "violated 0-1#0 required 6 found 7\n\
               violated 0-1#1 required 6 found 7\n"
    );
}

#[test]
fn readings_over_q_mod_1_travel_as_p_over_q() {
    let q = |n: i64, d: i64| QModOne::new(n, d).unwrap();
    let args = |relations: &str, selection: &str| {
        format!(
            r#"{{"ring":"QModOne","relations":{relations},"selection":{selection},"by":"{BY}"}}"#
        )
    };
    let a = run(&args(r#"[[0,1,"1/2"]]"#, r#"[[0,"1/4"],[1,"3/4"]]"#));
    assert!(!is_error(&a));
    let oid = SelectionDraft::new([(0, q(1, 4)), (1, q(3, 4))]).oid();
    assert_eq!(
        text(&a),
        head("in", "QModOne", oid.as_str())
            + "edge 0-1#0 required 1/2 found 1/2\n\
               coordinate 0 1/4\n"
    );
    // Curved over ℚ/ℤ: 1/2 + 1/3 around the loop.
    let curved = Relations::new(&(), [], [(0, 1, q(1, 2)), (1, 0, q(1, 3))]).unwrap();
    let b = run(&args(
        r#"[[0,1,"1/2"],[1,0,"1/3"]]"#,
        r#"[[0,"0/1"],[1,"1/2"]]"#,
    ));
    assert!(!is_error(&b));
    let oid = SelectionDraft::new([(0, q(0, 1)), (1, q(1, 2))]).oid();
    let word = |v: &QModOne| {
        if v.denominator().to_string() == "1" {
            v.numerator().to_string()
        } else {
            format!("{}/{}", v.numerator(), v.denominator())
        }
    };
    assert_eq!(
        text(&b),
        head("needs-ruling", "QModOne", oid.as_str()) + &cycles(&curved, word)
    );
}

// --- The ruler: no verdict -------------------------------------------------------

#[test]
fn a_selection_that_does_not_read_the_items_is_a_broken_ruler() {
    let missing = run(&z_args(TRIANGLE, "[[0,0],[1,3]]"));
    assert!(is_error(&missing));
    assert_eq!(text(&missing), "error selection missing 2\n");
    let unknown = run(&z_args(TRIANGLE, "[[0,0],[1,3],[2,7],[5,0]]"));
    assert!(is_error(&unknown));
    assert_eq!(text(&unknown), "error selection unknown 5\n");
    // An item declared beyond the relations must be read too.
    let extra = run(&format!(
        r#"{{"ring":"Z","items":[9],"relations":{TRIANGLE},"selection":[[0,0],[1,3],[2,7]],"by":"{BY}"}}"#
    ));
    assert!(is_error(&extra));
    assert_eq!(text(&extra), "error selection missing 9\n");
}

#[test]
fn a_value_outside_the_ring_is_a_broken_ruler() {
    let cases = [
        (
            z_args(r#"[[0,1,"1/2"]]"#, "[[0,0],[1,0]]"),
            "error ring relations.0 integer\n",
        ),
        (
            z_args("[[0,1,1]]", r#"[[0,0],[1,"1/2"]]"#),
            "error ring selection.1 integer\n",
        ),
        (
            format!(
                r#"{{"ring":"QModOne","relations":[[0,1,1]],"selection":[[0,"0/1"],[1,"0/1"]],"by":"{BY}"}}"#
            ),
            "error ring relations.0 p/q\n",
        ),
        (
            format!(r#"{{"ring":"ZMod","modulus":1,"relations":[],"selection":[],"by":"{BY}"}}"#),
            "error ring modulus 1\n",
        ),
    ];
    for (args, want) in cases {
        let a = run(&args);
        assert!(is_error(&a), "{args}");
        assert_eq!(text(&a), want, "{args}");
    }
}

// --- The wire: malformed is -32602 -----------------------------------------------

fn error_data(a: &Value) -> String {
    prism_mcp::json::print(get(a, &["error", "data"]))
}

#[test]
fn malformed_arguments_are_invalid_params() {
    let cases = [
        (
            r#"{"ring":"Z","relations":[],"selection":[]}"#.to_string(),
            "by",
        ),
        (
            format!(r#"{{"ring":"R","relations":[],"selection":[],"by":"{BY}"}}"#),
            "ring",
        ),
        (
            format!(r#"{{"ring":"Z","relations":{{}},"selection":[],"by":"{BY}"}}"#),
            "relations",
        ),
        (
            format!(r#"{{"ring":"Z","relations":[[0,1]],"selection":[],"by":"{BY}"}}"#),
            "relations.0",
        ),
        (
            format!(r#"{{"ring":"Z","relations":[[-1,1,0]],"selection":[],"by":"{BY}"}}"#),
            "relations.0",
        ),
        (
            format!(r#"{{"ring":"Z","relations":[[0,1,1.5]],"selection":[],"by":"{BY}"}}"#),
            "relations.0",
        ),
        (
            format!(r#"{{"ring":"Z","relations":[],"selection":[[0]],"by":"{BY}"}}"#),
            "selection.0",
        ),
        (
            format!(r#"{{"ring":"Z","relations":[],"selection":[],"items":[-3],"by":"{BY}"}}"#),
            "items.0",
        ),
        (
            r#"{"ring":"Z","relations":[],"selection":[],"by":"ABAB"}"#.to_string(),
            "by",
        ),
        (
            format!(r#"{{"ring":"Z","modulus":12,"relations":[],"selection":[],"by":"{BY}"}}"#),
            "modulus",
        ),
        (
            format!(r#"{{"ring":"ZMod","relations":[],"selection":[],"by":"{BY}"}}"#),
            "modulus",
        ),
        (
            format!(r#"{{"ring":"QModOne","relations":[[0,1,"1/0"]],"selection":[],"by":"{BY}"}}"#),
            "relations.0",
        ),
        (
            format!(r#"{{"ring":"QModOne","relations":[[0,1,"0.5"]],"selection":[],"by":"{BY}"}}"#),
            "relations.0",
        ),
        (
            format!(r#"{{"ring":"Z","relations":[],"selection":[],"by":"{BY}","signature":"x"}}"#),
            "signature",
        ),
    ];
    for (args, argument) in cases {
        let a = run(&args);
        assert_eq!(code(&a), -32602, "{args}");
        assert!(
            error_data(&a).starts_with(&format!(r#"{{"argument":"{argument}","#)),
            "{args}: {}",
            error_data(&a)
        );
    }
}

/// `by` is a label (algedonic::Signer): the tool says so, and that prism
/// holds no keys and never signs.
#[test]
fn the_description_says_by_is_a_label() {
    let out = ready(&[r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#.into()]);
    let Value::Array(tools) = get(&out[0], &["result", "tools"]) else {
        panic!("tools");
    };
    let check = tools
        .iter()
        .find(|t| str_of(get(t, &["name"])) == "check")
        .expect("check is listed");
    let d = str_of(get(check, &["description"]));
    assert!(d.contains("label"), "{d}");
    assert!(d.contains("never signs"), "{d}");
    assert_eq!(
        prism_mcp::json::print(get(check, &["inputSchema", "required"])),
        r#"["ring","relations","selection","by"]"#
    );
}
