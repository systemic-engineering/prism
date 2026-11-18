//! Certificate replay, slice R2: the Dark-2 report, and what it judges
//! (docs/spec/echo.md §10.4).
//!
//! From the lines a source reads (`crate::source`), the holes of Note
//! v3.3.2 §6.7 H1, each with its canonical text:
//! - `# L:C`: a line that doesn't read;
//! - `repeat L:1`: a repeated rule id, at each later line (§3.1.1 J′);
//! - `arity L:1`: a query whose arity the program doesn't use (M4 (a): any
//!   of the predicate's arities counts);
//! - `arity p`: a predicate used with two arities, counted over every fact
//!   and rule that read, repeats included (G7);
//! - `safety r ?v`: a variable of the head or of a negative literal in no
//!   positive literal, over the rules repeats leave (v3.0.1);
//! - `declaration p <canonical line>`: an ill-formed declaration (H7
//!   v3.2.5: no arity, a position outside `1 … n`, a repeated key; a
//!   predicate with two arities speaks as `arity p` alone; a trivial one,
//!   its value among its keys, is no hole, v3.2.7).
//!
//! They are printed in A6's order (`#` by offset, the reader's by offset,
//! then arity, safety, declarations, G8). The head under
//! `@mirror/dark-order` (certificate §2.4; Note §6.4) merges the first two
//! by least byte offset and puts the engine's after them. `basis` is the
//! unread lines (§6.4).
//!
//! **Light (H3), as far as R2 can tell without a run.** The verdict
//! `unknown` (and with it a performance of one `dark 2` atom and no `hole`
//! line, H5) needs "no light at all", which in general is the run's (R4,
//! R5). R2 decides it only where the lines say it outright, and leaves the
//! rest [`Light::Open`].

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::address::{self, Tau};
use crate::certificate::{Dark, Engine, Performance};
use crate::facts::Position;
use crate::interval::{self, Interval, Side};
use crate::record::Record as RecordLines;
use crate::source::{Literal, Pattern, Record, Source, Term};
use crate::value::{Atom, Value};

/// One hole of H1's table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hole {
    /// `# L:C`
    Unread(Position),
    /// `repeat L:1`
    Repeat(usize),
    /// `arity L:1`
    QueryArity(usize),
    /// `arity p`
    Arity(String),
    /// `safety r ?v`
    Safety { rule: String, var: String },
    /// `declaration p functional p(k₁, …) -> v`
    Declaration { pred: String, line: String },
}

fn sym(s: &str) -> Value {
    Value::Sym(s.into())
}

impl Hole {
    /// H1's canonical text.
    pub fn text(&self) -> String {
        match self {
            Hole::Unread(at) => format!("# {at}"),
            Hole::Repeat(l) => format!("repeat {l}:1"),
            Hole::QueryArity(l) => format!("arity {l}:1"),
            Hole::Arity(p) => format!("arity {p}"),
            Hole::Safety { rule, var } => format!("safety {rule} ?{var}"),
            Hole::Declaration { pred, line } => format!("declaration {pred} {line}"),
        }
    }

    /// The performance's `hole(sym …)` atom: the text split at its first
    /// two spaces (H5, G8).
    pub fn atom(&self) -> Atom {
        hole_atom(&self.text())
    }

    /// The `dark 2` atom naming this hole (certificate §3.1): `at L:C` for
    /// the reader's (a reader check at column 1), `engine <check>
    /// <subject>` for the engine's (a declaration names its predicate
    /// only, 71c).
    fn dark(&self) -> Atom {
        let at = |line: usize, column: usize| {
            Vec::from([
                sym("at"),
                Value::Nat(line as u64),
                Value::Nat(column as u64),
            ])
        };
        let mut args = Vec::from([Value::Nat(2)]);
        args.extend(match self {
            Hole::Unread(p) => at(p.line, p.column),
            Hole::Repeat(l) | Hole::QueryArity(l) => at(*l, 1),
            Hole::Arity(p) => Vec::from([sym("engine"), sym("arity"), sym(p)]),
            Hole::Safety { rule, var } => Vec::from([
                sym("engine"),
                sym("safety"),
                sym(rule),
                sym(&format!("?{var}")),
            ]),
            Hole::Declaration { pred, .. } => {
                Vec::from([sym("engine"), sym("declaration"), sym(pred)])
            }
        });
        Atom {
            pred: "dark".into(),
            args,
        }
    }

    /// H1's `A` is non-empty: the hole can add atoms.
    fn adds(&self) -> bool {
        !matches!(self, Hole::QueryArity(_) | Hole::Declaration { .. })
    }
}

/// A hole text as an atom: `hole(sym w₁, sym w₂[, sym rest])`.
fn hole_atom(text: &str) -> Atom {
    Atom {
        pred: "hole".into(),
        args: text.splitn(3, ' ').map(sym).collect(),
    }
}

/// How much light the source leaves, as far as R2 can tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Light {
    /// No hole: nothing for R2 to tell.
    NoHole,
    /// No light at all (H3 `unknown`): nothing left to run could give a
    /// light truth or a light falsehood.
    Dark,
    /// Some light: a fact is left that no hole and no declaration can
    /// remove (H2), so the verdict is `uncertain` or `paradox`.
    Lit,
    /// R2 can't tell: the run decides (R4), or W beside a hole that adds
    /// (H1: `unknown shadow under W`, R5).
    Open,
}

/// The Dark-2 report of one source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// H1's holes, in A6's order.
    pub holes: Vec<Hole>,
    /// The head under `@mirror/dark-order`: an index into `holes`.
    head: Option<usize>,
    /// The unread lines, ascending.
    pub basis: Vec<usize>,
    pub light: Light,
}

impl Report {
    /// The `dark 2` atom a total-dark performance carries: the head.
    pub fn head(&self) -> Option<Atom> {
        self.head.map(|i| self.holes[i].dark())
    }
}

/// The arities each predicate is used with, in facts and rules.
fn arities<'a>(
    rules: impl Iterator<Item = (&'a Pattern, &'a [Literal])>,
    facts: &[&Atom],
) -> BTreeMap<String, BTreeSet<usize>> {
    let mut out: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for (head, body) in rules {
        for p in core::iter::once(head).chain(body.iter().map(|l| &l.atom)) {
            out.entry(p.pred.clone()).or_default().insert(p.args.len());
        }
    }
    for f in facts {
        out.entry(f.pred.clone()).or_default().insert(f.args.len());
    }
    out
}

fn vars(p: &Pattern) -> impl Iterator<Item = &String> {
    p.args.iter().filter_map(|t| match t {
        Term::Var(v) => Some(v),
        Term::Const(_) => None,
    })
}

/// A declaration's canonical line (certificate §3.1.1 B, G).
fn canonical(pred: &str, key: &[String], value: &str) -> String {
    format!("functional {pred}({}) -> {value}", key.join(", "))
}

/// A position as a number; one too large to be any arity is `None`.
fn position(p: &str) -> Option<usize> {
    p.parse().ok()
}

/// The Dark-2 report of `source`.
pub fn report(source: &Source) -> Report {
    let mut rules = Vec::new();
    let mut facts = Vec::new();
    let mut queries = Vec::new();
    let mut declarations = Vec::new();
    for l in &source.lines {
        match &l.record {
            Record::Rule { id, head, body } => rules.push((id, head, body, l)),
            Record::Fact(a) => facts.push(a),
            Record::Query(a) => queries.push((a, l)),
            Record::Functional { pred, key, value } => declarations.push((pred, key, value)),
        }
    }
    let arity = arities(rules.iter().map(|(_, h, b, _)| (*h, b.as_slice())), &facts);
    let conflict: BTreeSet<&String> = arity
        .iter()
        .filter(|(_, a)| a.len() > 1)
        .map(|(p, _)| p)
        .collect();

    // The reader's checks, each with its byte offset.
    let mut reader: Vec<(usize, Hole)> = Vec::new();
    let mut seen: BTreeMap<&String, usize> = BTreeMap::new();
    for (id, _, _, l) in &rules {
        let n = seen.entry(*id).or_default();
        *n += 1;
        if *n > 1 {
            reader.push((l.offset, Hole::Repeat(l.line)));
        }
    }
    let repeated: BTreeSet<&String> = seen
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(id, _)| *id)
        .collect();
    let mut bad_queries = BTreeSet::new();
    for (q, l) in &queries {
        let fits = arity
            .get(&q.pred)
            .is_some_and(|a| a.contains(&q.args.len()));
        if !fits {
            reader.push((l.offset, Hole::QueryArity(l.line)));
            bad_queries.insert(l.line);
        }
    }
    reader.sort_by_key(|(o, _)| *o);

    // The engine's: arity, safety, declarations.
    let mut engine: Vec<Hole> = conflict.iter().map(|p| Hole::Arity((*p).clone())).collect();
    let mut safety = Vec::new();
    let mut unsafe_rules = BTreeSet::new();
    for (id, head, body, _) in rules.iter().filter(|(id, ..)| !repeated.contains(id)) {
        let positive: BTreeSet<&String> = body
            .iter()
            .filter(|l| !l.negative)
            .flat_map(|l| vars(&l.atom))
            .collect();
        let named: BTreeSet<&String> = vars(head)
            .chain(
                body.iter()
                    .filter(|l| l.negative)
                    .flat_map(|l| vars(&l.atom)),
            )
            .collect();
        for v in named.difference(&positive) {
            safety.push(((*id).clone(), (*v).clone()));
            unsafe_rules.insert(*id);
        }
    }
    safety.sort();
    engine.extend(
        safety
            .into_iter()
            .map(|(rule, var)| Hole::Safety { rule, var }),
    );
    let mut ill: BTreeSet<(String, String)> = BTreeSet::new();
    let mut declared = BTreeSet::new();
    for (pred, key, value) in &declarations {
        let line = canonical(pred, key, value);
        if conflict.contains(pred) {
            continue;
        }
        let n = arity.get(*pred).and_then(|a| a.first().copied());
        let positions: Vec<Option<usize>> = key.iter().map(|k| position(k)).collect();
        let inside = |p: Option<usize>| n.is_some_and(|n| p.is_some_and(|p| p <= n));
        let keys: BTreeSet<Option<usize>> = positions.iter().copied().collect();
        let well_formed = n.is_some()
            && positions.iter().all(|p| inside(*p))
            && inside(position(value))
            && keys.len() == positions.len();
        if !well_formed {
            ill.insert(((*pred).clone(), line));
        } else if !keys.contains(&position(value)) {
            declared.insert((*pred).clone());
        }
    }
    let ill_preds: BTreeSet<String> = ill.iter().map(|(p, _)| p.clone()).collect();
    engine.extend(
        ill.into_iter()
            .map(|(pred, line)| Hole::Declaration { pred, line }),
    );

    // A6's order, and the head of the merged order.
    let mut holes: Vec<Hole> = source.unread.iter().map(|u| Hole::Unread(u.at)).collect();
    holes.extend(reader.iter().map(|(_, h)| h.clone()));
    let head = if holes.is_empty() {
        (!engine.is_empty()).then_some(0)
    } else {
        let offsets = source
            .unread
            .iter()
            .map(|u| u.offset)
            .chain(reader.iter().map(|(o, _)| *o));
        offsets.enumerate().min_by_key(|(_, o)| *o).map(|(i, _)| i)
    };
    holes.extend(engine);

    // What is left to run: the eval program (H1's "left out").
    let mentions = |p: &Pattern, body: &[Literal]| {
        core::iter::once(p)
            .chain(body.iter().map(|l| &l.atom))
            .any(|a| conflict.contains(&a.pred))
    };
    let eval_rules: Vec<&Vec<Literal>> = rules
        .iter()
        .filter(|(id, h, b, _)| {
            !repeated.contains(id) && !unsafe_rules.contains(id) && !mentions(h, b)
        })
        .map(|(_, _, b, _)| *b)
        .collect();
    let eval_facts: Vec<&&Atom> = facts
        .iter()
        .filter(|f| !conflict.contains(&f.pred))
        .collect();
    let eval_queries = queries
        .iter()
        .filter(|(q, l)| !bad_queries.contains(&l.line) && !conflict.contains(&q.pred))
        .count();
    let unread = !source.unread.is_empty();

    let light = if holes.is_empty() {
        Light::NoHole
    } else if eval_facts.is_empty()
        && eval_rules
            .iter()
            .all(|b| b.iter().any(|l| !l.negative) && b.iter().all(|l| !l.negative))
        && (eval_queries == 0 || unread)
    {
        Light::Dark
    } else if holes.iter().any(Hole::adds)
        && rules
            .iter()
            .any(|(_, _, b, _)| b.iter().any(|l| l.negative))
    {
        Light::Open
    } else {
        // Every predicate a hole or a declaration could remove (H1's `D`,
        // the general rule, a paradox's group): D(#) is every predicate
        // used with arity >= 2 in a line that read; a declaration hole's,
        // its predicate at arity >= 2 (G1); and every declared predicate.
        let wide = |p: &String| {
            arity.get(p).is_some_and(|a| a.iter().any(|&n| n >= 2))
                || queries
                    .iter()
                    .any(|(q, _)| &q.pred == p && q.args.len() >= 2)
        };
        let removable = |p: &String| {
            (unread && wide(p)) || (ill_preds.contains(p) && wide(p)) || declared.contains(p)
        };
        if eval_facts.iter().any(|f| !removable(&f.pred)) {
            Light::Lit
        } else {
            Light::Open
        }
    };
    let basis = source.unread.iter().map(|u| u.at.line).collect();
    Report {
        holes,
        head,
        basis,
        light,
    }
}

/// `unread <runs>` (§6.4): maximal runs joined by `, `, a run of two or
/// more as `a-b`; `None` when nothing was unread.
pub fn basis_text(lines: &[usize]) -> Option<String> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &l in lines {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == l => *end = l,
            _ => runs.push((l, l)),
        }
    }
    let first = runs.first()?;
    let run = |(a, b): &(usize, usize)| {
        if a == b {
            format!("{a}")
        } else {
            format!("{a}-{b}")
        }
    };
    let mut out = format!("unread {}", run(first));
    for r in &runs[1..] {
        out.push_str(", ");
        out.push_str(&run(r));
    }
    Some(out)
}

/// The score's `basis(nat l₁, …)` atom (§6.4), at `@song/interval`.
pub fn basis_atom(lines: &[usize]) -> Atom {
    Atom {
        pred: "basis".into(),
        args: lines.iter().map(|&l| Value::Nat(l as u64)).collect(),
    }
}

/// The table's hole types (H1); a `hole` line of another type (`budget`,
/// `capacity`, `io`) is not the source's to judge.
const TABLE: [&str; 5] = ["#", "repeat", "arity", "safety", "declaration"];

fn in_table(text: &str) -> bool {
    TABLE.contains(&text.split(' ').next().unwrap_or_default())
}

/// The multiset difference: what only `claimed` has, and what only
/// `derived` has.
fn difference<T: Ord + Clone>(claimed: &[T], derived: &[T]) -> (Vec<T>, Vec<T>) {
    let mut c = claimed.to_vec();
    let mut d = derived.to_vec();
    c.sort();
    d.sort();
    let (mut only_c, mut only_d) = (Vec::new(), Vec::new());
    let (mut i, mut j) = (0, 0);
    while i < c.len() || j < d.len() {
        match (c.get(i), d.get(j)) {
            (Some(x), Some(y)) if x == y => {
                i += 1;
                j += 1;
            }
            (Some(x), Some(y)) if x < y => {
                only_c.push(x.clone());
                i += 1;
            }
            (Some(x), None) => {
                only_c.push(x.clone());
                i += 1;
            }
            (_, Some(y)) => {
                only_d.push(y.clone());
                j += 1;
            }
            (None, None) => break,
        }
    }
    (only_c, only_d)
}

/// One `hole` interval per text claimed or expected but not both (H7).
fn holes(
    out: &mut Vec<Interval>,
    claimed: &[String],
    derived: &[String],
    about: &'static str,
    side: fn(&str) -> Side,
) {
    let (only_c, only_d) = difference(claimed, derived);
    for t in &only_c {
        out.push(Interval::verdict("hole", side(t), Side::None, about));
    }
    for t in &only_d {
        out.push(Interval::verdict("hole", Side::None, side(t), about));
    }
}

/// The `dark 2` atom a performance line claims; `None` for a `dark 255`
/// (R6's).
fn claimed_dark(d: &Dark) -> Option<Atom> {
    let words = |w: &[&str]| w.iter().map(|s| sym(s)).collect::<Vec<_>>();
    let mut args = Vec::from([Value::Nat(2)]);
    match d {
        Dark::At { line, column } => {
            args.extend([sym("at"), Value::Nat(*line), Value::Nat(*column)]);
        }
        Dark::Engine(Engine::Arity(p)) => args.extend(words(&["engine", "arity", p])),
        Dark::Engine(Engine::Safety { rule, var }) => {
            args.extend(words(&["engine", "safety", rule, &format!("?{var}")]));
        }
        Dark::Engine(Engine::Declaration(p)) => {
            args.extend(words(&["engine", "declaration", p]));
        }
        Dark::Declined(_) => return None,
    }
    Some(Atom {
        pred: "dark".into(),
        args,
    })
}

/// Judge the performance and the record against the report: check 10,
/// composed (Note §6.4), under the hole (H4, H5, H7). `None` where R2
/// can't tell; otherwise every interval R2 owns, sorted (none: R2 agrees).
pub fn judge(
    report: &Report,
    performance: &Performance,
    record: &RecordLines,
) -> Option<Vec<Interval>> {
    let carries_model = performance.header.result.is_some()
        || !performance.layers.is_empty()
        || !performance.derives.is_empty()
        || !performance.contexts.is_empty();
    match report.light {
        Light::Open => return None,
        // Check 2: a model printed for a Dark source. Its interval's bytes
        // are open (docs/spec/echo.md Q26).
        Light::Dark if carries_model => return None,
        _ => {}
    }
    let expected: Vec<String> = report.holes.iter().map(Hole::text).collect();
    let mut out = Vec::new();

    // The record's `hole` lines, by hole text (the value before its first
    // ` #`; the said text is never compared).
    let claimed: Vec<String> = record
        .lines
        .iter()
        .filter(|l| l.key == "hole")
        .map(|l| {
            let end = l
                .value
                .windows(2)
                .position(|w| w == b" #")
                .unwrap_or(l.value.len());
            l.value[..end]
                .iter()
                .map(|&b| b as char)
                .collect::<String>()
        })
        .filter(|t| in_table(t))
        .collect();
    holes(&mut out, &claimed, &expected, "record", |t| {
        Side::Addr(address::hash(t.as_bytes()))
    });

    // The performance's: `hole` lines in an uncertain or paradox one only,
    // and one `dark 2` atom, the head, in an unknown one (H5).
    let dark = report.light == Light::Dark;
    let claimed: Vec<String> = performance
        .holes
        .iter()
        .map(|(t, _)| t.clone())
        .filter(|t| in_table(t))
        .collect();
    let derived = if dark { Vec::new() } else { expected };
    holes(&mut out, &claimed, &derived, "performance", |t| {
        Side::Addr(address::atom(&Tau::mirror_certificate(), &hole_atom(t)))
    });
    let claimed: Vec<Atom> = performance
        .dark
        .iter()
        .filter_map(|(d, _)| claimed_dark(d))
        .collect();
    let derived: Vec<Atom> = if dark {
        report.head().into_iter().collect()
    } else {
        Vec::new()
    };
    let (only_c, only_d) = difference(&claimed, &derived);
    let addr = |a: &Atom| Side::Addr(address::atom(&Tau::mirror_certificate(), a));
    if let ([c], [d]) = (only_c.as_slice(), only_d.as_slice()) {
        out.push(Interval::verdict("dark", addr(c), addr(d), "performance"));
    } else {
        for c in &only_c {
            out.push(Interval::verdict(
                "dark",
                addr(c),
                Side::None,
                "performance",
            ));
        }
        for d in &only_d {
            out.push(Interval::verdict(
                "dark",
                Side::None,
                addr(d),
                "performance",
            ));
        }
    }
    interval::sort(&mut out);
    Some(out)
}
