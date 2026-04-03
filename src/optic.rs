//! The five optic operations. The parser's vocabulary. The runtime's interface.
//!
//! fold | prism | traversal | lens | iso
//!
//! Everything else is a composition of these five.

use crate::beam::Beam;
use crate::precision::Precision;

/// The five optic operations as a trait.
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

/// Compose the full pipeline: fold → prism → traversal → lens → (maybe iso).
///
/// The compiler IS this composition. The Beam flows through each step.
/// Loss accumulates. Precision narrows. If convergence is reached, iso.
pub fn compile<P: Prism>(
    optic: &P,
    input: &P::Input,
    precision: Precision,
    transform: &dyn Fn(P::Projection) -> P::Projection,
) -> Beam<P::Projection> {
    // 1. Fold: decompose
    let eigenvalues = optic.fold(input);

    // 2. Prism: project with precision
    let projection = optic.prism(&eigenvalues.result, precision);

    // 3. Lens: transform
    let transformed = optic.lens(projection, transform);

    transformed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loss::ShannonLoss;
    use crate::oid::Oid;

    // -- Test implementation --

    /// A trivial Prism that operates on strings.
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

        fn prism(
            &self,
            eigenvalues: &Vec<char>,
            precision: Precision,
        ) -> Beam<String> {
            // Keep only chars above the precision threshold (by index)
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
                .map(|(i, c)| {
                    Beam::new(c).with_step(Oid::new(format!("{}", i)))
                })
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
        assert_eq!(beam.precision.as_f64(), 0.6);
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
        assert_eq!(beams[1].result, 'b');
        assert_eq!(beams[2].result, 'c');
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
        let beam = Beam::new("settled".to_string());
        let crystal = p.iso(beam);
        assert_eq!(crystal, "settled");
    }

    #[test]
    fn compile_full_pipeline() {
        let p = StringPrism;
        let input = "hello world".to_string();
        let beam = compile(&p, &input, Precision::new(0.5), &|s| s.to_uppercase());
        // fold: 11 chars. prism at 0.5: keep 5. lens: uppercase.
        assert_eq!(beam.result, "HELLO");
        assert!(beam.has_loss()); // lost 6 chars
    }

    #[test]
    fn compile_full_precision() {
        let p = StringPrism;
        let input = "hi".to_string();
        let beam = compile(&p, &input, Precision::new(1.0), &|s| s.to_uppercase());
        assert_eq!(beam.result, "HI");
        assert!(beam.is_lossless());
    }
}
