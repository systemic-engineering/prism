//! A settled item: canonical bytes under their address (docs/spec/laws.md).
//!
//! It was `prismqueer::brain::Crystal`, and `brain` re-exports it there:
//! `crossing` replays a law over crystals, so the floor holds the type.

use std::sync::Arc;

use crate::oid::{Oid, Scope};

/// A settled item: canonical bytes under their address. Immutable.
///
/// [`Crystal::settle`] is the only constructor, and it computes the address
/// from the bytes, so no crystal's address can disagree with its bytes, and
/// nothing lends its bytes mutably.
#[derive(Clone, PartialEq, Eq)]
pub struct Crystal {
    address: Oid,
    bytes: Arc<[u8]>,
}

impl Crystal {
    /// Settle `bytes` in `scope`: the address is `Oid::hash(scope, bytes)`.
    pub fn settle(scope: &Scope, bytes: Vec<u8>) -> Self {
        Crystal {
            address: Oid::hash(scope, &bytes),
            bytes: bytes.into(),
        }
    }

    /// The crystal's address.
    pub fn address(&self) -> &Oid {
        &self.address
    }

    /// The crystal's canonical bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The crystal's canonical bytes, shared: another handle on the same
    /// immutable allocation, never a copy (a store keeps these).
    pub fn shared_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }
}

impl std::fmt::Debug for Crystal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Crystal({:?}, {} B)", self.address, self.bytes.len())
    }
}
