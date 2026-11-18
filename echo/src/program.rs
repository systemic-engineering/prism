//! Certificate replay, slice R3: P, F and check 1 (docs/spec/echo.md
//! §10.6).
//!
//! **P**, the program address (certificate §3.1 header; §3.1.1 A, B, G):
//! `fragment(τ @pact/program root, ℓ "program", sorted)` over one atom per
//! distinct record of the source other than `fact`. Each atom is
//! `<keyword>(sym "<canonical line>")` at `@pact/program`, the canonical
//! line being the whole record re-printed from its parse: the keyword, one
//! space, the body with single spaces, `, ` between arguments, literals and
//! key positions, `not ` before a negative literal, zero-ary atoms bare,
//! ` -> `, variables with their `?`.
//!
//! **F**, the facts address: `fragment(τ @pact/fact root, ℓ "facts",
//! sorted)` over the distinct fact atoms, typed constants keeping their
//! types (S4).
//!
//! **Which lines** (Note §6.7 H5): every line that read, rules,
//! declarations and queries left out by H1 or not; every fact that read.
//! Lines that don't read are in neither.
//!
//! **Which header the source calls for** (H5; certificate §3.1.1 F): an
//! `unknown` performance (total darkness) keeps the pre-hole header, `none`
//! for both after a line that doesn't read or a reader fault, P and F after
//! an engine fault; every other performance claims P and F. Whether the
//! source is `unknown` is R2's light ([`Light`]); where R2 can't tell and a
//! reader hole makes the answer turn on it, R3 can't tell either (R4).
//!
//! **Check 1** (certificate §4.1): the header's `program` and `facts`,
//! each against echo's own: `interval source source <claimed> <recomputed>
//! about none at none 0 by 1` (Note §6.2), `none` on whichever side has no
//! address.

use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::address::{self, Address, Tau};
use crate::certificate::Header;
use crate::dark2::{Hole, Light, Report};
use crate::interval::{Interval, Side};
use crate::source::{Literal, Pattern, Record, Source, Term};
use crate::value::{Atom, Value};

/// The header's `program` and `facts` the source calls for; `None` is a
/// printed `none`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    pub program: Option<Address>,
    pub facts: Option<Address>,
}

/// A constant as it stands in a canonical line: a plain `sym` as its name;
/// any other constant in its canonical text (fixpoint §5.0.1 item 6).
fn constant(v: &Value, out: &mut String) {
    match v {
        Value::Sym(s) => out.push_str(s),
        other => out.push_str(&other.to_string()),
    }
}

/// `p` bare, else `p(t₁, t₂)`.
fn atom<T>(pred: &str, args: &[T], term: impl Fn(&T, &mut String), out: &mut String) {
    out.push_str(pred);
    if args.is_empty() {
        return;
    }
    out.push('(');
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        term(a, out);
    }
    out.push(')');
}

fn term(t: &Term, out: &mut String) {
    match t {
        Term::Var(v) => {
            out.push('?');
            out.push_str(v);
        }
        Term::Const(c) => constant(c, out),
    }
}

fn pattern(p: &Pattern, out: &mut String) {
    atom(&p.pred, &p.args, term, out);
}

fn literal(l: &Literal, out: &mut String) {
    if l.negative {
        out.push_str("not ");
    }
    pattern(&l.atom, out);
}

/// A record's canonical line (certificate §3.1, §3.1.1 B, G); `None` for a
/// fact, which has no program atom.
pub fn canonical(record: &Record) -> Option<String> {
    let mut out = String::new();
    match record {
        Record::Rule { id, head, body } => {
            out.push_str("rule ");
            out.push_str(id);
            out.push(' ');
            pattern(head, &mut out);
            out.push_str(" requires ");
            for (i, l) in body.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                literal(l, &mut out);
            }
        }
        Record::Query(a) => {
            out.push_str("query ");
            atom(&a.pred, &a.args, constant, &mut out);
        }
        Record::Functional { pred, key, value } => {
            out.push_str("functional ");
            out.push_str(pred);
            out.push('(');
            out.push_str(&key.join(", "));
            out.push_str(") -> ");
            out.push_str(value);
        }
        Record::Fact(_) => return None,
    }
    Some(out)
}

/// A fragment over the distinct members, sorted (fixpoint §5.0.1 items 1,
/// 2: the reader collapses source-level repeats when it forms the set).
fn set(tau: &Tau, label: &str, members: BTreeSet<Address>) -> Address {
    let sorted: Vec<Address> = members.into_iter().collect();
    address::fragment(tau, label, &sorted)
}

/// P: the program fragment over every record that read, other than facts.
pub fn program(source: &Source) -> Address {
    let tau = Tau::pact_program();
    let members = source
        .lines
        .iter()
        .filter_map(|l| {
            let line = canonical(&l.record)?;
            let keyword = line.split(' ').next().unwrap_or_default().to_string();
            Some(address::atom(
                &tau,
                &Atom {
                    pred: keyword,
                    args: Vec::from([Value::Sym(line)]),
                },
            ))
        })
        .collect();
    set(&tau, "program", members)
}

/// F: the facts fragment over every fact that read, at the store's `τ`.
pub fn facts(source: &Source) -> Address {
    let tau = Tau::pact_fact();
    let members = source
        .lines
        .iter()
        .filter_map(|l| match &l.record {
            Record::Fact(a) => Some(address::atom(&tau, a)),
            _ => None,
        })
        .collect();
    set(&tau, "facts", members)
}

/// The header the source calls for (H5; §3.1.1 F); `None` where that turns
/// on light R2 can't tell (a reader hole beside [`Light::Open`]).
pub fn expected(source: &Source, report: &Report) -> Option<Expected> {
    let reader = report
        .holes
        .iter()
        .any(|h| matches!(h, Hole::Unread(_) | Hole::Repeat(_) | Hole::QueryArity(_)));
    match (report.light, reader) {
        (Light::Dark, true) => Some(Expected {
            program: None,
            facts: None,
        }),
        (Light::Open, true) => None,
        _ => Some(Expected {
            program: Some(program(source)),
            facts: Some(facts(source)),
        }),
    }
}

fn side(a: Option<Address>) -> Side {
    a.map_or(Side::None, Side::Addr)
}

/// Check 1: one `source` interval per header field that differs from what
/// the source calls for, `program` first.
pub fn judge(expected: &Expected, header: &Header) -> Vec<Interval> {
    [
        (header.program, expected.program),
        (header.facts, expected.facts),
    ]
    .into_iter()
    .filter(|(claimed, recomputed)| claimed != recomputed)
    .map(|(claimed, recomputed)| Interval::source(side(claimed), side(recomputed)))
    .collect()
}
