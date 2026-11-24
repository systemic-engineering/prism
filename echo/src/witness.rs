//! The witness (docs/spec/echo.md §11; prism-025 Addendum item 3).
//!
//! Witnessed = address + key + the reader's pin + an independent
//! re-derivation (Seam SF §5) + **coverage**: the reader's verdict ranges
//! only over the bytes it read, and everything else is not vouched for
//! (Mara 052e5a13 §1.1). The answers are the witness's {accept, refuse,
//! not now}, never the roll's {true, maybe, false} (§1.2).
//!
//! The record is computed after the verdict, from the answer and a
//! [`Trace`] of the replay. Nothing in it enters the verdict, an interval,
//! `heard`, or any address (mirror-029 §6: the witness sits beside the
//! roll, never in its address). It prints after the seam `---`, below the
//! parity lines, which stay byte for byte (§11.4). The format is
//! provisional.
//!
//! A witness's answer is not a roll value. Neither converts to the other:
//!
//! ```compile_fail,E0308
//! use echo::witness::{Response, Roll};
//! fn settle(_: Roll) {}
//! settle(Response::Accept);
//! ```
//!
//! The control: a roll value where a roll value is expected, and an answer
//! where an answer is.
//!
//! ```
//! use echo::witness::{Response, Roll};
//! fn settle(_: Roll) {}
//! fn hear(_: Response) {}
//! settle(Roll::Maybe);
//! hear(Response::Accept);
//! ```

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::address::Address;
use crate::facts::Position;
use crate::replay::{
    self, Answer, Input, Inputs, Outcome, Reading, Trace, Undecided, Verdict, REPLAYED, SPEC,
    STRATA, VECTORS,
};

/// The roll's value, as `settle <addr> true | maybe | false` prints it. A
/// value of the world: a WFS `maybe` is stable and certified (Mara §1.2).
/// echo reads no roll value yet (the record's values are R6's); the type
/// is here so the two ternaries are two types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Roll {
    True,
    Maybe,
    False,
}

/// Why the answer is `not now`: one answer, several reasons, each the
/// verdict's own words (§11.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// `uncertain budget <n> ground rule instances`.
    Budget(u64),
    /// `unknown io <input> …`.
    Io(Vec<Input>),
    /// `unknown unreadable <input>:<L>:<C>`.
    Unreadable { input: Input, at: Position },
    /// `unknown unreplayed <stratum>`.
    Unreplayed(&'static str),
    /// `rings` with a stratum left uncovered: the floor of echo.
    Uncovered,
}

/// The witness's answer: {accept, refuse, not now}.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Response {
    Accept,
    Refuse,
    NotNow(Reason),
}

impl Response {
    /// The answer's words, as the record's `answer` line prints them.
    pub fn text(&self) -> String {
        match self {
            Response::Accept => String::from("accept"),
            Response::Refuse => String::from("refuse"),
            Response::NotNow(reason) => {
                let why = match reason {
                    Reason::Budget(n) => format!("budget {n} ground rule instances"),
                    Reason::Io(inputs) => {
                        let mut s = String::from("io");
                        for i in inputs {
                            s.push(' ');
                            s.push_str(i.name());
                        }
                        s
                    }
                    Reason::Unreadable { input, at } => {
                        format!("unreadable {}:{at}", input.name())
                    }
                    Reason::Unreplayed(stratum) => format!("unreplayed {stratum}"),
                    Reason::Uncovered => String::from("uncovered"),
                };
                format!("not now {why}")
            }
        }
    }
}

/// The answer for a verdict. `accept` only from `rings` with every
/// stratum `covered`: echo never accepts what it didn't replay.
pub fn respond(verdict: &Verdict, covered: bool) -> Response {
    match verdict {
        Verdict::Rings if covered => Response::Accept,
        Verdict::Rings => Response::NotNow(Reason::Uncovered),
        Verdict::Dissonant(_) => Response::Refuse,
        Verdict::Uncertain { budget } => Response::NotNow(Reason::Budget(*budget)),
        Verdict::Io(inputs) => Response::NotNow(Reason::Io(inputs.clone())),
        Verdict::Unreadable { input, at } => Response::NotNow(Reason::Unreadable {
            input: *input,
            at: *at,
        }),
        Verdict::Unreplayed(stratum) => Response::NotNow(Reason::Unreplayed(stratum)),
    }
}

/// The witness record of one replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness {
    pub response: Response,
    /// The replay's `heard`: the address and the inputs' coverage.
    pub heard: [Option<Address>; 4],
    pub trace: Trace,
}

/// Replay, and witness the replay. The answer is [`replay::replay`]'s.
pub fn witness(inputs: &Inputs<'_>, budget: u64) -> (Answer, Witness) {
    let (answer, trace) = replay::traced(inputs, budget);
    let covered = REPLAYED == STRATA.len()
        && trace
            .strata
            .iter()
            .all(|s| s.is_some_and(|o| o.undecided.is_none()));
    let witness = Witness {
        response: respond(&answer.verdict, covered),
        heard: answer.heard,
        trace,
    };
    (answer, witness)
}

fn addr(out: &mut String, a: Option<&Address>) {
    match a {
        Some(a) => {
            for b in a {
                let _ = write!(out, "{b:02x}");
            }
        }
        None => out.push_str("none"),
    }
}

fn undecided(u: Undecided) -> &'static str {
    match u {
        Undecided::Waits => "waits",
        Undecided::Open => "open",
    }
}

fn outcome(out: &mut String, o: &Outcome) {
    match (o.intervals, o.undecided) {
        (0, None) => out.push_str("agreed"),
        (0, Some(u)) => out.push_str(undecided(u)),
        (n, None) => {
            let _ = write!(out, "dissonant {n}");
        }
        (n, Some(u)) => {
            let _ = write!(out, "dissonant {n} {}", undecided(u));
        }
    }
}

const INPUTS: [Input; 4] = [
    Input::Source,
    Input::Certificate,
    Input::Record,
    Input::Score,
];

impl Witness {
    /// The record (§11.4): the seam, then one line each, each ending in LF.
    pub fn print(&self) -> String {
        let mut out = String::from("---\nwitness 1\nanswer ");
        out.push_str(&self.response.text());
        out.push_str("\naddress certificate ");
        addr(&mut out, self.heard[1].as_ref());
        let (p, f) = match &self.trace.key {
            Some((p, f)) => (Some(p), Some(f)),
            None => (None, None),
        };
        out.push_str("\nkey program ");
        addr(&mut out, p);
        out.push_str(" facts ");
        addr(&mut out, f);
        let _ = write!(
            out,
            " scope none\npin spec {SPEC} vectors {VECTORS} rig none\n"
        );
        for (stratum, o) in STRATA.iter().zip(&self.trace.strata) {
            if let Some(o) = o {
                let _ = write!(out, "derived {stratum} ");
                outcome(&mut out, o);
                out.push('\n');
            }
        }
        for (i, (input, reading)) in INPUTS.iter().zip(&self.trace.inputs).enumerate() {
            if *reading == Reading::Read {
                let _ = write!(out, "covered input {} ", input.name());
                addr(&mut out, self.heard[i].as_ref());
                out.push('\n');
            }
        }
        for (i, (input, reading)) in INPUTS.iter().zip(&self.trace.inputs).enumerate() {
            let why = match reading {
                Reading::Read => continue,
                Reading::Stopped => String::from("stopped"),
                Reading::Io => String::from("io"),
                Reading::Unreadable(at) => format!("unreadable {at}"),
                Reading::Unread => String::from("unread"),
                Reading::Absent => String::from("absent"),
            };
            let _ = write!(out, "uncovered input {} ", input.name());
            addr(&mut out, self.heard[i].as_ref());
            let _ = writeln!(out, " {why}");
        }
        for (stratum, o) in STRATA.iter().zip(&self.trace.strata) {
            if o.is_some_and(|o| o.undecided.is_none()) {
                let _ = writeln!(out, "covered stratum {stratum}");
            }
        }
        for (n, stratum) in STRATA.iter().enumerate() {
            let why = match self.trace.strata.get(n) {
                None => "unreplayed",
                Some(None) => "stopped",
                Some(Some(Outcome {
                    undecided: Some(u), ..
                })) => undecided(*u),
                Some(Some(_)) => continue,
            };
            let _ = writeln!(out, "uncovered stratum {stratum} {why}");
        }
        out.push_str("uncovered rest\n");
        out
    }
}
