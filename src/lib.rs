//! Prism — focus | project | split | join | zoom | refract.
//!
//! Six operations. The optic hierarchy as a trait.
//! The parser's vocabulary. The runtime's interface.
//! Everything else is a composition of these six.
//!
//! Beam<T>, Oid, ShannonLoss, Precision, Pressure, Recovery, ContentAddressed.
//! Zero dependencies. The types that outlive everything around them.
//!
//! A prism splits light into beams. A crystal is the lossless fixed point.

pub mod beam;
pub mod content;
pub mod loss;
pub mod metal;
pub mod oid;
pub mod precision;
pub mod spectral_oid;

pub use beam::{Beam, Recovery, Stage};
pub use content::ContentAddressed;
pub use loss::ShannonLoss;
pub use oid::Oid;
pub use precision::{Precision, Pressure};
pub use spectral_oid::SpectralOid;

// ---------------------------------------------------------------------------
// The Prism trait — six optic operations over Beam<T>
// ---------------------------------------------------------------------------

/// The six optic operations. The shared grammar of computation.
///
/// Any system that decomposes, projects, walks, joins, transforms,
/// and settles implements Prism. The parser. The compiler. The
/// runtime. The database.
///
/// All operations are Beam → Beam. Loss accumulates. Precision
/// narrows. The Stage field tracks where in the pipeline a beam
/// currently lives.
///
/// The Crystal associated type is bounded by `Prism<Crystal =
/// Self::Crystal>`, the fixed-point property: after one `refract`
/// the type stabilises, enabling the boot fold
/// MetalPrism → MirrorPrism → … → stdlib to typecheck without GATs
/// or trait objects.
pub trait Prism {
    /// The raw input (source text, graph, domain value).
    type Input;
    /// What the input decomposes to (eigenvalues, structured form).
    type Output;
    /// A single node visited during split.
    type Node;
    /// The crystal of a prism is itself a prism, and its crystal is the
    /// same type — the fixed-point property after one refract. This is
    /// what makes the boot fold terminate: after MetalPrism.refract(…)
    /// the type stabilises at MirrorPrism.
    type Crystal: Prism<Crystal = Self::Crystal>;

    /// Focus: read-only decomposition. Total. Maps Beam<Input> → Beam<Output>.
    /// Accumulates into the beam's trace. Transitions stage to Focused.
    fn focus(&self, beam: Beam<Self::Input>) -> Beam<Self::Output>;

    /// Project: precision-bounded projection. Uses the beam's precision
    /// field — callers narrow precision by passing `beam.with_precision(p)`.
    /// Refutation is encoded as ShannonLoss in the returned beam, not as
    /// Option/Result. Transitions stage to Projected.
    fn project(&self, beam: Beam<Self::Input>) -> Beam<Self::Output>;

    /// Split: multi-target walk. Each child beam inherits the parent's
    /// path (with an added step) and its loss/precision. Transitions
    /// stage to Split.
    fn split(&self, beam: Beam<Self::Input>) -> Vec<Beam<Self::Node>>;

    /// Join: the inverse of split. Collapse a vector of child beams back
    /// into one parent beam. How loss aggregates is impl-specific (sum,
    /// max, Shannon-add). Transitions stage to Joined.
    fn join(&self, children: Vec<Beam<Self::Node>>) -> Beam<Self::Output>;

    /// Zoom: apply a Beam → Beam transformation in focused context.
    /// The function operates on a beam; higher-level "zoom into a sub-prism"
    /// is built by passing `|beam| sub.focus(beam)` or similar.
    fn zoom(
        &self,
        beam: Beam<Self::Input>,
        f: &dyn Fn(Beam<Self::Input>) -> Beam<Self::Output>,
    ) -> Beam<Self::Output>;

    /// Refract: settle into the lossless invertible fixed point. The
    /// crystal IS another prism (the Crystal associated type is bounded
    /// by Prism). For meta-level prisms, this is the construction
    /// primitive: refract a source beam, get a new prism back.
    ///
    /// Impls SHOULD refuse to crystallize a beam with infinite loss,
    /// returning it unchanged with stage still at Projected rather than
    /// transitioning to Refracted.
    fn refract(&self, beam: Beam<Self::Input>) -> Beam<Self::Crystal>;

    // ---- Raw-input shortcut methods (default impls) ----

    fn focus_in(&self, input: Self::Input) -> Beam<Self::Output> {
        self.focus(Beam::new(input))
    }
    fn project_in(&self, input: Self::Input) -> Beam<Self::Output> {
        self.project(Beam::new(input))
    }
    fn split_in(&self, input: Self::Input) -> Vec<Beam<Self::Node>> {
        self.split(Beam::new(input))
    }
    fn refract_in(&self, input: Self::Input) -> Beam<Self::Crystal> {
        self.refract(Beam::new(input))
    }
}

/// Apply a Prism end-to-end: focus → project → refract.
///
/// Because all three operations take `Beam<Input>`, the pipeline
/// passes a fresh `Beam::new(input)` to each stage independently.
/// Loss is forwarded from focus → project by accumulation; the
/// refract step receives the projected beam's accumulated state.
///
/// The returned beam carries either a settled crystal or an
/// infinite-loss refutation.
///
/// If callers need zoom, they call it explicitly — zoom is a
/// per-stage tool, not a top-level pipeline stage.
pub fn apply<P: Prism>(prism: &P, input: P::Input) -> Beam<P::Crystal>
where
    P::Input: Clone,
{
    let focused = prism.focus(Beam::new(input.clone()));
    let project_beam = Beam::new(input.clone())
        .with_loss(focused.loss)
        .with_precision(focused.precision)
        .with_stage(focused.stage);
    let projected = prism.project(project_beam);
    let refract_beam = Beam::new(input)
        .with_loss(projected.loss)
        .with_precision(projected.precision)
        .with_stage(projected.stage);
    prism.refract(refract_beam)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple string prism used to exercise all six operations.
    ///
    /// type Input  = String  (raw string to decompose)
    /// type Output = String  (the (possibly truncated) string after project)
    /// type Node   = char    (individual characters from split)
    /// type Crystal = StringPrism (fixed-point: refract returns another StringPrism)
    struct StringPrism;

    impl Prism for StringPrism {
        type Input = String;
        type Output = String;
        type Node = char;
        /// Fixed-point case: Crystal = Self, so the recursion terminates.
        type Crystal = StringPrism;

        fn focus(&self, beam: Beam<String>) -> Beam<String> {
            beam.with_stage(Stage::Focused)
        }

        fn project(&self, beam: Beam<String>) -> Beam<String> {
            let precision = beam.precision.as_f64();
            if precision <= 0.0 {
                // No precision set — pass through losslessly.
                return beam.with_stage(Stage::Projected);
            }
            let chars: Vec<char> = beam.result.chars().collect();
            let cutoff = (chars.len() as f64 * precision) as usize;
            let cutoff = cutoff.min(chars.len());
            let kept: String = chars[..cutoff].iter().collect();
            let lost = chars.len() - cutoff;
            Beam::new(kept)
                .with_loss(ShannonLoss::new(lost as f64))
                .with_precision(beam.precision)
                .with_stage(Stage::Projected)
        }

        fn split(&self, beam: Beam<String>) -> Vec<Beam<char>> {
            let parent_loss = beam.loss.clone();
            let parent_precision = beam.precision.clone();
            beam.result
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    let mut child = Beam::new(c)
                        .with_step(Oid::new(format!("{}", i)))
                        .with_stage(Stage::Split);
                    child.loss = parent_loss.clone();
                    child.precision = parent_precision.clone();
                    child
                })
                .collect()
        }

        fn join(&self, children: Vec<Beam<char>>) -> Beam<String> {
            let s: String = children.iter().map(|b| b.result).collect();
            let total_loss: f64 = children.iter().map(|b| b.loss.as_f64()).sum();
            Beam::new(s)
                .with_loss(ShannonLoss::new(total_loss))
                .with_stage(Stage::Joined)
        }

        fn zoom(
            &self,
            beam: Beam<String>,
            f: &dyn Fn(Beam<String>) -> Beam<String>,
        ) -> Beam<String> {
            f(beam)
        }

        fn refract(&self, beam: Beam<String>) -> Beam<StringPrism> {
            // StringPrism has no fields — the crystal is a fresh instance.
            // Stage transitions to Refracted (beam result is the crystal prism itself).
            Beam::new(StringPrism)
                .with_stage(Stage::Refracted)
                .with_loss(beam.loss)
        }
    }

    #[test]
    fn focus_decomposes() {
        let p = StringPrism;
        let beam = p.focus(Beam::new("hello".to_string()));
        assert_eq!(beam.result, "hello");
        assert!(beam.is_lossless());
        assert_eq!(beam.stage, Stage::Focused);
    }

    #[test]
    fn project_with_precision() {
        let p = StringPrism;
        let input = Beam::new("hello".to_string()).with_precision(Precision::new(0.6));
        let beam = p.project(input);
        // 5 chars * 0.6 = 3.0 → cutoff 3 → "hel"
        assert_eq!(beam.result, "hel");
        assert!(beam.has_loss());
        assert_eq!(beam.stage, Stage::Projected);
    }

    #[test]
    fn project_full_precision_is_lossless() {
        let p = StringPrism;
        let input = Beam::new("hi".to_string()).with_precision(Precision::new(1.0));
        let beam = p.project(input);
        assert_eq!(beam.result, "hi");
        assert!(beam.is_lossless());
        assert_eq!(beam.stage, Stage::Projected);
    }

    #[test]
    fn split_walks_nodes() {
        let p = StringPrism;
        let beams = p.split(Beam::new("abc".to_string()));
        assert_eq!(beams.len(), 3);
        assert_eq!(beams[0].result, 'a');
        assert_eq!(beams[0].path[0].as_str(), "0");
        assert_eq!(beams[0].stage, Stage::Split);
    }

    #[test]
    fn zoom_transforms() {
        let p = StringPrism;
        let beam = Beam::new("hello".to_string());
        let transformed = p.zoom(beam, &|b| b.map(|s| s.to_uppercase()));
        assert_eq!(transformed.result, "HELLO");
    }

    #[test]
    fn zoom_preserves_trace() {
        let p = StringPrism;
        let beam = Beam::new("x".to_string())
            .with_step(Oid::new("origin"))
            .with_loss(ShannonLoss::new(0.5));
        let transformed = p.zoom(beam, &|b| b.map(|s| s.to_uppercase()));
        assert_eq!(transformed.result, "X");
        assert_eq!(transformed.path[0].as_str(), "origin");
        assert!(transformed.has_loss());
    }

    #[test]
    fn refract_crystallizes() {
        let p = StringPrism;
        let beam = p.refract(Beam::new("settled".to_string()));
        // Crystal is a StringPrism (unit struct). Stage must be Refracted.
        assert_eq!(beam.stage, Stage::Refracted);
        // The crystal prism is itself a valid Prism implementation.
        let inner = p.focus(Beam::new("verify".to_string()));
        assert_eq!(inner.result, "verify");
    }

    #[test]
    fn apply_full_pipeline() {
        let p = StringPrism;
        // apply does focus → project → refract; no precision set → lossless project
        let beam = apply(&p, "hello world".to_string());
        assert_eq!(beam.stage, Stage::Refracted);
    }

    #[test]
    fn apply_full_precision() {
        let p = StringPrism;
        let beam = apply(&p, "hi".to_string());
        assert_eq!(beam.stage, Stage::Refracted);
        assert!(beam.is_lossless());
    }

    #[test]
    fn join_reverses_split() {
        let p = StringPrism;
        let children = p.split(Beam::new("hi".to_string()));
        let rejoined = p.join(children);
        assert_eq!(rejoined.result, "hi");
        assert_eq!(rejoined.stage, Stage::Joined);
    }
}
