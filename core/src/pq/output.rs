//! `pq::Output` — the refract DSL.
//!
//! Per pq spec §5.3, `Output` is a discriminated union over the
//! refract step's settle target. Each variant is a distinct JSON
//! key-shape.

use serde::{Deserialize, Serialize};

use super::Reference;
use crate::Oid;

/// CAS-safe ref update payload — `{"old": "...", "new": "..."}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CasUpdate {
    pub old: Oid,
    pub new: Oid,
}

/// Refract DSL. Each variant maps to a distinct JSON key-shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Output {
    /// `{"to_path": "...", "message"?: "..."}` — commit to a path.
    /// `message` is omitted on the wire when `None`.
    ToPath {
        to_path: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        message: Option<String>,
    },
    /// `{"ref": "..."}` — advance a ref (branch creation / update).
    Ref {
        #[serde(rename = "ref")]
        ref_: Reference,
    },
    /// `{"cas": {"old": "...", "new": "..."}}` — CAS-safe ref update.
    Cas { cas: CasUpdate },
    /// `{"to_ref": "..."}` — write merged result to a ref.
    ToRef { to_ref: Reference },
    /// `{"snapshot": true}` — crystallize without committing.
    Snapshot { snapshot: bool },
    /// `{"flush": true}` — shard flush to disk.
    Flush { flush: bool },
}
