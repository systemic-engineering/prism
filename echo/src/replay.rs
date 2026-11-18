//! Certificate replay (prism-022 S1, S2): slice R1, the surface
//! (docs/spec/echo.md §10.1); slice R2, the source and the Dark-2 report
//! (§10.4; `crate::source`, `crate::dark2`); and slice R3, P, F and check
//! 1 (§10.6; `crate::program`). The intervals of every slice print as one
//! list, by stratum, then by bytes (Note §6.2).
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

use crate::address::{self, Address, Tau};
use crate::certificate::{self, Performance};
use crate::dark2;
use crate::facts::Position;
use crate::interval::{self, Interval};
use crate::program;
use crate::record;
use crate::source;

/// The prism commit whose `docs/spec/echo.md` echo implements: `3d00a4e`,
/// version 4, §10 with R2 as built. Full and lowercase (Note §4.2, Q2:
/// prism's repository is SHA-1). Always an earlier commit than the one
/// that sets it (the self-reference rule).
pub const SPEC: &str = "3d00a4ecea49d9908c7c03d6973e6484f03075da";

/// The git tree of `docs/spec/echo-vectors` echo was verified against: the
/// tree at `72d1b88`, the R3 red commit that added Taut's 68.
pub const VECTORS: &str = "5bead5f96f6497e5e2d7a1bba62a4f33973ff18c";

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
/// `program`. Each slice removes its word from the front.
const REPLAYED: usize = 2;

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

/// The work meter: counts ground rule instances considered, never time.
struct Meter {
    left: u64,
}

struct Exhausted;

impl Meter {
    fn spend(&mut self) -> Result<(), Exhausted> {
        self.left = self.left.checked_sub(1).ok_or(Exhausted)?;
        Ok(())
    }
}

/// Replay the inputs within `budget` ground rule instances. The binary
/// always passes [`BUDGET`].
pub fn replay(inputs: &Inputs<'_>, budget: u64) -> Answer {
    let heard = [
        inputs.source.map(address::hash),
        inputs.certificate.map(address::hash),
        inputs.record.map(address::hash),
        inputs.score.flatten().map(address::hash),
    ];
    Answer {
        verdict: judge(inputs, budget),
        heard,
    }
}

fn judge(inputs: &Inputs<'_>, budget: u64) -> Verdict {
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
            return Verdict::Unreadable {
                input: Input::Certificate,
                at: r.at,
            }
        }
    };
    let record = match record::read(record) {
        Ok(r) => r,
        Err(r) => {
            return Verdict::Unreadable {
                input: Input::Record,
                at: r.at,
            }
        }
    };
    let mut meter = Meter { left: budget };
    if consider(&performance, &mut meter).is_err() {
        return Verdict::Uncertain { budget };
    }
    // R2: the source and its Dark-2 report. The source is never
    // `unreadable`: a line that doesn't read is a hole (Note §5; Seam row
    // 8).
    let source = source::read(source);
    let report = dark2::report(&source);
    let mut intervals = Vec::new();
    // The first stratum some check couldn't replay, in [`STRATA`]'s order.
    let mut waiting: Option<usize> = None;
    let mut wait = |stratum: usize| waiting = Some(waiting.map_or(stratum, |w| w.min(stratum)));
    match dark2::judge(&report, &performance, &record) {
        Some(found) => intervals.extend(found),
        // Check 2, a model printed for a Dark source: the source stratum's
        // own check, whose bytes are open (Q26).
        None if report.light == dark2::Light::Dark => wait(SOURCE),
        // Light R2 can't tell without a run.
        None => wait(REPLAYED),
    }
    // R3: P and F, check 1. Where the header turns on light R2 can't
    // tell, the run decides.
    match program::expected(&source, &report) {
        Some(expected) => intervals.extend(program::judge(&expected, &performance.header)),
        None => wait(REPLAYED),
    }
    if !intervals.is_empty() {
        // One list across strata: by stratum, then by bytes (Note §6.2).
        interval::sort(&mut intervals);
        return Verdict::Dissonant(intervals);
    }
    conclude(&STRATA[waiting.unwrap_or(REPLAYED)..])
}

/// R1's replay: every ground rule instance the certificate prints (its
/// `derive` lines) is considered once, its head and premises addressed
/// under `H` at the store's `τ`. One unit of budget each.
fn consider(p: &Performance, meter: &mut Meter) -> Result<Vec<Address>, Exhausted> {
    let tau = Tau::pact_fact();
    let mut addresses = Vec::new();
    for d in &p.derives {
        meter.spend()?;
        addresses.push(address::atom(&tau, &d.head));
        addresses.extend(d.premises.iter().map(|a| address::atom(&tau, a)));
    }
    Ok(addresses)
}
