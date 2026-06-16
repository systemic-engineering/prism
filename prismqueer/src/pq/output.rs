//! `pq::Output` — the settle DSL.
//!
//! Per pq spec §5.3, `Output` is a discriminated union over the
//! settle step's settle target. Each variant is a distinct JSON
//! key-shape.

use serde::{Deserialize, Serialize};

use super::Reference;
use crate::Oid;

/// CAS-safe ref update payload — `{"old": "...", "new": "..."}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CasUpdate {
    /// The currently-observed Oid (the CAS guard).
    pub old: Oid,
    /// The Oid to write if the guard matches.
    pub new: Oid,
}

/// Settle DSL. Each variant maps to a distinct JSON key-shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Output {
    /// `{"to_path": "...", "message"?: "..."}` — commit to a path.
    /// `message` is omitted on the wire when `None`.
    ToPath {
        /// The path to commit to.
        to_path: String,
        /// An optional commit message.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        message: Option<String>,
    },
    /// `{"ref": "..."}` — advance a ref (branch creation / update).
    Ref {
        /// The ref to advance.
        #[serde(rename = "ref")]
        ref_: Reference,
    },
    /// `{"cas": {"old": "...", "new": "..."}}` — CAS-safe ref update.
    Cas {
        /// The compare-and-set payload.
        cas: CasUpdate,
    },
    /// `{"to_ref": "..."}` — write merged result to a ref.
    ToRef {
        /// The ref to write to.
        to_ref: Reference,
    },
    /// `{"snapshot": true}` — crystallize without committing.
    Snapshot {
        /// Always `true` on the wire — the variant tag IS the action.
        snapshot: bool,
    },
    /// `{"flush": true}` — shard flush to disk.
    Flush {
        /// Always `true` on the wire — the variant tag IS the action.
        flush: bool,
    },
}
