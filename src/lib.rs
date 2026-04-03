//! Prism — fold | prism | traversal | lens | iso.
//!
//! Five operations. The optic hierarchy as a trait.
//! The parser's vocabulary. The runtime's interface.
//! Everything else is a composition of these five.
//!
//! Beam<T>, Oid, ShannonLoss, Precision, Pressure, Recovery, ContentAddressed.
//! Zero dependencies. The types that outlive everything around them.
//!
//! A prism splits light into beams.

pub mod beam;
pub mod content;
pub mod loss;
pub mod oid;
pub mod precision;

pub use beam::{Beam, Recovery};
pub use content::ContentAddressed;
pub use loss::ShannonLoss;
pub use oid::Oid;
pub use precision::{Precision, Pressure};

// ---------------------------------------------------------------------------
// The Prism trait — the five optic operations
// ---------------------------------------------------------------------------

/// The five optic operations.
///
/// Implementors provide concrete types for each stage.
/// The Beam flows through, accumulating path, loss, precision.
pub trait Prism {
    /// The input to decompose (source text, graph, domain).
    type Input;
    /// The eigenvalue decomposition of the input.
    type Eigenvalues;
    /// What survives the precision cut.
    type Projection;
    /// A node visited during traversal.
    type Node;
    /// Evidence that the system has settled.
    type Convergence;
    /// The immutable fixed point. The Fortran vector.
    type Crystal;

    /// Fold: structure → eigenvalues. Read-only. Accumulate.
    /// The decomposition. Observe without modifying.
    fn fold(&self, input: &Self::Input) -> Beam<Self::Eigenvalues>;

    /// Prism: eigenvalues → projection. Partial. Precision-bounded.
    /// The boundary test. Returns what survives the cut.
    fn prism(
        &self,
        eigenvalues: &Self::Eigenvalues,
        precision: Precision,
    ) -> Beam<Self::Projection>;

    /// Traversal: walk the projection. Multiple targets. Accumulate Beams.
    /// The multi-site observation.
    fn traversal(&self, projection: &Self::Projection) -> Vec<Beam<Self::Node>>;

    /// Lens: focus, transform, put back. The recursive step.
    /// Apply a function to the projection. Return the modified Beam.
    fn lens(
        &self,
        beam: Beam<Self::Projection>,
        f: &dyn Fn(Self::Projection) -> Self::Projection,
    ) -> Beam<Self::Projection>;

    /// Iso: the fixed point. Lossless. Invertible. Immutable.
    /// The crystal. Only callable when convergence is proven.
    fn iso(&self, beam: Beam<Self::Convergence>) -> Self::Crystal;
}

/// Apply a Prism: fold → prism → lens.
///
/// The Beam flows through each step.
/// Loss accumulates. Precision narrows.
pub fn apply<P: Prism>(
    optic: &P,
    input: &P::Input,
    precision: Precision,
    transform: &dyn Fn(P::Projection) -> P::Projection,
) -> Beam<P::Projection> {
    let eigenvalues = optic.fold(input);
    let projection = optic.prism(&eigenvalues.result, precision);
    optic.lens(projection, transform)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct StringPrism;

    impl Prism for StringPrism {
        type Input = String;
        type Eigenvalues = Vec<char>;
        type Projection = String;
        type Node = char;
        type Convergence = String;
        type Crystal = String;

        fn fold(&self, input: &String) -> Beam<Vec<char>> {
            Beam::new(input.chars().collect())
        }

        fn prism(&self, eigenvalues: &Vec<char>, precision: Precision) -> Beam<String> {
            let cutoff = (eigenvalues.len() as f64 * precision.as_f64()) as usize;
            let kept: String = eigenvalues[..cutoff.min(eigenvalues.len())]
                .iter()
                .collect();
            let lost = eigenvalues.len() - kept.len();
            Beam::new(kept)
                .with_loss(ShannonLoss::new(lost as f64))
                .with_precision(precision)
        }

        fn traversal(&self, projection: &String) -> Vec<Beam<char>> {
            projection
                .chars()
                .enumerate()
                .map(|(i, c)| Beam::new(c).with_step(Oid::new(format!("{}", i))))
                .collect()
        }

        fn lens(
            &self,
            beam: Beam<String>,
            f: &dyn Fn(String) -> String,
        ) -> Beam<String> {
            beam.map(f)
        }

        fn iso(&self, beam: Beam<String>) -> String {
            beam.result
        }
    }

    #[test]
    fn fold_decomposes() {
        let p = StringPrism;
        let beam = p.fold(&"hello".to_string());
        assert_eq!(beam.result, vec!['h', 'e', 'l', 'l', 'o']);
        assert!(beam.is_lossless());
    }

    #[test]
    fn prism_projects_with_precision() {
        let p = StringPrism;
        let eigenvalues = vec!['h', 'e', 'l', 'l', 'o'];
        let beam = p.prism(&eigenvalues, Precision::new(0.6));
        assert_eq!(beam.result, "hel");
        assert!(beam.has_loss());
    }

    #[test]
    fn prism_full_precision_is_lossless() {
        let p = StringPrism;
        let eigenvalues = vec!['h', 'i'];
        let beam = p.prism(&eigenvalues, Precision::new(1.0));
        assert_eq!(beam.result, "hi");
        assert!(beam.is_lossless());
    }

    #[test]
    fn traversal_walks_nodes() {
        let p = StringPrism;
        let beams = p.traversal(&"abc".to_string());
        assert_eq!(beams.len(), 3);
        assert_eq!(beams[0].result, 'a');
        assert_eq!(beams[0].path[0].as_str(), "0");
    }

    #[test]
    fn lens_transforms() {
        let p = StringPrism;
        let beam = Beam::new("hello".to_string());
        let transformed = p.lens(beam, &|s| s.to_uppercase());
        assert_eq!(transformed.result, "HELLO");
    }

    #[test]
    fn lens_preserves_trace() {
        let p = StringPrism;
        let beam = Beam::new("x".to_string())
            .with_step(Oid::new("origin"))
            .with_loss(ShannonLoss::new(0.5));
        let transformed = p.lens(beam, &|s| s.to_uppercase());
        assert_eq!(transformed.result, "X");
        assert_eq!(transformed.path[0].as_str(), "origin");
        assert!(transformed.has_loss());
    }

    #[test]
    fn iso_crystallizes() {
        let p = StringPrism;
        let crystal = p.iso(Beam::new("settled".to_string()));
        assert_eq!(crystal, "settled");
    }

    #[test]
    fn apply_full_pipeline() {
        let p = StringPrism;
        let beam = apply(&p, &"hello world".to_string(), Precision::new(0.5), &|s| {
            s.to_uppercase()
        });
        assert_eq!(beam.result, "HELLO");
        assert!(beam.has_loss());
    }

    #[test]
    fn apply_full_precision() {
        let p = StringPrism;
        let beam = apply(&p, &"hi".to_string(), Precision::new(1.0), &|s| {
            s.to_uppercase()
        });
        assert_eq!(beam.result, "HI");
        assert!(beam.is_lossless());
    }
}
