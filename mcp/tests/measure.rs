//! The measure tool (docs/spec/mcp.md §5.4): prismqueer's `measured::measure`
//! over the wire, on `prismqueer/tests/measured.rs`'s known values. The
//! window is a real ruler's address, and the addresses expected are the
//! library's own `measure` under that ruler: the tool reports what it says.
//! It measures and never seals.

use prism_mcp::json::Value;
use prismqueer::algedonic::{Clock, Draft, Question, Region, Ruler, Wald};
use prismqueer::hypothesis::Frame;
use prismqueer::measured::{measure, Estimator, Method, Support};
use prismqueer::oid::Oid;
use prismqueer::room::{Q, Z};

use crate::common::{call, code, get, is_error, ready, str_of, texts};

/// The frame's reader.
const READER: &str = "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd";

fn window() -> Ruler {
    let r = |n: i64, d: i64| Q::new(n, d).expect("non-zero").value().clone();
    Draft::new(
        Region::above(r(0, 1)),
        Question::Max,
        Wald::new(r(1, 20), r(1, 20)).expect("valid rates"),
        Clock::agreed(10, 1, 100),
    )
    .seal(Oid::new("window-keeper"))
}

fn run(args: &str) -> Value {
    ready(&[call(1, "measure", args)]).remove(0)
}

fn text(a: &Value) -> String {
    let t = texts(a);
    assert_eq!(t.len(), 1, "{t:?}");
    t[0].clone()
}

fn args(samples: &str, streams: usize, estimator: &str, support: &str) -> String {
    format!(
        r#"{{"samples":{samples},"streams":{streams},"estimator":"{estimator}","support":{support},"window":"{}","reader":"{READER}"}}"#,
        window().oid().as_str()
    )
}

fn counts(m: &[&[i64]]) -> Vec<Vec<Z>> {
    m.iter()
        .map(|r| r.iter().map(|&n| Z::from(n)).collect())
        .collect()
}

/// The lines before the estimate, from the library's `Measured`.
fn header(m: &[&[i64]], d: usize, method: Method, words: &str) -> String {
    let frame = Frame::new(Oid::new(READER), d);
    let measured = measure(&window(), &frame, &counts(m), method).expect("measurable");
    format!(
        "measured\naddress {}\nwindow {}\nframe {} reader {READER} streams {d}\n{words}\nsamples {} {}\n",
        measured.oid().as_str(),
        window().oid().as_str(),
        frame.oid().as_str(),
        m.len(),
        measured.samples_oid().as_str(),
    )
}

const TWO: [&[i64]; 4] = [&[1, 2], &[2, 2], &[3, 5], &[4, 7]];
const TWO_JSON: &str = "[[1,2],[2,2],[3,5],[4,7]]";

/// prismqueer/tests/measured.rs: K = S⁻¹ = [[8, −4], [−4, 20/9]], exactly.
#[test]
fn two_streams_under_mle_are_exact_rationals() {
    let a = run(&args(TWO_JSON, 2, "mle", "[[0,1]]"));
    assert!(!is_error(&a));
    let method = Method {
        estimator: Estimator::Mle,
        support: Support::full(2),
    };
    assert_eq!(
        text(&a),
        header(&TWO, 2, method, "method mle support 0-1") + "row 0 8 -4\nrow 1 -4 20/9\n"
    );
    assert!(!text(&a).contains('.'), "never a decimal");
}

/// n − 1: S scales by 4/3, so K by 3/4.
#[test]
fn the_unbiased_estimator_divides_by_n_minus_one() {
    let a = run(&args(TWO_JSON, 2, "unbiased", "[[1,0]]"));
    assert!(!is_error(&a));
    let method = Method {
        estimator: Estimator::Unbiased,
        support: Support::full(2),
    };
    assert_eq!(
        text(&a),
        header(&TWO, 2, method, "method unbiased support 0-1") + "row 0 6 -3\nrow 1 -3 5/3\n"
    );
}

/// The path A–B–C: K̂_AC is an exact zero, declared.
#[test]
fn the_path_has_an_exact_zero_where_no_edge_is_declared() {
    let three: [&[i64]; 4] = [&[0, 0, 0], &[1, 1, 0], &[2, 1, 2], &[1, 2, 2]];
    let a = run(&args(
        "[[0,0,0],[1,1,0],[2,1,2],[1,2,2]]",
        3,
        "mle",
        "[[0,1],[1,2]]",
    ));
    assert!(!is_error(&a));
    let method = Method {
        estimator: Estimator::Mle,
        support: Support::new([(0, 1), (1, 2)]),
    };
    assert_eq!(
        text(&a),
        header(&three, 3, method, "method mle support 0-1 1-2")
            + "row 0 8/3 -4/3 0\nrow 1 -4/3 14/3 -2\nrow 2 0 -2 2\n"
    );
}

// --- Refusals are answers ----------------------------------------------------------

fn context(d: usize, words: &str) -> String {
    format!(
        "window {}\nframe {} reader {READER} streams {d}\n{words}\n",
        window().oid().as_str(),
        Frame::new(Oid::new(READER), d).oid().as_str(),
    )
}

#[test]
fn the_four_cycle_is_unresolved_never_a_number() {
    let a = run(&args("[[0,0,0,0]]", 4, "mle", "[[0,1],[1,2],[2,3],[3,0]]"));
    assert!(!is_error(&a), "a refusal is an answer");
    assert_eq!(
        text(&a),
        "refused unresolved support 0-1 0-3 1-2 2-3\n".to_string()
            + &context(4, "method mle support 0-1 0-3 1-2 2-3")
    );
}

#[test]
fn every_refusal_is_an_answer_with_its_kind() {
    let cases = [
        (
            args("[[1,2],[2,2]]", 2, "mle", "[[0,1]]"),
            "refused too-few-samples samples 2 needed 3\n".to_string()
                + &context(2, "method mle support 0-1"),
        ),
        (
            args("[[1,1],[1,1],[1,1]]", 2, "mle", "[[0,1]]"),
            "refused not-invertible clique 0 1\n".to_string()
                + &context(2, "method mle support 0-1"),
        ),
        (
            args("[[1,2],[1,2,3]]", 2, "mle", "[]"),
            "refused wrong-shape sample 1 len 3 streams 2\n".to_string()
                + &context(2, "method mle support none"),
        ),
        (
            args("[[1,2]]", 2, "mle", "[[0,5]]"),
            "refused wrong-shape edge 0 5\n".to_string() + &context(2, "method mle support 0-5"),
        ),
        (
            args("[[1,2]]", 2, "unbiased", "[]"),
            "refused too-few-samples samples 1 needed 2\n".to_string()
                + &context(2, "method unbiased support none"),
        ),
    ];
    for (args, want) in cases {
        let a = run(&args);
        assert!(!is_error(&a), "{args}");
        assert_eq!(text(&a), want, "{args}");
    }
}

// --- It never seals ------------------------------------------------------------------

#[test]
fn measure_takes_no_signer_and_prints_none() {
    let with_by = format!(
        r#"{{"samples":{TWO_JSON},"streams":2,"estimator":"mle","support":[[0,1]],"window":"{}","reader":"{READER}","by":"{READER}"}}"#,
        window().oid().as_str()
    );
    let a = run(&with_by);
    assert_eq!(code(&a), -32602);
    assert_eq!(
        prism_mcp::json::print(get(&a, &["error", "data"])),
        r#"{"argument":"by","expected":"none"}"#
    );
    let b = run(&args(TWO_JSON, 2, "mle", "[[0,1]]"));
    for word in ["seal", "signer", "sealed", "metrix", " by "] {
        assert!(!text(&b).to_lowercase().contains(word), "{word}");
    }
    let out = ready(&[r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#.into()]);
    let Value::Array(tools) = get(&out[0], &["result", "tools"]) else {
        panic!("tools");
    };
    let m = tools
        .iter()
        .find(|t| str_of(get(t, &["name"])) == "measure")
        .expect("measure is listed");
    let Value::Object(properties) = get(m, &["inputSchema", "properties"]) else {
        panic!("properties");
    };
    let names: Vec<&str> = properties.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "estimator",
            "reader",
            "samples",
            "streams",
            "support",
            "window"
        ]
    );
    assert!(str_of(get(m, &["description"])).contains("never seals"));
}

// --- The wire: malformed is -32602 ------------------------------------------------

#[test]
fn malformed_arguments_are_invalid_params() {
    let w = window().oid().as_str().to_string();
    let base = |field: &str, value: &str| {
        let mut parts = vec![
            ("samples", "[[1,2]]".to_string()),
            ("streams", "2".to_string()),
            ("estimator", r#""mle""#.to_string()),
            ("support", "[]".to_string()),
            ("window", format!(r#""{w}""#)),
            ("reader", format!(r#""{READER}""#)),
        ];
        for p in &mut parts {
            if p.0 == field {
                p.1 = value.to_string();
            }
        }
        let body: Vec<String> = parts
            .iter()
            .filter(|p| !p.1.is_empty())
            .map(|(k, v)| format!(r#""{k}":{v}"#))
            .collect();
        format!("{{{}}}", body.join(","))
    };
    let cases = [
        (base("samples", r#"[[1,"2"]]"#), "samples.0"),
        (base("samples", "[[1,2.5]]"), "samples.0"),
        (base("samples", "[1,2]"), "samples.0"),
        (base("samples", ""), "samples"),
        (base("streams", "-1"), "streams"),
        (base("estimator", r#""mean""#), "estimator"),
        (base("support", "[[0]]"), "support.0"),
        (base("support", "[[0,-1]]"), "support.0"),
        (base("window", r#""not-hex""#), "window"),
        (base("reader", r#""""#), "reader"),
    ];
    for (args, argument) in cases {
        let a = run(&args);
        assert_eq!(code(&a), -32602, "{args}");
        let data = prism_mcp::json::print(get(&a, &["error", "data"]));
        assert!(
            data.starts_with(&format!(r#"{{"argument":"{argument}","#)),
            "{args}: {data}"
        );
    }
}
