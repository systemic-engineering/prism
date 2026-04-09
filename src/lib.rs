//! Prism — focus | project | split | zoom | refract.
//!
//! Five operations. The optic hierarchy as a trait.
//! The parser's vocabulary. The runtime's interface.
//! Everything else is a composition of these five.
//!
//! Beam<T>, Oid, ShannonLoss, Precision, Pressure, Recovery, ContentAddressed.
//! Zero dependencies. The types that outlive everything around them.
//!
//! A prism splits light into beams. A crystal is the lossless fixed point.
//!
//! For the composition layer — monoid structure, meta-prisms, and
//! classical functional optics — enable the `optics` cargo feature.

pub mod beam;
pub mod connection;
pub mod content;
pub mod loss;
pub mod metal;
pub mod oid;
pub mod precision;
pub mod spectral_oid;

#[cfg(feature = "optics")]
pub mod optics;

pub use beam::{Beam, Recovery, Stage};
pub use connection::{Connection, ScalarConnection};
pub use content::ContentAddressed;
pub use loss::ShannonLoss;
pub use oid::Oid;
pub use precision::{Precision, Pressure};
pub use spectral_oid::SpectralOid;

// ---------------------------------------------------------------------------
// The Prism trait — five optic operations over Beam<T>
// ---------------------------------------------------------------------------

/// The five optic operations. The shared grammar of computation.
///
/// Any system that decomposes, projects, walks, transforms, and
/// settles implements Prism. The parser. The compiler. The runtime.
/// The database.
///
/// Loss accumulates. Precision narrows. The Stage field tracks where
/// in the pipeline a beam currently lives. Each operation's output
/// type is the next operation's input type — the pipeline chains
/// without clones:
///
///   focus: Beam<Input> → Beam<Focused>
///   project: Beam<Focused> → Beam<Projected>
///   split: Beam<Projected> → Vec<Beam<Part>>
///   zoom: (Beam<Projected>, f) → Beam<Projected>
///   refract: Beam<Projected> → Beam<Crystal>
///
/// `split` is the only operation that leaves the single-beam monoid:
/// it produces a population of beams at a "level above." Gathering
/// those beams back into one is the job of a meta-prism, which lives
/// in the optics layer — not a method on the base trait. A single
/// prism physically disperses; it cannot recombine its own output.
///
/// The Crystal associated type is bounded by `Prism<Crystal =
/// Self::Crystal>`, the fixed-point property: after one `refract`
/// the type stabilises, enabling the boot fold
/// MetalPrism → MirrorPrism → … → stdlib to typecheck without GATs
/// or trait objects.
pub trait Prism {
    /// The raw input (source text, graph, domain value).
    type Input;
    /// Output of focus; input of project.
    type Focused;
    /// Output of project; input of split / zoom / refract.
    type Projected;
    /// What split yields — a single part of the projection.
    type Part;
    /// The crystal of a prism is itself a prism, and its crystal is the
    /// same type — the fixed-point property after one refract. This is
    /// what makes the boot fold terminate: after MetalPrism.refract(…)
    /// the type stabilises at MirrorPrism.
    type Crystal: Prism<Crystal = Self::Crystal>;

    /// Focus: read-only decomposition. Beam<Input> → Beam<Focused>.
    /// Accumulates into the beam's trace. Transitions stage to Focused.
    fn focus(&self, beam: Beam<Self::Input>) -> Beam<Self::Focused>;

    /// Project: precision-bounded projection. Beam<Focused> → Beam<Projected>.
    /// Uses the beam's precision field — callers narrow precision by passing
    /// `beam.with_precision(p)`. Refutation is encoded as ShannonLoss in the
    /// returned beam, not as Option/Result. Transitions stage to Projected.
    fn project(&self, beam: Beam<Self::Focused>) -> Beam<Self::Projected>;

    /// Split: multi-target walk. Beam<Projected> → Vec<Beam<Part>>.
    /// Each child beam inherits the parent's path (with an added step)
    /// and its loss/precision. Transitions stage to Split.
    fn split(&self, beam: Beam<Self::Projected>) -> Vec<Beam<Self::Part>>;

    /// Zoom: apply a Beam → Beam transformation in projected context.
    /// Stays in Projected-space (it's a self-map on Beam<Projected>).
    fn zoom(
        &self,
        beam: Beam<Self::Projected>,
        f: &dyn Fn(Beam<Self::Projected>) -> Beam<Self::Projected>,
    ) -> Beam<Self::Projected>;

    /// Refract: settle into the lossless invertible fixed point.
    /// Beam<Projected> → Beam<Crystal>. The crystal IS another prism
    /// (the Crystal associated type is bounded by Prism). For meta-level
    /// prisms, this is the construction primitive: refract a projected
    /// beam, get a new prism back.
    ///
    /// Impls SHOULD refuse to crystallize a beam with infinite loss,
    /// returning it unchanged with stage still at Projected rather than
    /// transitioning to Refracted.
    fn refract(&self, beam: Beam<Self::Projected>) -> Beam<Self::Crystal>;

    // ---- Raw-input shortcut (default impl) ----

    /// Wrap a raw input in a fresh beam and focus it.
    /// Only `focus_in` makes sense as a raw-input shortcut: project,
    /// split, and refract all take mid-pipeline types. Use `apply` for
    /// the full pipeline.
    fn focus_in(&self, input: Self::Input) -> Beam<Self::Focused> {
        self.focus(Beam::new(input))
    }
}

/// Apply a Prism end-to-end: focus → project → refract.
///
/// The read-cut-settle pipeline. Loss accumulates. Precision narrows.
/// Each operation genuinely transforms its input into its output — the
/// beam flows through without clones or metadata patching.
///
/// The returned beam carries the settled Crystal (which is itself a Prism).
/// If callers need zoom or split, they call those operations explicitly
/// on the intermediate beams — zoom is a per-stage tool, not a top-level
/// pipeline stage.
pub fn apply<P: Prism>(prism: &P, input: P::Input) -> Beam<P::Crystal> {
    let focused = prism.focus(Beam::new(input));
    let projected = prism.project(focused);
    prism.refract(projected)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple string prism used to exercise all six operations.
    ///
    /// type Input     = String      (raw string to decompose)
    /// type Focused   = Vec<char>   (focus yields the char decomposition)
    /// type Projected = String      (project yields the precision-cut string)
    /// type Part      = char        (split yields individual chars)
    /// type Crystal   = StringPrism (fixed-point: refract returns another StringPrism)
    struct StringPrism;

    impl Prism for StringPrism {
        type Input = String;
        type Focused = Vec<char>;
        type Projected = String;
        type Part = char;
        /// Fixed-point case: Crystal = Self, so the recursion terminates.
        type Crystal = StringPrism;

        fn focus(&self, beam: Beam<String>) -> Beam<Vec<char>> {
            let result: Vec<char> = beam.result.chars().collect();
            Beam {
                result,
                path: beam.path,
                loss: beam.loss,
                precision: beam.precision,
                recovered: beam.recovered,
                stage: Stage::Focused,
                connection: beam.connection,
            }
        }

        fn project(&self, beam: Beam<Vec<char>>) -> Beam<String> {
            let precision = beam.precision.as_f64();
            if precision <= 0.0 {
                // No precision set — pass through losslessly.
                let result: String = beam.result.iter().collect();
                return Beam {
                    result,
                    path: beam.path,
                    loss: beam.loss,
                    precision: beam.precision,
                    recovered: beam.recovered,
                    stage: Stage::Projected,
                    connection: beam.connection,
                };
            }
            let cutoff = (beam.result.len() as f64 * precision) as usize;
            let cutoff = cutoff.min(beam.result.len());
            let kept: String = beam.result[..cutoff].iter().collect();
            let lost = beam.result.len() - cutoff;
            Beam {
                result: kept,
                path: beam.path,
                loss: ShannonLoss::new(beam.loss.as_f64() + lost as f64),
                precision: beam.precision,
                recovered: beam.recovered,
                stage: Stage::Projected,
                connection: beam.connection,
            }
        }

        fn split(&self, beam: Beam<String>) -> Vec<Beam<char>> {
            let parent_loss = beam.loss;
            let parent_precision = beam.precision;
            beam.result
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    let mut path = beam.path.clone();
                    path.push(Oid::new(format!("{}", i)));
                    Beam {
                        result: c,
                        path,
                        loss: parent_loss.clone(),
                        precision: parent_precision.clone(),
                        recovered: None,
                        stage: Stage::Split,
                        connection: Default::default(),
                    }
                })
                .collect()
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
            Beam {
                result: StringPrism,
                path: beam.path,
                loss: beam.loss,
                precision: beam.precision,
                recovered: beam.recovered,
                stage: Stage::Refracted,
                connection: beam.connection,
            }
        }
    }

    #[test]
    fn focus_decomposes() {
        let p = StringPrism;
        let beam = p.focus(Beam::new("hello".to_string()));
        // focus yields Vec<char>
        assert_eq!(beam.result, vec!['h', 'e', 'l', 'l', 'o']);
        assert!(beam.is_lossless());
        assert_eq!(beam.stage, Stage::Focused);
    }

    #[test]
    fn project_with_precision() {
        let p = StringPrism;
        // project takes Beam<Vec<char>> (focus's output)
        let focused = p.focus(Beam::new("hello".to_string()).with_precision(Precision::new(0.6)));
        let beam = p.project(focused);
        // 5 chars * 0.6 = 3.0 → cutoff 3 → "hel"
        assert_eq!(beam.result, "hel");
        assert!(beam.has_loss());
        assert_eq!(beam.stage, Stage::Projected);
    }

    #[test]
    fn project_full_precision_is_lossless() {
        let p = StringPrism;
        let focused = p.focus(Beam::new("hi".to_string()).with_precision(Precision::new(1.0)));
        let beam = p.project(focused);
        assert_eq!(beam.result, "hi");
        assert!(beam.is_lossless());
        assert_eq!(beam.stage, Stage::Projected);
    }

    #[test]
    fn split_walks_parts() {
        let p = StringPrism;
        // split takes Beam<Projected> = Beam<String>
        let projected = p.project(p.focus(Beam::new("abc".to_string())));
        let beams = p.split(projected);
        assert_eq!(beams.len(), 3);
        assert_eq!(beams[0].result, 'a');
        assert_eq!(beams[0].path[0].as_str(), "0");
        assert_eq!(beams[0].stage, Stage::Split);
    }

    #[test]
    fn zoom_transforms() {
        let p = StringPrism;
        // zoom takes Beam<Projected> = Beam<String>
        let projected = p.project(p.focus(Beam::new("hello".to_string())));
        let transformed = p.zoom(projected, &|b| b.map(|s| s.to_uppercase()));
        assert_eq!(transformed.result, "HELLO");
    }

    #[test]
    fn zoom_preserves_trace() {
        let p = StringPrism;
        // zoom takes Beam<Projected> = Beam<String>
        let projected = p
            .project(p.focus(
                Beam::new("x".to_string())
                    .with_step(Oid::new("origin"))
                    .with_loss(ShannonLoss::new(0.5)),
            ));
        let transformed = p.zoom(projected, &|b| b.map(|s| s.to_uppercase()));
        assert_eq!(transformed.result, "X");
        assert_eq!(transformed.path[0].as_str(), "origin");
        assert!(transformed.has_loss());
    }

    #[test]
    fn refract_crystallizes() {
        let p = StringPrism;
        // refract takes Beam<Projected> = Beam<String>
        let projected = p.project(p.focus(Beam::new("settled".to_string())));
        let beam = p.refract(projected);
        // Crystal is a StringPrism (unit struct). Stage must be Refracted.
        assert_eq!(beam.stage, Stage::Refracted);
        // The crystal prism is itself a valid Prism implementation.
        let inner = beam.result.focus(Beam::new("verify".to_string()));
        assert_eq!(inner.result, vec!['v', 'e', 'r', 'i', 'f', 'y']);
    }

    #[test]
    fn apply_full_pipeline() {
        let p = StringPrism;
        // apply chains focus → project → refract; no precision set → lossless project
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

}
