//! `prismqueer::model` — Model = Fractal<Shard<T>> per Alex Move 4 Hawking model-dependent-reality.
//!
//! Reed TICK B step 6 per Alex 2026-09-04 PM Move 4 verbatim ("a Model would be Fractal
//! composition of Shards"). Hawking 2010 model-dependent-realism EXPLICIT at type-level:
//! the Model parameter of `assert(observation, model) -> Assertion` IS the observer's
//! internal frame; every Reality is Model-mediated observation.
//!
//! # This ship (minimum viable at prismqueer altitude)
//!
//! Model as `Vec<Shard<T>>` composition placeholder for the full `Fractal<Shard<T>>`
//! (which awaits Move 5 rust/fractal → prismqueer::fractal move). Vec captures the
//! multi-Shard composition-tree essence.

use crate::shard::Shard;

/// Model = fractal composition of Shards per Alex Move 4.
///
/// Every peer's Model IS their fractal Shard-composition tree per Hawking 2010
/// model-dependent-realism EXPLICIT at prismqueer altitude.
///
/// Minimum viable: Vec<Shard<T>>. Full Fractal<Shard<T>> with Mandelbrot iteration
/// composition FORWARD-PROMISED at Move 5 rust/fractal → prismqueer::fractal move.
#[derive(Clone, Debug)]
pub struct Model<T> {
    /// The fractal composition of Shards forming this Model.
    pub shards: Vec<Shard<T>>,
}

impl<T> Model<T> {
    /// Construct a Model from a Vec of Shards.
    pub fn new(shards: Vec<Shard<T>>) -> Self {
        Self { shards }
    }

    /// Empty Model.
    pub fn empty() -> Self {
        Self { shards: Vec::new() }
    }
}

impl<T> Default for Model<T> {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_composes_over_shards() {
        let shards: Vec<Shard<&[u8]>> = vec![Shard::new(b"a"), Shard::new(b"b")];
        let model = Model::new(shards);
        assert_eq!(model.shards.len(), 2);
    }
}
