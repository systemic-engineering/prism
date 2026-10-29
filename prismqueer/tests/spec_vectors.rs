//! prism-001: language-neutral test vectors for `terni::Imperfect`.
//!
//! The vectors in `docs/spec/imperfect-vectors.json` are generated here, from
//! the crate itself, through terni's own serde wire format. So they are true
//! by construction. This test fails if the committed file and the crate's
//! behaviour ever drift apart.
//!
//! Regenerate after an intended change:
//! `PRISM_SPEC_UPDATE=1 cargo test -p prismqueer --test spec_vectors`

use serde_json::{json, Value};
use terni::{Eh, Imperfect, Loss};

/// A loss type the vectors exercise, with two small named losses.
trait Fixture: Loss + serde::Serialize + 'static {
    const NAME: &'static str;
    fn small() -> Self;
    fn other() -> Self;
}

impl Fixture for usize {
    const NAME: &'static str = "count";
    fn small() -> Self {
        1
    }
    fn other() -> Self {
        3
    }
}

impl Fixture for Vec<String> {
    const NAME: &'static str = "list";
    fn small() -> Self {
        vec!["line 4 not parsed".into()]
    }
    fn other() -> Self {
        vec!["field 'date' dropped".into()]
    }
}

type I<L> = Imperfect<i64, String, L>;

fn wire<T: serde::Serialize>(x: &T) -> Value {
    serde_json::to_value(x).expect("serializable")
}

fn inputs<L: Fixture>() -> Vec<I<L>> {
    vec![
        Imperfect::Success(2),
        Imperfect::Partial(2, L::small()),
        Imperfect::Partial(2, L::zero()),
        Imperfect::Failure("boom".into(), L::small()),
    ]
}

fn nexts<L: Fixture>() -> Vec<I<L>> {
    vec![
        Imperfect::Success(10),
        Imperfect::Partial(10, L::other()),
        Imperfect::Partial(10, L::zero()),
        Imperfect::Failure("bad".into(), L::other()),
    ]
}

fn case(loss: &str, op: &str, input: Value, args: Value, expected: Value) -> Value {
    json!({ "loss": loss, "op": op, "input": input, "args": args, "expected": expected })
}

fn cases<L: Fixture>() -> Vec<Value> {
    let n = L::NAME;
    let mut out = Vec::new();
    for x in inputs::<L>() {
        out.push(case(
            n,
            "map",
            wire(&x),
            json!({}),
            wire(&x.clone().map(|v| v + 1)),
        ));
        out.push(case(
            n,
            "map_err",
            wire(&x),
            json!({}),
            wire(&x.clone().map_err(|e| format!("{e}!"))),
        ));
        out.push(case(n, "loss", wire(&x), json!({}), wire(&x.loss())));
        out.push(case(n, "ok", wire(&x), json!({}), wire(&x.clone().ok())));
        out.push(case(n, "err", wire(&x), json!({}), wire(&x.clone().err())));
        let result: Result<i64, String> = x.clone().into();
        let result = match result {
            Ok(v) => json!({ "ok": v }),
            Err(e) => json!({ "err": e }),
        };
        out.push(case(n, "into_result", wire(&x), json!({}), result));
        out.push(case(
            n,
            "unwrap_or",
            wire(&x),
            json!({ "default": 7 }),
            wire(&x.clone().unwrap_or(7)),
        ));
        for next in nexts::<L>() {
            let args = json!({ "next": wire(&next) });
            let bound = x.clone().eh(|v| next.clone().map(|w| v + w));
            out.push(case(n, "eh", wire(&x), args.clone(), wire(&bound)));
            let recovered = x.clone().recover(|_| next.clone());
            out.push(case(n, "recover", wire(&x), args.clone(), wire(&recovered)));
            let composed: I<L> = x.clone().compose(next.clone());
            out.push(case(n, "compose", wire(&x), args, wire(&composed)));
        }
    }
    let ok: I<L> = Ok(5).into();
    let err: I<L> = Err("nope".to_string()).into();
    out.push(case(
        n,
        "from_result",
        json!({ "ok": 5 }),
        json!({}),
        wire(&ok),
    ));
    out.push(case(
        n,
        "from_result",
        json!({ "err": "nope" }),
        json!({}),
        wire(&err),
    ));
    let some: Imperfect<i64, (), L> = Some(5).into();
    let none: Imperfect<i64, (), L> = None.into();
    out.push(case(n, "from_option", json!(5), json!({}), wire(&some)));
    out.push(case(n, "from_option", Value::Null, json!({}), wire(&none)));

    let sequences: Vec<Vec<I<L>>> = vec![
        vec![],
        vec![Imperfect::Success(1)],
        vec![Imperfect::Partial(1, L::small())],
        vec![Imperfect::Partial(1, L::zero())],
        vec![
            Imperfect::Partial(1, L::small()),
            Imperfect::Success(1),
            Imperfect::Partial(1, L::other()),
        ],
    ];
    for steps in sequences {
        let mut ctx = Eh::<L>::new();
        for s in steps.clone() {
            let _ = ctx.eh(s);
        }
        let finished: I<L> = ctx.finish(1);
        let steps: Vec<Value> = steps.iter().map(wire).collect();
        out.push(case(
            n,
            "eh_context",
            Value::Null,
            json!({ "steps": steps, "finish": 1 }),
            wire(&finished),
        ));
    }
    let mut ctx = Eh::<L>::new();
    let _ = ctx.eh(Imperfect::<i64, String, L>::Partial(1, L::small()));
    let stopped = ctx.eh(Imperfect::<i64, String, L>::Failure(
        "stop".into(),
        L::other(),
    ));
    let steps = vec![
        wire(&I::<L>::Partial(1, L::small())),
        wire(&I::<L>::Failure("stop".into(), L::other())),
    ];
    out.push(case(
        n,
        "eh_context_failure",
        Value::Null,
        json!({ "steps": steps }),
        json!({ "error": stopped.err(), "accumulated": wire(&ctx.into_loss()) }),
    ));

    let pairs = [
        (L::zero(), L::small()),
        (L::small(), L::zero()),
        (L::small(), L::other()),
        (L::other(), L::small()),
        (L::total(), L::small()),
        (L::small(), L::total()),
    ];
    for (a, b) in pairs {
        let args = json!({ "a": wire(&a), "b": wire(&b) });
        out.push(case(n, "combine", Value::Null, args, wire(&a.combine(b))));
    }
    out.push(case(n, "zero", Value::Null, json!({}), wire(&L::zero())));
    out.push(case(n, "total", Value::Null, json!({}), wire(&L::total())));
    for l in [L::zero(), L::small()] {
        out.push(case(
            n,
            "is_zero",
            Value::Null,
            json!({ "loss": wire(&l) }),
            json!(l.is_zero()),
        ));
    }
    out
}

fn document() -> Value {
    let mut all = cases::<usize>();
    all.extend(cases::<Vec<String>>());
    json!({
        "spec": "docs/spec/imperfect.md",
        "source": "terni 0.7.0 (prism/imperfect), generated by prismqueer/tests/spec_vectors.rs",
        "wire": "terni serde: {status: success, value} | {status: partial, value, loss} | {status: failure, error, loss}",
        "types": { "value": "integer", "error": "string" },
        "losses": {
            "count": "non-negative integer; zero = 0; combine = saturating add; total = 18446744073709551615 (u64 max, absorbing)",
            "list": "list of strings; zero = []; combine = append; total = [] (NOT absorbing, see spec)"
        },
        "functions": {
            "map": "v -> v + 1",
            "map_err": "e -> e + \"!\"",
            "eh": "v -> args.next with its value increased by v",
            "recover": "e -> args.next",
            "unwrap_or": "args.default",
            "compose": "args.next",
            "eh_context": "feed each step to one context, then finish(args.finish)",
            "eh_context_failure": "feed each step; the Failure step returns its error; report the accumulated loss"
        },
        "cases": all
    })
}

#[test]
fn imperfect_vectors_match_the_crate() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/imperfect-vectors.json");
    let generated = serde_json::to_string_pretty(&document()).expect("json") + "\n";
    if std::env::var("PRISM_SPEC_UPDATE").is_ok() {
        std::fs::write(&path, &generated).expect("write vectors");
    }
    let committed =
        std::fs::read_to_string(&path).expect("docs/spec/imperfect-vectors.json must exist");
    assert!(
        committed == generated,
        "vectors drifted from terni; regenerate with PRISM_SPEC_UPDATE=1"
    );
}
