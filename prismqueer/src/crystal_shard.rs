//! `prismqueer::crystal_shard` — Crystal<T> alias for settled Shard<T> per Alex Move 5+16.
//!
//! Reed TICK B step 4 per Alex 2026-09-04 PM Move 5 (mechanical-loop LANDED at rust/fractal;
//! this is minimum-viable Crystal-as-settled-Shard at prismqueer altitude; full rust/fractal
//! move to prismqueer::fractal FORWARD-PROMISED).
//!
//! Crystal IS a Shard whose Mandelbrot iteration bounded (per Reed 2026-07-20 rust/fractal/
//! mandelbrot.rs docblock "bounded orbit → Crystal<T> (settled; content-addressed; SAGA-
//! replayable; immutable)"). At prismqueer altitude, Crystal<T> = Shard<T> where
//! `transparency == Transparency::Clear` OR `iteration_index >= convergence_threshold`.
//!
//! Full Crystal<T> struct with separate identity (immutability guarantee; SAGA-replay
//! anchor; git-projection persistence) FORWARD-PROMISED at Move 5 fractal-move-to-prismqueer
//! tick.

use crate::shard::Shard;

/// Crystal<T> = settled Shard<T> at prismqueer altitude per Alex Move 5+16.
///
/// Type alias for now; full struct with immutability + SAGA-replay + content-addressed
/// git-projection FORWARD-PROMISED at Move 5 rust/fractal → prismqueer::fractal move tick.
pub type Crystal<T> = Shard<T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crystal_is_shard_type_alias_composes() {
        let crystal: Crystal<&[u8]> = Crystal::new(b"settled");
        assert_eq!(crystal.iteration_index, 0);
    }
}
