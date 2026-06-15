//! `pq::Target` — the focus DSL.
//!
//! Per pq spec §5.1, `Target` is a discriminated union over the
//! key-shape on the wire. Each variant is a distinct JSON object
//! shape; the empty target `{}` is its own variant.

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use super::Reference;
use crate::Oid;

/// Focus DSL. Each variant maps to a distinct JSON key-shape.
///
/// - `{}`                     → `Target::Empty`
/// - `{"oid": "..."}`         → `Target::Oid`
/// - `{"path": "..."}`        → `Target::Path`
/// - `{"ref": "..."}`         → `Target::Ref`
/// - `{"pair": [..., ...]}`   → `Target::Pair`
/// - `{"refs": true}`         → `Target::Refs`
/// - `{"shard": true}`        → `Target::Shard`
///
/// Cross-variant disambiguation is exact-key. An unrecognised key shape
/// (e.g. `{"unknown": "..."}`) fails deserialisation rather than
/// stringly-coercing into a neighbour — `Empty` accepts only the
/// literal empty object.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Target {
    /// `{"oid": "..."}` — focus by content address.
    Oid { oid: Oid },
    /// `{"path": "..."}` — focus by working-tree path.
    Path { path: String },
    /// `{"ref": "..."}` — focus by named ref. `ref` is a Rust keyword,
    /// hence the trailing-underscore field name plus serde rename.
    Ref {
        #[serde(rename = "ref")]
        ref_: Reference,
    },
    /// `{"pair": [a, b]}` — focus on the pair (diff/merge inputs).
    Pair { pair: Box<[Target; 2]> },
    /// `{"refs": true}` — focus the ref-set.
    Refs { refs: bool },
    /// `{"shard": true}` — focus the shard's summary.
    Shard { shard: bool },
    /// `{}` — the shard's current focus.
    Empty {},
}

// ─── manual Deserialize ─────────────────────────────────────────────────
//
// Serde 1.x does not support `#[serde(deny_unknown_fields)]` on enum
// variants. The default untagged behaviour for struct variants
// silently tolerates extra fields, which would allow `{"unknown":
// "..."}` to coerce into `Target::Empty {}`. The pq wire's no-stringly
// contract (`feedback-no-stringly-types`) forbids that, so we drive
// the dispatch manually through a `Value` shape and pattern-match the
// single canonical key.

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TargetVisitor;

        impl<'de> Visitor<'de> for TargetVisitor {
            type Value = Target;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "a Target object: {} | {oid} | {path} | {ref} | {pair} | {refs} | {shard}",
                )
            }

            fn visit_map<M>(self, mut map: M) -> Result<Target, M::Error>
            where
                M: MapAccess<'de>,
            {
                // Gather all entries first; the dispatch is exact.
                let mut entries: Vec<(String, Value)> = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, Value>()? {
                    entries.push((k, v));
                }

                if entries.is_empty() {
                    return Ok(Target::Empty {});
                }
                if entries.len() != 1 {
                    return Err(de::Error::custom(format!(
                        "Target object must have exactly one key, got {}",
                        entries.len()
                    )));
                }
                let (key, value) = entries.into_iter().next().unwrap();
                match key.as_str() {
                    "oid" => Ok(Target::Oid {
                        oid: serde_json::from_value(value).map_err(de::Error::custom)?,
                    }),
                    "path" => Ok(Target::Path {
                        path: serde_json::from_value(value).map_err(de::Error::custom)?,
                    }),
                    "ref" => Ok(Target::Ref {
                        ref_: serde_json::from_value(value).map_err(de::Error::custom)?,
                    }),
                    "pair" => Ok(Target::Pair {
                        pair: serde_json::from_value(value).map_err(de::Error::custom)?,
                    }),
                    "refs" => Ok(Target::Refs {
                        refs: serde_json::from_value(value).map_err(de::Error::custom)?,
                    }),
                    "shard" => Ok(Target::Shard {
                        shard: serde_json::from_value(value).map_err(de::Error::custom)?,
                    }),
                    other => Err(de::Error::custom(format!(
                        "unknown Target key: `{}`",
                        other
                    ))),
                }
            }
        }

        deserializer.deserialize_map(TargetVisitor)
    }
}
