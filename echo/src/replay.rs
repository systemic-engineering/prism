//! Certificate replay (prism-022 S1, S2), slice R1: the surface
//! (docs/spec/echo.md §10.1). RED STUB: answers the same wrong thing for
//! every input.

use alloc::string::String;
use alloc::vec::Vec;

use crate::address::Address;
use crate::facts::Position;

pub const SPEC: &str = "";
pub const VECTORS: &str = "";
pub const BUDGET: u64 = 0;
pub const STRATA: [&str; 8] = [""; 8];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Source,
    Certificate,
    Record,
    Score,
}

pub struct Inputs<'a> {
    pub source: Option<&'a [u8]>,
    pub certificate: Option<&'a [u8]>,
    pub record: Option<&'a [u8]>,
    pub score: Option<Option<&'a [u8]>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Rings,
    Uncertain { budget: u64 },
    Io(Vec<Input>),
    Unreadable { input: Input, at: Position },
    Unreplayed(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub verdict: Verdict,
    pub heard: [Option<Address>; 4],
}

impl Answer {
    pub fn exit(&self) -> u8 {
        0
    }

    pub fn print(&self) -> String {
        String::new()
    }
}

pub fn conclude(_unreplayed: &[&'static str]) -> Verdict {
    Verdict::Unreplayed("")
}

pub fn replay(_inputs: &Inputs<'_>, _budget: u64) -> Answer {
    Answer {
        verdict: Verdict::Unreplayed(""),
        heard: [None; 4],
    }
}
