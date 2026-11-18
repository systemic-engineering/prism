//! The certificate's printed form (mirror certificate §3.2). RED STUB.

use crate::address::Address;
use crate::facts::Refusal;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Performance {}

impl Performance {
    pub fn model(&self) -> Option<Address> {
        None
    }
}

pub fn read(_input: &[u8]) -> Result<Performance, Refusal> {
    Ok(Performance {})
}
