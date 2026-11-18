//! Certificate replay, slice R3: P, F and check 1 (docs/spec/echo.md
//! §10.6). Red: the surface only; every answer is "can't tell".

use alloc::string::String;
use alloc::vec::Vec;

use crate::address::Address;
use crate::certificate::Header;
use crate::dark2::Report;
use crate::interval::Interval;
use crate::source::{Record, Source};

/// The header's `program` and `facts` the source calls for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    pub program: Option<Address>,
    pub facts: Option<Address>,
}

/// A record's canonical line; `None` for a fact.
pub fn canonical(_record: &Record) -> Option<String> {
    None
}

/// P.
pub fn program(_source: &Source) -> Address {
    [0; 32]
}

/// F.
pub fn facts(_source: &Source) -> Address {
    [0; 32]
}

/// The header the source calls for; `None` where R3 can't tell.
pub fn expected(_source: &Source, _report: &Report) -> Option<Expected> {
    None
}

/// Check 1.
pub fn judge(_expected: &Expected, _header: &Header) -> Vec<Interval> {
    Vec::new()
}
