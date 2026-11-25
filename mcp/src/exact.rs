//! The tools on prism's floor, prism-laws (docs/spec/mcp.md §5.3, §5.4):
//! `check` (`agreement::check`) and `measure` (`measured::measure`), exact
//! over ℤ, ℤ/n, ℚ/ℤ and ℚ. Integers travel as JSON integers and rationals
//! as `"p/q"` strings; nothing is ever a decimal.
//!
//! - **The line between an answer and no answer** is mcp.md §5's subject
//!   and ruler. `check`'s verdicts (`in`, `out`, `needs-ruling`) and
//!   `measure`'s refusals (`unresolved`, `too-few-samples`, …) are answers,
//!   `isError: false`. A selection that doesn't read the relations' items,
//!   or a value outside the declared ring, leaves nothing to check: no
//!   verdict, `isError: true`.
//! - **No keys.** `check`'s `by` is the label `SelectionDraft::seal` puts
//!   beside the selection's address (`algedonic::Signer`, a signature
//!   *claim*, never verified). `measure` never seals at all.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use prism_laws::agreement::{self, Consistent, Reading, Relations, SelectionDraft, Verdict};
use prism_laws::field::Edge;
use prism_laws::hypothesis::Frame;
use prism_laws::measured::{self, Estimator, Method, Shape, Support};
use prism_laws::oid::Oid;
use prism_laws::room::{Cycle, Modulus, QModOne, ZMod, Q, Z};

use crate::json::Value;
use crate::server::{object, text, Error};
use crate::tools::{names, only, result, schema, Args};

// ------------------------------------------------------------- the wire

/// `^[0-9a-f]{64}$`: an Oid as prism prints one.
fn oid(args: &Args, key: &str) -> Result<Oid, Error> {
    let hex = || Error::params(key, "64 lowercase hex");
    match args.get(key) {
        Some(Value::Str(s))
            if s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
        {
            Ok(Oid::new(s.as_str()))
        }
        _ => Err(hex()),
    }
}

fn natural(v: &Value) -> Option<u64> {
    match v {
        Value::Int(n) => u64::try_from(*n).ok(),
        _ => None,
    }
}

fn array<'a>(v: Option<&'a Value>, key: &str, expected: &str) -> Result<&'a [Value], Error> {
    match v {
        Some(Value::Array(items)) => Ok(items),
        _ => Err(Error::params(key, expected)),
    }
}

/// A value as it travels: an integer, or `"p/q"`.
enum Wire {
    Int(i64),
    Ratio(i64, i64),
}

/// `-?(0|[1-9][0-9]*)` within i64; `-0` refused, as the reader does.
fn integer(s: &str) -> Option<i64> {
    let digits = s.strip_prefix('-').unwrap_or(s);
    let canonical = digits == "0" || (!digits.is_empty() && !digits.starts_with('0'));
    if !canonical || !digits.bytes().all(|b| b.is_ascii_digit()) || s == "-0" {
        return None;
    }
    s.parse().ok()
}

/// `p/q`: p an integer, q a positive one.
fn ratio(s: &str) -> Option<(i64, i64)> {
    let (p, q) = s.split_once('/')?;
    let (p, q) = (integer(p)?, integer(q)?);
    (q > 0).then_some((p, q))
}

fn wire(v: &Value) -> Option<Wire> {
    match v {
        Value::Int(n) => Some(Wire::Int(*n)),
        Value::Str(s) => ratio(s).map(|(p, q)| Wire::Ratio(p, q)),
        _ => None,
    }
}

/// An exact rational, `p` or `p/q` in lowest terms.
fn rational(q: &Q) -> String {
    let r = q.value();
    if r.is_integer() {
        r.numer().to_string()
    } else {
        format!("{}/{}", r.numer(), r.denom())
    }
}

fn joined<T: ToString>(items: impl IntoIterator<Item = T>) -> String {
    items
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// No verdict: the ruler is broken (§5).
fn broken(why: String) -> Result<Value, Error> {
    Ok(result(alloc::vec![why + "\n"], true))
}

// ---------------------------------------------------------------- check

const CHECK: &str = "Checks a selection against exact relations among items \
                     (prism_laws::agreement::check). Each relation [from, to, t] says \
                     s(to) \u{2212} s(from) = t in one ring: Z, ZMod (with its modulus) or \
                     QModOne (values as \"p/q\" strings); items are integers \u{2265} 0. The \
                     answer is in (every relation, required and found, and the selection's \
                     place in the coset of consistent readings), out (exactly the violated \
                     relations) or needs-ruling (the relations are curved, so no selection \
                     could be in: the witness cycles with their twists, never a pick). A \
                     selection that doesn't read exactly the relations' items, or a value \
                     outside the ring, gets no verdict (isError). `by` is who the selection is \
                     attributed to: sealing puts it as a label beside the selection's address \
                     (algedonic::Signer), never verified; prism holds no keys and never signs.";

const MEASURE: &str = "Estimates a precision matrix exactly over Q from integer counts \
                       (prism_laws::measured::measure): samples (rows of `streams` integers), \
                       the estimator mle (divide by n) or unbiased (by n \u{2212} 1), and the \
                       declared support edges [i, j], a zero off them declared conditional \
                       independence. Full support gives the inverse sample covariance, chordal \
                       support the covariance-selection MLE in closed form; non-chordal \
                       support is refused as unresolved, never approximated. Entries are exact \
                       rationals (\"p/q\"), never decimals, with the measurement's address. \
                       Refusals (unresolved, too-few-samples, not-invertible, wrong-shape) are \
                       answers. The window is named by its address, the frame by its reader \
                       and the number of streams. It measures and never seals: no draft, no \
                       signer.";

fn hex_schema(description: &str) -> Value {
    object([
        ("description", text(description)),
        ("pattern", text("^[0-9a-f]{64}$")),
        ("type", text("string")),
    ])
}

fn rows_schema(description: &str, items: Value) -> Value {
    object([
        ("description", text(description)),
        ("items", items),
        ("type", text("array")),
    ])
}

fn natural_schema() -> Value {
    object([("minimum", Value::Int(0)), ("type", text("integer"))])
}

pub(crate) fn check_tool() -> Value {
    object([
        ("name", text("check")),
        ("title", text("check: is a selection in the consistent readings?")),
        ("description", text(CHECK)),
        (
            "inputSchema",
            schema(
                [
                    (
                        "ring",
                        object([
                            ("enum", names(["Z", "ZMod", "QModOne"])),
                            ("type", text("string")),
                        ]),
                    ),
                    (
                        "modulus",
                        object([
                            ("description", text("n of ZMod, n \u{2265} 2; ZMod only.")),
                            ("minimum", Value::Int(2)),
                            ("type", text("integer")),
                        ]),
                    ),
                    (
                        "items",
                        rows_schema("Items beyond those the relations name.", natural_schema()),
                    ),
                    (
                        "relations",
                        rows_schema(
                            "[from, to, t]: s(to) \u{2212} s(from) = t; t an integer, or \"p/q\" in QModOne.",
                            object([("type", text("array"))]),
                        ),
                    ),
                    (
                        "selection",
                        rows_schema(
                            "[item, reading], one per item.",
                            object([("type", text("array"))]),
                        ),
                    ),
                    (
                        "by",
                        hex_schema("Who the selection is attributed to: a label, never a key."),
                    ),
                ],
                ["ring", "relations", "selection", "by"],
            ),
        ),
    ])
}

pub(crate) fn measure_tool() -> Value {
    let pair = object([("items", natural_schema()), ("type", text("array"))]);
    object([
        ("name", text("measure")),
        ("title", text("measure: a precision matrix, exactly")),
        ("description", text(MEASURE)),
        (
            "inputSchema",
            schema(
                [
                    (
                        "samples",
                        rows_schema(
                            "n samples, each `streams` integer counts.",
                            object([
                                ("items", object([("type", text("integer"))])),
                                ("type", text("array")),
                            ]),
                        ),
                    ),
                    (
                        "streams",
                        object([
                            ("description", text("d, the frame's dimension.")),
                            ("minimum", Value::Int(0)),
                            ("type", text("integer")),
                        ]),
                    ),
                    (
                        "estimator",
                        object([
                            ("enum", names(["mle", "unbiased"])),
                            ("type", text("string")),
                        ]),
                    ),
                    (
                        "support",
                        rows_schema("The declared edges [i, j] between streams.", pair),
                    ),
                    ("window", hex_schema("The window ruler's address.")),
                    ("reader", hex_schema("The frame's reader.")),
                ],
                [
                    "samples",
                    "streams",
                    "estimator",
                    "support",
                    "window",
                    "reader",
                ],
            ),
        ),
    ])
}

/// The ring a check reads in.
enum Ring {
    Z,
    ZMod(Modulus),
    QModOne,
}

/// Rows of `[natural…, value]`, shape-checked: `width` naturals, then an
/// integer or a `"p/q"`.
fn rows(
    args: &Args,
    key: &str,
    expected: &str,
    width: usize,
) -> Result<Vec<(Vec<u64>, Wire)>, Error> {
    let items = array(args.get(key), key, expected)?;
    items
        .iter()
        .enumerate()
        .map(|(k, row)| {
            let bad = || Error::params(&format!("{key}.{k}"), expected);
            let Value::Array(cells) = row else {
                return Err(bad());
            };
            if cells.len() != width + 1 {
                return Err(bad());
            }
            let ids = cells[..width]
                .iter()
                .map(natural)
                .collect::<Option<Vec<u64>>>()
                .ok_or_else(bad)?;
            let value = wire(&cells[width]).ok_or_else(bad)?;
            Ok((ids, value))
        })
        .collect()
}

pub(crate) fn check(args: &Args) -> Result<Value, Error> {
    only(
        args,
        &["ring", "modulus", "items", "relations", "selection", "by"],
        "",
    )?;
    let ring_name = match args.get("ring") {
        Some(Value::Str(s)) if ["Z", "ZMod", "QModOne"].contains(&s.as_str()) => s.as_str(),
        _ => return Err(Error::params("ring", "Z, ZMod or QModOne")),
    };
    let modulus = match (ring_name, args.get("modulus")) {
        ("ZMod", Some(Value::Int(n))) => Some(*n),
        ("ZMod", _) => return Err(Error::params("modulus", "integer")),
        (_, Some(_)) => return Err(Error::params("modulus", "none")),
        (_, None) => None,
    };
    let items = match args.get("items") {
        None => Vec::new(),
        Some(v) => {
            let items = array(Some(v), "items", "array of integers \u{2265} 0")?;
            items
                .iter()
                .enumerate()
                .map(|(k, v)| {
                    natural(v)
                        .ok_or_else(|| Error::params(&format!("items.{k}"), "integer \u{2265} 0"))
                })
                .collect::<Result<Vec<u64>, Error>>()?
        }
    };
    let relations = rows(args, "relations", "[from, to, value]", 2)?;
    let selection = rows(args, "selection", "[item, value]", 1)?;
    let by = oid(args, "by")?;

    let ring = match (ring_name, modulus) {
        ("Z", _) => Ring::Z,
        ("QModOne", _) => Ring::QModOne,
        (_, m) => {
            let m = m.unwrap_or_default();
            match Modulus::new(m) {
                Some(n) => Ring::ZMod(n),
                None => return broken(format!("error ring modulus {m}")),
            }
        }
    };
    let input = Input {
        items,
        relations,
        selection,
        by,
    };
    match &ring {
        Ring::Z => run(
            &(),
            "Z".into(),
            &input,
            |w| match w {
                Wire::Int(n) => Some(Z::from(*n)),
                Wire::Ratio(..) => None,
            },
            "integer",
            |z| z.value().to_string(),
        ),
        Ring::ZMod(n) => run(
            n,
            format!("ZMod {}", n.value()),
            &input,
            |w| match w {
                Wire::Int(x) => Some(ZMod::new(*x, n)),
                Wire::Ratio(..) => None,
            },
            "integer",
            |z| z.residue().to_string(),
        ),
        Ring::QModOne => run(
            &(),
            "QModOne".into(),
            &input,
            |w| match w {
                Wire::Ratio(p, q) => QModOne::new(*p, *q),
                Wire::Int(_) => None,
            },
            "p/q",
            |q| {
                if q.denominator().to_string() == "1" {
                    q.numerator().to_string()
                } else {
                    format!("{}/{}", q.numerator(), q.denominator())
                }
            },
        ),
    }
}

/// A check's arguments, shape-checked, before the ring reads the values.
struct Input {
    items: Vec<u64>,
    relations: Vec<(Vec<u64>, Wire)>,
    selection: Vec<(Vec<u64>, Wire)>,
    by: Oid,
}

fn edge(e: &Edge) -> String {
    let (a, b) = e.ends();
    format!("{a}-{b}#{}", e.nth())
}

fn cycle<V>(c: &Cycle<V>, word: &impl Fn(&V) -> String) -> String {
    format!(
        "cycle twist {} vertices {} edges {}\n",
        word(c.pairing()),
        joined(c.vertices()),
        joined(c.edges().iter().map(edge))
    )
}

/// Read the values in the ring, check, and print the verdict.
fn run<V: Reading>(
    ring: &V::Ring,
    ring_word: String,
    input: &Input,
    read: impl Fn(&Wire) -> Option<V>,
    expected: &str,
    word: impl Fn(&V) -> String,
) -> Result<Value, Error> {
    let mut relations = Vec::new();
    for (k, (ids, w)) in input.relations.iter().enumerate() {
        let Some(v) = read(w) else {
            return broken(format!("error ring relations.{k} {expected}"));
        };
        relations.push((ids[0], ids[1], v));
    }
    let mut readings = Vec::new();
    for (k, (ids, w)) in input.selection.iter().enumerate() {
        let Some(v) = read(w) else {
            return broken(format!("error ring selection.{k} {expected}"));
        };
        readings.push((ids[0], v));
    }
    let Ok(relations) = Relations::new(ring, input.items.iter().copied(), relations) else {
        return broken("error ring relations".into());
    };
    let selection = SelectionDraft::new(readings).seal(input.by.clone());
    let verdict = match agreement::check(&relations, &selection) {
        Ok(v) => v,
        Err(agreement::Refused::Missing(v)) => {
            return broken(format!("error selection missing {v}"))
        }
        Err(agreement::Refused::Unknown(v)) => {
            return broken(format!("error selection unknown {v}"))
        }
        Err(agreement::Refused::Room(_)) => return broken("error ring selection".into()),
    };
    let head = |w: &str| {
        format!(
            "{w}\nring {ring_word}\nselection {} by {}\n",
            selection.oid().as_str(),
            selection.signer().by().as_str()
        )
    };
    let out = match verdict {
        Verdict::In(certificate) => {
            let mut out = head("in");
            for c in certificate.checked() {
                out.push_str(&format!(
                    "edge {} required {} found {}\n",
                    edge(&c.edge()),
                    word(c.required()),
                    word(c.found())
                ));
            }
            // One coordinate per free direction, named by its smallest item.
            if let Ok(Consistent::Readings { free, .. }) = relations.consistent() {
                for (d, x) in free.iter().zip(certificate.coordinates()) {
                    if let Some(item) = d.items().first() {
                        out.push_str(&format!("coordinate {item} {}\n", word(x)));
                    }
                }
            }
            out
        }
        Verdict::Out(violated) => {
            let mut out = head("out");
            for c in &violated {
                out.push_str(&format!(
                    "violated {} required {} found {}\n",
                    edge(&c.edge()),
                    word(c.required()),
                    word(c.found())
                ));
            }
            out
        }
        Verdict::NeedsRuling(cycles) => {
            let mut out = head("needs-ruling");
            for c in &cycles {
                out.push_str(&cycle(c, &word));
            }
            out
        }
    };
    Ok(result(alloc::vec![out], false))
}

// -------------------------------------------------------------- measure

fn support_words(s: &Support) -> String {
    if s.edges().is_empty() {
        "none".into()
    } else {
        joined(s.edges().iter().map(|(i, j)| format!("{i}-{j}")))
    }
}

fn refusal(r: &measured::Refused) -> String {
    match r {
        measured::Refused::WrongShape(Shape::Sample {
            index,
            len,
            streams,
        }) => format!("wrong-shape sample {index} len {len} streams {streams}"),
        measured::Refused::WrongShape(Shape::Edge { from, to }) => {
            format!("wrong-shape edge {from} {to}")
        }
        measured::Refused::Unresolved(s) => format!("unresolved support {}", support_words(s)),
        measured::Refused::TooFewSamples { samples, needed } => {
            format!("too-few-samples samples {samples} needed {needed}")
        }
        measured::Refused::NotInvertible { clique } => {
            format!("not-invertible clique {}", joined(clique))
        }
        // Only sealing refuses these, and this tool never seals.
        measured::Refused::OtherWindow { .. } => "other-window".into(),
        measured::Refused::Draft(_) => "draft".into(),
    }
}

pub(crate) fn measure(args: &Args) -> Result<Value, Error> {
    only(
        args,
        &[
            "samples",
            "streams",
            "estimator",
            "support",
            "window",
            "reader",
        ],
        "",
    )?;
    let rows = array(
        args.get("samples"),
        "samples",
        "array of arrays of integers",
    )?;
    let counts = rows
        .iter()
        .enumerate()
        .map(|(k, row)| {
            let bad = || Error::params(&format!("samples.{k}"), "array of integers");
            let Value::Array(cells) = row else {
                return Err(bad());
            };
            cells
                .iter()
                .map(|c| match c {
                    Value::Int(n) => Ok(Z::from(*n)),
                    _ => Err(bad()),
                })
                .collect::<Result<Vec<Z>, Error>>()
        })
        .collect::<Result<Vec<Vec<Z>>, Error>>()?;
    let streams = args
        .get("streams")
        .and_then(natural)
        .and_then(|d| usize::try_from(d).ok())
        .ok_or_else(|| Error::params("streams", "integer \u{2265} 0"))?;
    let estimator = match args.get("estimator") {
        Some(Value::Str(s)) if s == "mle" => Estimator::Mle,
        Some(Value::Str(s)) if s == "unbiased" => Estimator::Unbiased,
        _ => return Err(Error::params("estimator", "mle or unbiased")),
    };
    let pairs = array(args.get("support"), "support", "array of [i, j]")?;
    let edges = pairs
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let bad = || Error::params(&format!("support.{k}"), "[i, j]");
            let ends = match p {
                Value::Array(e) if e.len() == 2 => (natural(&e[0]), natural(&e[1])),
                _ => return Err(bad()),
            };
            match ends {
                (Some(i), Some(j)) => usize::try_from(i)
                    .ok()
                    .zip(usize::try_from(j).ok())
                    .ok_or_else(bad),
                _ => Err(bad()),
            }
        })
        .collect::<Result<Vec<(usize, usize)>, Error>>()?;
    let window = oid(args, "window")?;
    let reader = oid(args, "reader")?;

    let frame = Frame::new(reader.clone(), streams);
    let method = Method {
        estimator,
        support: Support::new(edges),
    };
    let context = format!(
        "window {}\nframe {} reader {} streams {streams}\nmethod {} support {}\n",
        window.as_str(),
        frame.oid().as_str(),
        reader.as_str(),
        match estimator {
            Estimator::Mle => "mle",
            Estimator::Unbiased => "unbiased",
        },
        support_words(&method.support)
    );
    let out = match measured::measure_under(window, &frame, &counts, method) {
        Err(r) => format!("refused {}\n{context}", refusal(&r)),
        Ok(m) => {
            let mut out = format!("measured\naddress {}\n{context}", m.oid().as_str());
            out.push_str(&format!(
                "samples {} {}\n",
                m.samples(),
                m.samples_oid().as_str()
            ));
            for (i, row) in m.estimate().iter().enumerate() {
                out.push_str(&format!("row {i} {}\n", joined(row.iter().map(rational))));
            }
            out
        }
    };
    Ok(result(alloc::vec![out], false))
}
