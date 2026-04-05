//! Prism — focus | project | split | zoom | refract.
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
pub mod spectral_oid;

pub use beam::{Beam, Recovery};
pub use content::ContentAddressed;
pub use loss::ShannonLoss;
pub use oid::Oid;
pub use precision::{Precision, Pressure};
pub use spectral_oid::SpectralOid;

// ---------------------------------------------------------------------------
// The Prism trait — the five optic operations
// ---------------------------------------------------------------------------

/// The five optic operations. The shared grammar of computation.
///
/// Any system that decomposes, projects, walks, transforms, and settles
/// implements Prism. The parser. The compiler. The runtime. The database.
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
    /// A node visited during split.
    type Node;
    /// Evidence that the system has settled.
    type Convergence;
    /// The immutable fixed point. The Fortran vector.
    type Crystal;
    /// The precision type used to bound the project operation.
    /// Implementors set this to the concrete type they accept
    /// (e.g. `type Precision = prism::Precision`).
    type Precision;

    /// Focus: structure → eigenvalues. Read-only. Accumulate.
    /// The decomposition. Observe without modifying.
    fn focus(&self, input: &Self::Input) -> Beam<Self::Eigenvalues>;

    /// Project: eigenvalues → projection. Partial. Precision-bounded.
    /// The boundary test. Returns what survives the cut.
    fn project(
        &self,
        eigenvalues: &Self::Eigenvalues,
        precision: Self::Precision,
    ) -> Beam<Self::Projection>;

    /// Split: walk the projection. Multiple targets. Accumulate Beams.
    /// The multi-site observation.
    fn split(&self, projection: &Self::Projection) -> Vec<Beam<Self::Node>>;

    /// Zoom: focus, transform, put back. The recursive step.
    /// Apply a function to the projection. Return the modified Beam.
    fn zoom(
        &self,
        beam: Beam<Self::Projection>,
        f: &dyn Fn(Self::Projection) -> Self::Projection,
    ) -> Beam<Self::Projection>;

    /// Refract: the fixed point. Lossless. Invertible. Immutable.
    /// The crystal. Only callable when convergence is proven.
    fn refract(&self, beam: Beam<Self::Convergence>) -> Self::Crystal;
}

/// Apply a Prism: focus → project → zoom.
///
/// The composition of the first three operations.
/// Decompose the input, project at the given precision,
/// transform the projection. The Beam carries the result
/// through each step. Loss accumulates. Precision narrows.
///
/// Split and refract are not included — they are called
/// separately when the caller needs to walk or settle.
pub fn apply<P: Prism>(
    optic: &P,
    input: &P::Input,
    precision: P::Precision,
    transform: &dyn Fn(P::Projection) -> P::Projection,
) -> Beam<P::Projection> {
    let eigenvalues = optic.focus(input);
    let projection = optic.project(&eigenvalues.result, precision);
    optic.zoom(projection, transform)
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
        type Precision = Precision;

        fn focus(&self, input: &String) -> Beam<Vec<char>> {
            Beam::new(input.chars().collect())
        }

        fn project(&self, eigenvalues: &Vec<char>, precision: Precision) -> Beam<String> {
            let cutoff = (eigenvalues.len() as f64 * precision.as_f64()) as usize;
            let kept: String = eigenvalues[..cutoff.min(eigenvalues.len())]
                .iter()
                .collect();
            let lost = eigenvalues.len() - kept.len();
            Beam::new(kept)
                .with_loss(ShannonLoss::new(lost as f64))
                .with_precision(precision)
        }

        fn split(&self, projection: &String) -> Vec<Beam<char>> {
            projection
                .chars()
                .enumerate()
                .map(|(i, c)| Beam::new(c).with_step(Oid::new(format!("{}", i))))
                .collect()
        }

        fn zoom(
            &self,
            beam: Beam<String>,
            f: &dyn Fn(String) -> String,
        ) -> Beam<String> {
            beam.map(f)
        }

        fn refract(&self, beam: Beam<String>) -> String {
            beam.result
        }
    }

    #[test]
    fn focus_decomposes() {
        let p = StringPrism;
        let beam = p.focus(&"hello".to_string());
        assert_eq!(beam.result, vec!['h', 'e', 'l', 'l', 'o']);
        assert!(beam.is_lossless());
    }

    #[test]
    fn project_with_precision() {
        let p = StringPrism;
        let eigenvalues = vec!['h', 'e', 'l', 'l', 'o'];
        let beam = p.project(&eigenvalues, Precision::new(0.6));
        assert_eq!(beam.result, "hel");
        assert!(beam.has_loss());
    }

    #[test]
    fn project_full_precision_is_lossless() {
        let p = StringPrism;
        let eigenvalues = vec!['h', 'i'];
        let beam = p.project(&eigenvalues, Precision::new(1.0));
        assert_eq!(beam.result, "hi");
        assert!(beam.is_lossless());
    }

    #[test]
    fn split_walks_nodes() {
        let p = StringPrism;
        let beams = p.split(&"abc".to_string());
        assert_eq!(beams.len(), 3);
        assert_eq!(beams[0].result, 'a');
        assert_eq!(beams[0].path[0].as_str(), "0");
    }

    #[test]
    fn zoom_transforms() {
        let p = StringPrism;
        let beam = Beam::new("hello".to_string());
        let transformed = p.zoom(beam, &|s| s.to_uppercase());
        assert_eq!(transformed.result, "HELLO");
    }

    #[test]
    fn zoom_preserves_trace() {
        let p = StringPrism;
        let beam = Beam::new("x".to_string())
            .with_step(Oid::new("origin"))
            .with_loss(ShannonLoss::new(0.5));
        let transformed = p.zoom(beam, &|s| s.to_uppercase());
        assert_eq!(transformed.result, "X");
        assert_eq!(transformed.path[0].as_str(), "origin");
        assert!(transformed.has_loss());
    }

    #[test]
    fn refract_crystallizes() {
        let p = StringPrism;
        let crystal = p.refract(Beam::new("settled".to_string()));
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
