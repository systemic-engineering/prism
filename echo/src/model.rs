//! Certificate replay, slice R4: the positive run (docs/spec/echo.md §10.8).
//!
//! echo evaluates the positive program itself, from the definitions, never
//! from the certificate:
//! - **the least model** by naive evaluation (certificate §1: "a tick is
//!   also the naive stage `min{n : a ∈ T_Pⁿ(F)}`"): stage 0 is the eval
//!   program's facts; round `n` forms every ground instance of every rule
//!   whose positive premises are all in stage `n − 1`, and stage `n` adds
//!   their heads; the run ends at the first round that adds nothing, which
//!   is check 5's closure round;
//! - **the depth** of an atom is the round that first adds it, facts at 0
//!   (§1's pinned conventions: `1 + max` over its first-tick derivations);
//! - **the first-tick derivations** (§2.1) are the instances formed in the
//!   round that first adds their head: exactly those whose positive
//!   premises are all at depths below the head's. A head that is a fact has
//!   none (Note §6.5 G5).
//!
//! **The budget** (certificate §3.2 UNKNOWN; §4.3): every ground rule
//! instance echo forms, in every round, the closure round included, is one
//! unit. Counted, never timed.
//!
//! **What R4 judges** (checks 3, 4, 5 and 8, certificate §4.1; Note §6.2's
//! `structure` rows): the certificate's `layer` lines against the model and
//! its depths (`stray`, `depth`, `unlayered`), and its `derive` lines
//! against the first-tick derivations as Notes (`missing`, `extra`).
//!
//! Every map and set here is ordered (`BTreeMap`, `BTreeSet`): no hash
//! order reaches the output.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use crate::address::{self, Address, Tau};
use crate::certificate::{Dark, Performance, Run as Beat};
use crate::dark2::{Hole, Light, Report};
use crate::interval::{Interval, Side};
use crate::program::{self, Expected};
use crate::source::{Literal, Pattern, Record, Source, Term};
use crate::value::{Atom, Value};

/// The work meter: ground rule instances, counted, never timed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meter {
    left: u64,
    spent: u64,
}

/// The budget ran out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exhausted;

impl Meter {
    pub fn new(budget: u64) -> Meter {
        Meter {
            left: budget,
            spent: 0,
        }
    }

    /// Charge one ground rule instance.
    pub fn spend(&mut self) -> Result<(), Exhausted> {
        self.left = self.left.checked_sub(1).ok_or(Exhausted)?;
        self.spent += 1;
        Ok(())
    }

    /// The instances charged so far.
    pub fn spent(&self) -> u64 {
        self.spent
    }
}

/// One rule of the program echo runs: its id, its canonical line
/// (certificate §3.1.1 B) and its parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule<'a> {
    pub id: &'a str,
    pub line: String,
    pub head: &'a Pattern,
    pub body: &'a [Literal],
}

/// A ground rule instance: the head, the rule (an index into the rules the
/// run was given) and the positive premises in body order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instance {
    pub head: Atom,
    pub rule: usize,
    pub premises: Vec<Atom>,
}

/// The positive run: the least model with each atom's depth, and the
/// first-tick derivations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Positive {
    pub depth: BTreeMap<Atom, u64>,
    pub derivations: BTreeSet<Instance>,
}

impl Positive {
    /// `M`'s address: `fragment(@pact/fact root, "model", sorted)` over
    /// every atom of the model, facts included (certificate §3.1).
    pub fn model(&self) -> Address {
        let tau = Tau::pact_fact();
        let members: BTreeSet<Address> =
            self.depth.keys().map(|a| address::atom(&tau, a)).collect();
        let sorted: Vec<Address> = members.into_iter().collect();
        address::fragment(&tau, "model", &sorted)
    }

    /// The deepest tick, `None` for an empty model.
    pub fn top(&self) -> Option<u64> {
        self.depth.values().copied().max()
    }
}

/// Whether the source is a positive program: no `not` in any rule that
/// read (certificate §3.2: "a `not` anywhere means W"; Q31).
pub fn positive(source: &Source) -> bool {
    !source.lines.iter().any(|l| match &l.record {
        Record::Rule { body, .. } => body.iter().any(|b| b.negative),
        _ => false,
    })
}

/// The eval program's facts and rules (H1), in source order. With `core`,
/// only its rules with no `not` literal: the positive core (H3 v3.3).
pub fn program<'a>(source: &'a Source, report: &Report, core: bool) -> (Vec<Atom>, Vec<Rule<'a>>) {
    let mut facts = Vec::new();
    let mut rules = Vec::new();
    for &i in &report.eval {
        let l = &source.lines[i];
        match &l.record {
            Record::Fact(a) => facts.push(a.clone()),
            Record::Rule { id, head, body } => {
                if core && body.iter().any(|b| b.negative) {
                    continue;
                }
                rules.push(Rule {
                    id,
                    line: program::canonical(&l.record).unwrap_or_default(),
                    head,
                    body,
                });
            }
            _ => {}
        }
    }
    (facts, rules)
}

type Binding = BTreeMap<String, Value>;

/// Match `pattern` against `atom` under `binding`; the extended binding, or
/// `None`.
fn unify(pattern: &Pattern, atom: &Atom, binding: &Binding) -> Option<Binding> {
    if pattern.pred != atom.pred || pattern.args.len() != atom.args.len() {
        return None;
    }
    let mut out = binding.clone();
    for (t, v) in pattern.args.iter().zip(&atom.args) {
        match t {
            Term::Const(c) => {
                if c != v {
                    return None;
                }
            }
            Term::Var(x) => match out.get(x) {
                Some(bound) if bound != v => return None,
                Some(_) => {}
                None => {
                    out.insert(x.clone(), v.clone());
                }
            },
        }
    }
    Some(out)
}

/// `pattern` under `binding`; `None` if a variable is unbound (an unsafe
/// rule, which the eval program never holds).
fn ground(pattern: &Pattern, binding: &Binding) -> Option<Atom> {
    let args = pattern
        .args
        .iter()
        .map(|t| match t {
            Term::Const(c) => Some(c.clone()),
            Term::Var(x) => binding.get(x).cloned(),
        })
        .collect::<Option<Vec<Value>>>()?;
    Some(Atom {
        pred: pattern.pred.clone(),
        args,
    })
}

/// Every ground instance of `rule` whose positive premises are all in the
/// stage, in a fixed order (body order, then the stage's atom order).
fn instances(
    rule: &Rule<'_>,
    index: &BTreeMap<&str, Vec<&Atom>>,
    out: &mut Vec<(Atom, Vec<Atom>)>,
) {
    let positive: Vec<&Pattern> = rule
        .body
        .iter()
        .filter(|l| !l.negative)
        .map(|l| &l.atom)
        .collect();
    let mut partial: Vec<(Binding, Vec<Atom>)> = Vec::from([(Binding::new(), Vec::new())]);
    for p in &positive {
        let candidates = index.get(p.pred.as_str()).map_or(&[][..], |v| v.as_slice());
        let mut next = Vec::new();
        for (binding, premises) in &partial {
            for a in candidates {
                if let Some(b) = unify(p, a, binding) {
                    let mut ps = premises.clone();
                    ps.push((*a).clone());
                    next.push((b, ps));
                }
            }
        }
        partial = next;
    }
    for (binding, premises) in partial {
        if let Some(head) = ground(rule.head, &binding) {
            out.push((head, premises));
        }
    }
}

/// The least model of `facts` and `rules`' positive literals, by naive
/// evaluation, with depths and first-tick derivations. Every instance
/// formed is charged to `meter`.
pub fn evaluate(
    facts: &[Atom],
    rules: &[Rule<'_>],
    meter: &mut Meter,
) -> Result<Positive, Exhausted> {
    let mut run = Positive::default();
    for f in facts {
        run.depth.insert(f.clone(), 0);
    }
    let mut round = 0;
    loop {
        round += 1;
        let mut index: BTreeMap<&str, Vec<&Atom>> = BTreeMap::new();
        for a in run.depth.keys() {
            index.entry(a.pred.as_str()).or_default().push(a);
        }
        let mut added: BTreeSet<Atom> = BTreeSet::new();
        let mut found: Vec<Instance> = Vec::new();
        for (r, rule) in rules.iter().enumerate() {
            let mut formed = Vec::new();
            instances(rule, &index, &mut formed);
            for (head, premises) in formed {
                meter.spend()?;
                if !run.depth.contains_key(&head) {
                    added.insert(head.clone());
                    found.push(Instance {
                        head,
                        rule: r,
                        premises,
                    });
                }
            }
        }
        if added.is_empty() {
            return Ok(run);
        }
        for a in added {
            run.depth.insert(a, round);
        }
        run.derivations.extend(found);
    }
}

/// Whether a declaration of the eval program is violated in `stable`: some
/// key group with two different values (H3 v3.3: "some key group has two
/// different values, both in the stable set"). A trivial declaration (its
/// value among its keys) governs nothing.
pub fn certain_paradox(source: &Source, report: &Report, stable: &BTreeMap<Atom, u64>) -> bool {
    report.eval.iter().any(|&i| match &source.lines[i].record {
        Record::Functional { pred, key, value } => {
            let pos = |s: &String| s.parse::<usize>().ok().and_then(|p| p.checked_sub(1));
            let (Some(keys), Some(v)) = (
                key.iter().map(pos).collect::<Option<Vec<usize>>>(),
                pos(value),
            ) else {
                return false;
            };
            if keys.contains(&v) {
                return false;
            }
            let mut groups: BTreeMap<Vec<&Value>, BTreeSet<&Value>> = BTreeMap::new();
            for a in stable.keys().filter(|a| &a.pred == pred) {
                let k: Option<Vec<&Value>> = keys.iter().map(|&p| a.args.get(p)).collect();
                if let (Some(k), Some(val)) = (k, a.args.get(v)) {
                    groups.entry(k).or_default().insert(val);
                }
            }
            groups.values().any(|vals| vals.len() > 1)
        }
        _ => false,
    })
}

/// `negs` for a positive rule: the empty set (Note §2.1; Taut's 41).
fn empty_negs() -> Address {
    address::fragment(&Tau::pact_fact(), "negs", &[])
}

/// A Note's address (Note §2.1, §2.2): `note(addr head, addr rule, addr
/// negs, addr p₁, …)` at `@song/note` root, the rule its record atom's
/// address at `@pact/program` (§3.1.1 B), `negs` the empty set's.
pub fn note(head: &Atom, rule_line: &str, premises: &[Atom]) -> Address {
    let fact = Tau::pact_fact();
    let keyword = rule_line.split(' ').next().unwrap_or_default();
    let rule = address::atom(
        &Tau::pact_program(),
        &Atom {
            pred: keyword.into(),
            args: Vec::from([Value::Sym(rule_line.into())]),
        },
    );
    let mut args = Vec::from([
        Value::Addr(address::atom(&fact, head)),
        Value::Addr(rule),
        Value::Addr(empty_negs()),
    ]);
    args.extend(
        premises
            .iter()
            .map(|p| Value::Addr(address::atom(&fact, p))),
    );
    address::atom(
        &Tau::song_note(),
        &Atom {
            pred: "note".into(),
            args,
        },
    )
}

/// What R4 found: the `structure` intervals, and whether some claim had no
/// pinned bytes to print (docs/spec/echo.md §10.9), so the stratum stays
/// unreplayed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Judged {
    pub intervals: Vec<Interval>,
    pub undecided: bool,
}

/// Checks 3, 4, 5 and 8 (certificate §4.1) over a positive performance:
/// its layers against the model and its depths, its derivations against
/// the first-tick derivations, as Notes.
pub fn judge(
    source: &Source,
    rules: &[Rule<'_>],
    run: &Positive,
    performance: &Performance,
) -> Judged {
    let tau = Tau::pact_fact();
    let addr = |a: &Atom| Side::Addr(address::atom(&tau, a));
    let mut out = Judged::default();

    // Checks 3 and 4: every layer line against the model.
    let mut agreed: BTreeSet<&Atom> = BTreeSet::new();
    let mut layered: BTreeSet<&Atom> = BTreeSet::new();
    for l in &performance.layers {
        layered.insert(&l.atom);
        match run.depth.get(&l.atom) {
            None => out.intervals.push(Interval::structure(
                "stray",
                addr(&l.atom),
                Side::None,
                l.tick,
                1,
            )),
            Some(&d) if d != l.tick => out.intervals.push(Interval::structure(
                "depth",
                addr(&l.atom),
                Side::None,
                l.tick,
                l.tick.abs_diff(d),
            )),
            Some(_) => {
                // The same line twice: a repeat in a set (§3.2), whose
                // interval has no pinned kind (Q42).
                if !agreed.insert(&l.atom) {
                    out.undecided = true;
                }
            }
        }
    }
    // Checks 4 and 5: every model atom is layered.
    for (a, &d) in &run.depth {
        if !layered.contains(a) {
            out.intervals
                .push(Interval::structure("unlayered", Side::None, addr(a), d, 1));
        }
    }

    // Check 8: the derivations, as Notes, heads in the model only.
    let derived: BTreeMap<Address, &Atom> = run
        .derivations
        .iter()
        .map(|i| (note(&i.head, &rules[i.rule].line, &i.premises), &i.head))
        .collect();
    let mut matched: BTreeSet<Address> = BTreeSet::new();
    for d in &performance.derives {
        let Some(&depth) = run.depth.get(&d.head) else {
            // Not in the model: `stray` names it by its layer. A head that
            // isn't layered either has no pinned interval (Q40).
            if !layered.contains(&d.head) {
                out.undecided = true;
            }
            continue;
        };
        let mut named = source.lines.iter().filter_map(|l| match &l.record {
            Record::Rule { id, .. } if id == &d.rule => program::canonical(&l.record),
            _ => None,
        });
        let (Some(line), None) = (named.next(), named.next()) else {
            // No rule, or two, with that id: the Note has no rule address
            // (Q41).
            out.undecided = true;
            continue;
        };
        let n = note(&d.head, &line, &d.premises);
        if derived.contains_key(&n) && matched.insert(n) {
            continue;
        }
        out.intervals.push(Interval::structure(
            "extra",
            Side::Addr(n),
            Side::None,
            depth,
            1,
        ));
    }
    for (n, head) in &derived {
        if !matched.contains(n) {
            let depth = run.depth.get(*head).copied().unwrap_or_default();
            out.intervals.push(Interval::structure(
                "missing",
                Side::None,
                Side::Addr(*n),
                depth,
                1,
            ));
        }
    }
    out
}

/// Whether the performance is one R4 judges as a positive run: no W line
/// (`under`, `over`, `context`: check 6 is R5's), no budget `erasure`
/// (R7's), and no `dark 2` claim, which certificate §3.2 judges by checks
/// 10 and 1 (R2, R3; Q43).
pub fn judgeable(performance: &Performance) -> bool {
    performance.layers.iter().all(|l| l.run == Beat::Tick)
        && performance.derives.iter().all(|d| d.run == Beat::Tick)
        && performance.contexts.is_empty()
        && performance.erasures.is_empty()
        && !performance
            .dark
            .iter()
            .any(|(d, _)| matches!(d, Dark::At { .. } | Dark::Engine(_)))
}

/// What R4 makes of a source before anything is judged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replay<'a> {
    /// A positive program (no `not` in a rule that read).
    pub positive: bool,
    /// The rules echo ran: the eval program's, or its positive core's.
    pub rules: Vec<Rule<'a>>,
    /// The run: a positive source's eval program; a W source's positive
    /// core where the header turns on it; else none (R5's).
    pub run: Option<Positive>,
    /// A declaration is certainly violated in the run (H3 v3.3).
    pub paradox: bool,
    /// The header the source calls for (R3's, decided further by the run
    /// where R3 couldn't tell); `None` where it still turns on light.
    pub header: Option<Expected>,
    /// Whether the performance should print this run: the light is not
    /// `unknown` (no hole; lit; or open with a certain paradox).
    pub lit: bool,
}

/// R4 before the performance: run what the source calls for, within the
/// meter, and decide what the run can decide of the header (H3 v3.3, H5):
/// a certain paradox is `paradox`, so P and F; W beside a hole that adds,
/// with none, is `unknown`, so the pre-hole header, `none` after the
/// reader's fault (Q37).
pub fn replay<'a>(
    source: &'a Source,
    report: &Report,
    meter: &mut Meter,
) -> Result<Replay<'a>, Exhausted> {
    let positive = positive(source);
    let header = program::expected(source, report);
    let core = !positive && header.is_none() && report.holes.iter().any(Hole::adds);
    let (facts, rules) = program(source, report, core);
    let run = if positive || core {
        Some(evaluate(&facts, &rules, meter)?)
    } else {
        None
    };
    let paradox = run
        .as_ref()
        .is_some_and(|r| certain_paradox(source, report, &r.depth));
    let header = header.or(match (paradox, core) {
        (true, _) => Some(Expected {
            program: Some(program::program(source)),
            facts: Some(program::facts(source)),
        }),
        (false, true) => Some(Expected {
            program: None,
            facts: None,
        }),
        (false, false) => None,
    });
    let lit = match report.light {
        Light::NoHole | Light::Lit => true,
        Light::Open => paradox,
        Light::Dark => false,
    };
    Ok(Replay {
        positive,
        rules,
        run,
        paradox,
        header,
        lit,
    })
}
