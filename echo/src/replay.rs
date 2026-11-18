//! Certificate replay (prism-022 S1, S2), slice R1: the surface
//! (docs/spec/echo.md §10.1).
//!
//! The output is Note v3.3.2 §6.3's, as mirror's v3.2 contract delta
//! (`docs/pin-batch/2026-10-30-tune-exit-contract-v3.2.md`) gives it: the
//! verdict line, then `heard` (§4.2, B8), then `implements`, every line
//! ending in LF. The exits are Alex's: 0 rings, 1 dissonant or uncertain,
//! 2 unknown.
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
use crate::facts::Position;
use crate::record;

/// The prism commit whose `docs/spec/echo.md` echo implements: §10, the
/// decomposition and R1's exact surface. Full and lowercase (Note §4.2,
/// Q2: prism's repository is SHA-1). Always an earlier commit than the one
/// that sets it (the self-reference rule).
pub const SPEC: &str = "7a6dd355564cffaf6c47bb4f650390a9725352f4";

/// The git tree of `docs/spec/echo-vectors` echo was verified against: the
/// tree at `73df7f0`, the R1 red commit that added mirror's fixtures.
pub const VECTORS: &str = "ce646bf56a9f00a9309522d3178e4de4ca482060";

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

/// What echo replays, of [`STRATA`], so far: nothing. Each slice from R2
/// on removes its word from the front.
const UNREPLAYED: [&str; 8] = STRATA;

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
    /// `rings`: every stratum replayed and agreed. Not reachable in R1.
    Rings,
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
            Verdict::Uncertain { .. } => 1,
            Verdict::Io(_) | Verdict::Unreadable { .. } | Verdict::Unreplayed(_) => 2,
        }
    }

    /// The answer's lines, each ending in LF.
    pub fn print(&self) -> String {
        let mut out = match &self.verdict {
            Verdict::Rings => String::from("rings"),
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
    // The source is never `unreadable`: a source that doesn't read is a
    // Dark-2 source, judged from R2 on (Note §5; Seam row 8).
    let _ = source;
    let performance = match certificate::read(certificate) {
        Ok(p) => p,
        Err(r) => {
            return Verdict::Unreadable {
                input: Input::Certificate,
                at: r.at,
            }
        }
    };
    if let Err(r) = record::read(record) {
        return Verdict::Unreadable {
            input: Input::Record,
            at: r.at,
        };
    }
    let mut meter = Meter { left: budget };
    if consider(&performance, &mut meter).is_err() {
        return Verdict::Uncertain { budget };
    }
    conclude(&UNREPLAYED)
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
