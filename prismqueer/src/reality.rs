//! `prismqueer::reality` — Reality sum-type per Alex Move 16.
//!
//! Reed TICK B step 8 per Alex 2026-09-04 PM Move 16 verbatim ("What if reality is a
//! sum-type? Fractured - disconnected Shards / Flux - liquid Shards / Settled - crystallized
//! Shards") + Move 17 ("@reality/hodobody is Flux<Reality>").
//!
//! # Alex Move 16+17 sum-type at STATE altitude
//!
//! ```text
//! Reality = Fractured(Vec<Shard>) | Settled(Crystal<Shard>)
//! Flux<Reality> = externally-wrapped in-motion state (hodobodo reading-label)
//! ```
//!
//! Two-altitude clean partition: STATE (this Move 16 sum-type) × TRAJECTORY
//! (Move 10 Object/Subject applies to Settled only). Hodobodo dissolves at species-mint
//! altitude AND reframes as `Flux<Reality>` at type-composition altitude per Move 17.
//!
//! # Composition
//!
//! - `prismqueer::shard::Shard<T>` (Discharge #5 `22723bb`)
//! - `prismqueer::crystal_shard::Crystal<T>` (Discharge #8)

use crate::crystal_shard::Crystal;
use crate::shard::Shard;

/// Reality sum-type at STATE altitude per Alex Move 16.
///
/// `Fractured(Vec<Shard>)` = disconnected components (H^0 > 1)
/// `Settled(Crystal<Shard>)` = crystallized content-addressed immutable
///
/// `Flux<Reality>` per Move 17 is externally-wrapped in-motion state; carried
/// by `prismqueer::flux::Flux` (LANDED Discharge #4a `ec25c19`). Hodobodo IS
/// `Flux<Reality>` at type-composition altitude per Move 17 (no separate variant).
#[derive(Clone, Debug)]
pub enum Reality<T> {
    /// Disconnected Shards; H^0 > 1; parts not yet composable.
    Fractured(Vec<Shard<T>>),
    /// Crystallized settled Shard; content-addressed via Oid.
    Settled(Crystal<T>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reality_fractured_composes_disconnected_shards() {
        let shards: Vec<Shard<&[u8]>> = vec![Shard::new(b"a"), Shard::new(b"b")];
        let reality: Reality<&[u8]> = Reality::Fractured(shards);
        match reality {
            Reality::Fractured(v) => assert_eq!(v.len(), 2),
            _ => panic!("expected Fractured"),
        }
    }

    #[test]
    fn reality_settled_composes_crystal() {
        let reality: Reality<&[u8]> = Reality::Settled(Crystal::new(b"c"));
        match reality {
            Reality::Settled(_) => {}
            _ => panic!("expected Settled"),
        }
    }
}
