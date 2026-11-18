//! The interval: one located dissonance between the report and the source
//! (Note v3.3.2 §6.2).
//!
//! ```text
//! interval <stratum> <kind> <left> <right> about <about> at <run> <tick> by <distance>
//! ```
//!
//! Bytes: an atom at `τ(@song/interval)` with the root flag, eight typed
//! arguments `(sym stratum, sym kind, left, right, sym about, sym run, nat
//! tick, nat distance)` (Taut P1; vectors 45–54, 66, 76). `left` and
//! `right` are an `addr` or the `sym` `none`; addresses print as 64
//! lowercase hex (O5). The tune's lines are sorted by stratum, in its
//! enumeration's order, then by the printed line's bytes; `dissonant <n>`
//! counts lines (§6.2).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::address::Address;
use crate::value::{Atom, Value};

/// The strata, in the order lines are printed (§6.2: the tune's, then the
/// Dark-2 enumeration, then the piece's).
pub const STRATA: [&str; 7] = [
    "source",
    "structure",
    "verdict",
    "grammar",
    "reader",
    "engine",
    "piece",
];

/// One side of an interval: an address, or `none`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    None,
    Addr(Address),
}

impl Side {
    fn value(&self) -> Value {
        match self {
            Side::None => Value::Sym("none".into()),
            Side::Addr(a) => Value::Addr(*a),
        }
    }

    fn print(&self, out: &mut String) {
        match self {
            Side::None => out.push_str("none"),
            Side::Addr(a) => {
                for b in a {
                    let _ = write!(out, "{b:02x}");
                }
            }
        }
    }
}

/// One interval. R2's kinds belong to no run: beat `none 0`, distance 1
/// (H7, V4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interval {
    pub stratum: &'static str,
    pub kind: &'static str,
    pub left: Side,
    pub right: Side,
    pub about: &'static str,
    pub run: &'static str,
    pub tick: u64,
    pub distance: u64,
}

impl Interval {
    /// A `verdict`-stratum interval with no run (`at none 0 by 1`).
    pub fn verdict(kind: &'static str, left: Side, right: Side, about: &'static str) -> Self {
        Interval {
            stratum: "verdict",
            kind,
            left,
            right,
            about,
            run: "none",
            tick: 0,
            distance: 1,
        }
    }

    /// Check 1's interval (Note §6.2): stratum and kind `source`, the
    /// claimed address left and the recomputed one right, about `none`, no
    /// run (`at none 0`), distance 1 (a hash has no metric).
    pub fn source(left: Side, right: Side) -> Self {
        Interval {
            stratum: "source",
            kind: "source",
            left,
            right,
            about: "none",
            run: "none",
            tick: 0,
            distance: 1,
        }
    }

    /// A `structure`-stratum interval of the positive run (Note §6.2:
    /// `stray`, `unlayered`, `depth`, `missing`, `extra`): about `none`, at
    /// `tick <tick>`.
    pub fn structure(
        kind: &'static str,
        left: Side,
        right: Side,
        tick: u64,
        distance: u64,
    ) -> Self {
        Interval {
            stratum: "structure",
            kind,
            left,
            right,
            about: "none",
            run: "tick",
            tick,
            distance,
        }
    }

    /// The printed line, without its LF.
    pub fn text(&self) -> String {
        let mut out = format!("interval {} {} ", self.stratum, self.kind);
        self.left.print(&mut out);
        out.push(' ');
        self.right.print(&mut out);
        let _ = write!(
            out,
            " about {} at {} {} by {}",
            self.about, self.run, self.tick, self.distance
        );
        out
    }

    /// The interval atom, hashed at `@song/interval` root.
    pub fn atom(&self) -> Atom {
        let sym = |s: &str| Value::Sym(s.into());
        Atom {
            pred: "interval".into(),
            args: Vec::from([
                sym(self.stratum),
                sym(self.kind),
                self.left.value(),
                self.right.value(),
                sym(self.about),
                sym(self.run),
                Value::Nat(self.tick),
                Value::Nat(self.distance),
            ]),
        }
    }

    fn rank(&self) -> usize {
        STRATA
            .iter()
            .position(|s| *s == self.stratum)
            .unwrap_or(STRATA.len())
    }
}

/// Sort lines as the tune prints them: by stratum, then by line bytes.
pub fn sort(intervals: &mut [Interval]) {
    intervals.sort_by_cached_key(|i| (i.rank(), i.text()));
}
