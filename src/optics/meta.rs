//! MetaPrism — operates on populations of beams.
//!
//! A base prism's `split` produces `Vec<Beam<Part>>`. To work with that
//! population as a unit, you wrap it in a MetaPrism parameterized by a
//! Gather strategy. The MetaPrism's refract collapses the population
//! back into a single Beam using the strategy.
//!
//! This is where the inter-level movement happens: base prisms live at
//! level 0 (`Beam<T>`), meta prisms live at level 1 (`Vec<Beam<T>>`).

use crate::{Beam, Prism, Stage};
use super::gather::{Gather, SumGather};

// Types go here.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Precision;

    #[test]
    fn meta_prism_project_gathers_to_single_beam() {
        let meta: MetaPrism<String, SumGather> = MetaPrism::new(SumGather);
        let population = vec![
            Beam::new("foo".to_string()),
            Beam::new("bar".to_string()),
            Beam::new("baz".to_string()),
        ];
        let input = Beam::new(population);
        let focused = meta.focus(input);
        let projected = meta.project(focused);
        assert_eq!(projected.result, "foobarbaz");
        assert_eq!(projected.stage, Stage::Projected);
    }

    #[test]
    fn meta_prism_full_pipeline_ends_at_refracted() {
        let meta: MetaPrism<String, SumGather> = MetaPrism::new(SumGather);
        let population = vec![
            Beam::new("a".to_string()),
            Beam::new("b".to_string()),
        ];
        let out = crate::apply(&meta, population);
        assert_eq!(out.stage, Stage::Refracted);
    }

    #[test]
    fn meta_prism_crystal_is_self() {
        fn require_prism<P: Prism>() {}
        require_prism::<MetaPrism<String, SumGather>>();
    }
}
