//! Certificate replay (prism-022 S1, S2): slice R1, the surface
//! (docs/spec/echo.md §10.1); slice R2, the source and the Dark-2 report
//! (§10.4; `crate::source`, `crate::dark2`); slice R3, P, F and check 1
//! (§10.6; `crate::program`); and slice R4, the positive run and checks 3,
//! 4, 5 and 8 (§10.8; `crate::model`). The intervals of every slice print
//! as one list, by stratum, then by bytes (Note §6.2).
//!
//! The output is Note v3.3.2 §6.3's, as mirror's v3.2 contract delta
//! (`docs/pin-batch/2026-10-30-tune-exit-contract-v3.2.md`) gives it: the
//! verdict line, then `heard` (§4.2, B8), then `implements`, then one
//! `interval` line per dissonance (§6.2), every line ending in LF. The
//! exits are Alex's: 0 rings, 1 dissonant or uncertain, 2 unknown.
//!
//! **The floor:** `rings` comes only out of [`conclude`] with nothing left
//! unreplayed. Every stratum echo can't replay yet is `unknown unreplayed
//! <stratum>`, the first in [`STRATA`]'s order. echo never claims a ring it
//! didn't check.
//!
//! This is bytes in, lines out: no file, clock, network or process (the
//! library is `no_std`). The binary reads its arguments and hands them
//! here.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::address::{self, Address};
use crate::certificate;
use crate::dark2;
use crate::facts::Position;
use crate::interval::{self, Interval};
use crate::model::{self, Meter};
use crate::program;
use crate::record;
use crate::source;

/// The prism commit whose `docs/spec/echo.md` echo implements: `fa96f40`,
/// version 5, §10 with R3 as built. Full and lowercase (Note §4.2, Q2:
/// prism's repository is SHA-1). Always an earlier commit than the one
/// that sets it (the self-reference rule).
pub const SPEC: &str = "fa96f40b3436a71490173fb99a513c03cb7199a7";

/// The git tree of `docs/spec/echo-vectors` echo was verified against: the
/// tree at `575d55b`, the R4 red commit that added Taut's Notes (38, 41,
/// 44a).
pub const VECTORS: &str = "49698ad6c68df77eefedd4e19336e2675c0e9ae0";

/// The work budget, in ground rule instances considered (certificate §3.2
/// UNKNOWN: "a work count … fixed as a constant in the checker's own
/// tree"; prism-022 S1: the tune's 10M; docs/spec/echo.md §10.2 Q21).
/// Counted, never timed.
pub const BUDGET: u64 = 10_000_000;

/// The strata of certificate replay, in the order echo replays them
/// (docs/spec/echo.md §10: the word each slice retires, R2 to R9).
pub const STRATA: [&str; 8] = [
    "source", "program", "model", "contexts", "verdict", "budget", "address", "score",
];

/// How many of [`STRATA`] echo replays so far: R2 retired `source`, R3
/// `program`, R4 `model`. Each slice removes its word from the front.
pub const REPLAYED: usize = 3;

/// `source`'s place in [`STRATA`]: check 2, whose interval bytes are open
/// (docs/spec/echo.md Q26), is still the source stratum's.
const SOURCE: usize = 0;

/// echo's four inputs, in the order every list of them is printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Source,
    Certificate,
    Record,
    Score,
}

impl Input {
    pub fn name(self) -> &'static str {
        match self {
            Input::Source => "source",
            Input::Certificate => "certificate",
            Input::Record => "record",
            Input::Score => "score",
        }
    }
}

/// The inputs' raw bytes. `None` is an input that couldn't be read; for
/// the score, `None` is "not given" and `Some(None)` "given, but couldn't
/// be read".
#[derive(Clone, Copy, Debug)]
pub struct Inputs<'a> {
    pub source: Option<&'a [u8]>,
    pub certificate: Option<&'a [u8]>,
    pub record: Option<&'a [u8]>,
    pub score: Option<Option<&'a [u8]>>,
}

/// The first line of echo's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// `rings`: every stratum replayed and agreed. Not reachable before R8.
    Rings,
    /// `dissonant <n>`: the intervals, sorted (Note §6.2), one line each.
    /// From R2 on, `<n>` counts the strata replayed so far.
    Dissonant(Vec<Interval>),
    /// `uncertain budget <n> ground rule instances`: echo's own budget ran
    /// out; a run with more could continue.
    Uncertain { budget: u64 },
    /// `unknown io <input> …`: every input that couldn't be read, in order.
    Io(Vec<Input>),
    /// `unknown unreadable <input>:<L>:<C>`: the first input that doesn't
    /// read, at its first unread byte.
    Unreadable { input: Input, at: Position },
    /// `unknown unreplayed <stratum>`: the first stratum echo has no method
    /// for yet.
    Unreplayed(&'static str),
}

/// echo's answer: the verdict and what it heard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub verdict: Verdict,
    /// BLAKE3 of each input's raw bytes, in [`Input`] order; `None` for an
    /// input not read (and for a score not given).
    pub heard: [Option<Address>; 4],
}

fn hex(out: &mut String, a: &Address) {
    for b in a {
        let _ = write!(out, "{b:02x}");
    }
}

impl Answer {
    /// The exit code (Note §6.3): 0 rings, 1 uncertain, 2 unknown.
    pub fn exit(&self) -> u8 {
        match self.verdict {
            Verdict::Rings => 0,
            Verdict::Dissonant(_) | Verdict::Uncertain { .. } => 1,
            Verdict::Io(_) | Verdict::Unreadable { .. } | Verdict::Unreplayed(_) => 2,
        }
    }

    /// The answer's lines, each ending in LF.
    pub fn print(&self) -> String {
        let mut out = match &self.verdict {
            Verdict::Rings => String::from("rings"),
            Verdict::Dissonant(intervals) => format!("dissonant {}", intervals.len()),
            Verdict::Uncertain { budget } => {
                format!("uncertain budget {budget} ground rule instances")
            }
            Verdict::Io(inputs) => {
                let mut s = String::from("unknown io");
                for i in inputs {
                    s.push(' ');
                    s.push_str(i.name());
                }
                s
            }
            Verdict::Unreadable { input, at } => {
                format!("unknown unreadable {}:{at}", input.name())
            }
            Verdict::Unreplayed(stratum) => format!("unknown unreplayed {stratum}"),
        };
        out.push_str("\nheard");
        let inputs = [
            Input::Source,
            Input::Certificate,
            Input::Record,
            Input::Score,
        ];
        for (input, heard) in inputs.iter().zip(&self.heard) {
            out.push(' ');
            out.push_str(input.name());
            out.push(' ');
            match heard {
                Some(a) => hex(&mut out, a),
                None => out.push_str("none"),
            }
        }
        let _ = write!(out, "\nimplements spec {SPEC} vectors {VECTORS} rig none\n");
        if let Verdict::Dissonant(intervals) = &self.verdict {
            for i in intervals {
                out.push_str(&i.text());
                out.push('\n');
            }
        }
        out
    }
}

/// The verdict once everything echo can replay agreed: `rings` only when
/// nothing is left unreplayed.
pub fn conclude(unreplayed: &[&'static str]) -> Verdict {
    match unreplayed.first() {
        Some(stratum) => Verdict::Unreplayed(stratum),
        None => Verdict::Rings,
    }
}

/// How far echo read one input (docs/spec/echo.md §11.2's coverage).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// Read whole by a stratum's reader.
    Read,
    /// Not read: an earlier answer ended the run.
    Stopped,
    /// Couldn't be read (`unknown io`).
    Io,
    /// Refused at its first unread byte.
    Unreadable(Position),
    /// Hashed for `heard`, never read (the score, until R9).
    Unread,
    /// Not given (the score).
    Absent,
}

/// Why a stratum that ran didn't decide (§11.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Undecided {
    /// Its checks turn on a later stratum, or it has nothing to judge
    /// until one runs.
    Waits,
    /// Its check's bytes are open in the spec (Q26, Q40–Q42).
    Open,
}

/// What one stratum's checks did on this input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The interval lines its checks found.
    pub intervals: usize,
    /// `None`: decided.
    pub undecided: Option<Undecided>,
}

/// How far one replay went, input by input and stratum by stratum. The
/// witness reports it (`crate::witness`); the verdict never reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    /// Each input, in [`Input`] order.
    pub inputs: [Reading; 4],
    /// Each replayed stratum, in [`STRATA`]'s order; `None` not reached.
    pub strata: [Option<Outcome>; REPLAYED],
    /// echo's own P and F (§10.6), once the source read.
    pub key: Option<(Address, Address)>,
}

/// Replay the inputs within `budget` ground rule instances. The binary
/// always passes [`BUDGET`].
pub fn replay(inputs: &Inputs<'_>, budget: u64) -> Answer {
    traced(inputs, budget).0
}

/// [`replay`], with the trace of how far it went.
pub fn traced(inputs: &Inputs<'_>, budget: u64) -> (Answer, Trace) {
    let heard = [
        inputs.source.map(address::hash),
        inputs.certificate.map(address::hash),
        inputs.record.map(address::hash),
        inputs.score.flatten().map(address::hash),
    ];
    let given = |i: Option<&[u8]>| i.map_or(Reading::Io, |_| Reading::Stopped);
    let mut trace = Trace {
        inputs: [
            given(inputs.source),
            given(inputs.certificate),
            given(inputs.record),
            match inputs.score {
                None => Reading::Absent,
                Some(None) => Reading::Io,
                Some(Some(_)) => Reading::Unread,
            },
        ],
        strata: [None; REPLAYED],
        key: None,
    };
    let verdict = judge(inputs, budget, &mut trace);
    (Answer { verdict, heard }, trace)
}

fn judge(inputs: &Inputs<'_>, budget: u64, trace: &mut Trace) -> Verdict {
    let (Some(source), Some(certificate), Some(record), Some(_)) = (
        inputs.source,
        inputs.certificate,
        inputs.record,
        inputs.score.unwrap_or(Some(&[])),
    ) else {
        let mut io = Vec::new();
        if inputs.source.is_none() {
            io.push(Input::Source);
        }
        if inputs.certificate.is_none() {
            io.push(Input::Certificate);
        }
        if inputs.record.is_none() {
            io.push(Input::Record);
        }
        if inputs.score == Some(None) {
            io.push(Input::Score);
        }
        return Verdict::Io(io);
    };
    let performance = match certificate::read(certificate) {
        Ok(p) => p,
        Err(r) => {
            trace.inputs[1] = Reading::Unreadable(r.at);
            return Verdict::Unreadable {
                input: Input::Certificate,
                at: r.at,
            };
        }
    };
    trace.inputs[1] = Reading::Read;
    let record = match record::read(record) {
        Ok(r) => r,
        Err(r) => {
            trace.inputs[2] = Reading::Unreadable(r.at);
            return Verdict::Unreadable {
                input: Input::Record,
                at: r.at,
            };
        }
    };
    // R2: the source and its Dark-2 report. The source is never
    // `unreadable`: a line that doesn't read is a hole (Note §5; Seam row
    // 8).
    trace.inputs[2] = Reading::Read;
    let source = source::read(source);
    trace.inputs[0] = Reading::Read;
    trace.key = Some((program::program(&source), program::facts(&source)));
    let report = dark2::report(&source);
    // R4: echo's own run, before anything is judged, so a budget that runs
    // out is `uncertain` whatever else differs. A positive source runs its
    // eval program; a W source only its positive core, and only where the
    // header turns on a certain paradox (H3 v3.3; Taut's 95, 97).
    let Ok(r4) = model::replay(&source, &report, &mut Meter::new(budget)) else {
        return Verdict::Uncertain { budget };
    };
    let mut intervals = Vec::new();
    // The first stratum some check couldn't replay, in [`STRATA`]'s order.
    let mut waiting: Option<usize> = None;
    let mut wait = |stratum: usize| waiting = Some(waiting.map_or(stratum, |w| w.min(stratum)));
    let outcome = |intervals: usize, undecided| {
        Some(Outcome {
            intervals,
            undecided,
        })
    };
    trace.strata[0] = match dark2::judge(&report, &performance, &record) {
        Some(found) => {
            let n = found.len();
            intervals.extend(found);
            outcome(n, None)
        }
        // Check 2, a model printed for a Dark source: the source stratum's
        // own check, whose bytes are open (Q26).
        None if report.light == dark2::Light::Dark => {
            wait(SOURCE);
            outcome(0, Some(Undecided::Open))
        }
        // Light R2 can't tell without a run.
        None => {
            wait(REPLAYED);
            outcome(0, Some(Undecided::Waits))
        }
    };
    // R3: P and F, check 1, where R2's light, or R4's run, decides which
    // header the source calls for.
    trace.strata[1] = match r4.header {
        Some(expected) => {
            let found = program::judge(&expected, &performance.header);
            let n = found.len();
            intervals.extend(found);
            outcome(n, None)
        }
        None => {
            wait(REPLAYED);
            outcome(0, Some(Undecided::Waits))
        }
    };
    // R4: checks 3, 4, 5 and 8 over a positive run the performance should
    // print: not `unknown` (a Dark source prints no model; an open light
    // could still be `unknown`, unless a paradox is certain), with no W
    // line, no budget and no `dark 2` claim (R5's, R7's, check 10's).
    trace.strata[2] = match &r4.run {
        Some(run) if r4.positive && r4.lit && model::judgeable(&performance) => {
            let judged = model::judge(&source, &r4.rules, run, &performance);
            let n = judged.intervals.len();
            intervals.extend(judged.intervals);
            if judged.undecided {
                wait(REPLAYED);
                outcome(n, Some(Undecided::Open))
            } else {
                outcome(n, None)
            }
        }
        _ => {
            wait(REPLAYED);
            outcome(0, Some(Undecided::Waits))
        }
    };
    if !intervals.is_empty() {
        // One list across strata: by stratum, then by bytes (Note §6.2).
        interval::sort(&mut intervals);
        return Verdict::Dissonant(intervals);
    }
    conclude(&STRATA[waiting.unwrap_or(REPLAYED)..])
}
