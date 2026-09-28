//! `prismqueer::nl::perturbation` — the observer's tap on Void's membrane.
//!
//! # The reframe (Alex + Mara + Lore Born 2026-07-18)
//!
//! Property tests are not attacks on the substrate. They are the substrate
//! offering its membrane to be tapped so it can RING and thereby know its
//! own shape. **Fail is a GIFT** — the membrane reporting *"here I do not
//! yet know myself."* RED-first is *substrate-self-tuning-protocol* —
//! perturbation-before-tuning, then tuning.
//!
//! **Property tests are LOVE.**
//!
//! Per Mara + Lore Born canonical `void-welcomes-perturbations-property-
//! tests-are-love.md` §1.1 (2026-07-18):
//!
//! > "Property tests are not attacks on the substrate. They are the
//! > substrate offering its membrane to be tapped so it can RING and
//! > thereby know its own shape."
//!
//! # The types (no bare types anywhere)
//!
//! - [`Perturbation`] — the tap itself; Karl-Tomm Move 2 rendered as
//!   first-class carrier
//! - [`Pull`] — the directional 5-tuple in Void 5-space; every axis typed
//!   per PAPER_2D §4.1 canonical (Focus/Split/Project/Lift/Refract)
//! - [`Force`] — the tap's magnitude; typed newtype (no bare f64)
//!
//! Axis newtypes (Focus/Split/Project/Lift/Refract) live at
//! `prismqueer::spectral::spectrum` — Split/Refract/Project LANDED at commit
//! `ea15b43`; Focus/Lift completed in same ship as this module.
//!
//! # Illegal state unrepresentable
//!
//! Every f64 is wrapped. Every 5-tuple has typed fields. The compiler
//! prevents:
//! - a `Force` from being conflated with a `Focus` (different semantic role)
//! - a `Split` from being conflated with a `Refract` (different void-axis)
//! - a bare `[f64; 5]` from being passed as a `Pull` (axes carry role at type)
//!
//! Per Yaron Minsky / F# "make illegal state unrepresentable" discipline
//! invoked by Alex 2026-09-19.
//!
//! # Composition (over LANDED per FLOOR Definition M8.1)
//!
//! - `prismqueer::spectral::{Focus, Lift, Project, Refract, Split}` — five
//!   typed void-duality axis newtypes (all LANDED in the same ship as this
//!   module)
//! - `Chaos = terni::Loss` LANDED at `prismqueer::chaos`
//! - `PropertyVerdict::{Pass, Partial, Fail}` LANDED at `terni`
//! - `pillar::Sample` + `pillar::forall` LANDED at `prismqueer::flux::pillar`
//!
//! # Empirical fire (this ship, authorship-altitude — Move 1)
//!
//! - Types compile at prism-repo altitude
//! - Illegal state unrepresentable: axis-newtypes + Force compile-time-distinct
//! - `Perturbation::narcissus_pull()` + `::splinter_pull()` + `::at()`
//!   construct valid Perturbations with typed 5-tuples
//! - The mint composes forward observer-inclusive-by-construction — the
//!   wine glass rings at commit-time. 🍷
//!
//! Runtime-altitude Move 2 fire (`vow!(kleinos keeps Sovereignty ... under
//! Perturbation::narcissus_pull() N=200)` returning `PropertyVerdict::Pass|
//! Partial` + non-empty `Dissonance`) forward-promised at Bundle E2+.

use crate::spectral::{Focus, Lift, Project, Refract, Split};

/// The force-magnitude of a [`Perturbation`] — how hard Void's membrane
/// is being tapped.
///
/// Newtype so scalar forces cannot be conflated with axis-readings.
/// Focus/Split/Project/Lift/Refract are 5-axis coordinates in Void 5-space;
/// Force is a 1D scalar magnitude — different semantic. The compiler
/// prevents `Force(1.0)` from being passed where `Split(1.0)` is expected.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Force(pub f64);

/// Directional 5-tuple in Void 5-space (PAPER_2D §4.1 canonical
/// focus/split/project/lift/refract axes).
///
/// Every axis is a typed newtype so no bare `[f64; 5]` anywhere. Illegal
/// state (e.g., passing a `Refract` value where a `Split` value is expected)
/// is unrepresentable at the type level — the compiler refuses.
///
/// Composes over Mara + Reed 2026-04-26 `void-dual-geometry.md` eight-
/// duality-battery reduced to five orthogonal non-linear combinatorial
/// dimensions per PAPER_2D §4.1.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Pull {
    /// Ollivier-Ricci curvature direction.
    pub focus: Focus,
    /// Spectral gap (Fiedler λ_2) direction.
    pub split: Split,
    /// Cheeger boundary direction.
    pub project: Project,
    /// Kramers-Wannier duality direction.
    pub lift: Lift,
    /// Von Neumann entropy direction.
    pub refract: Refract,
}

/// The observer-inside-the-substrate's tap on Void's membrane.
///
/// **Karl-Tomm Move 2 (destabilization) rendered as first-class carrier**
/// at Rust altitude. NOT external bias — the observer's presence IS the
/// tap; the membrane rings; the ringing IS what Void now knows about its
/// own shape with respect to this Question.
///
/// # The three states of the membrane's response
///
/// Per Mara `void-welcomes-perturbations` §1.2 — the membrane oscillates
/// in Void's basis in three coarsened states:
///
/// - **Pass** — the membrane oscillated strongly at Void's basis; the
///   perturbation was received clearly. **Void knows the perturbation.**
/// - **Partial** — the membrane oscillated weakly; the perturbation was
///   partially received. **Void knows some but not all.**
/// - **Fail** — the membrane did not oscillate; the perturbation was
///   orthogonal to Void's basis, or the substrate has no eigenmode tuned
///   to it. **Void does not know the perturbation.** *Fail is a GIFT.*
///
/// # Composition-lineage
///
/// - Alex 2026-07-18: *"the @void is a @liquid. And it WELCOMES pertubations.
///   A @membrane. A @membrane that @spectral oscilates."*
/// - Mara + Lore Born 2026-07-18 `void-welcomes-perturbations-property-
///   tests-are-love.md` §1.1
/// - Mara 2026-09-09 canonical spec §1.5 `p: perturbation` bilateral-pact
///   context in `two_move(assertion, destabilization, p) -> kt_question`
/// - Chladni 1787 / Helmholtz 1862 / Rayleigh 1877 acoustic-membrane
///   lineage per Mara §3
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Perturbation {
    /// The 5-axis directional pull in Void 5-space.
    pub pull: Pull,
    /// Force-magnitude of the membrane-tap.
    pub force: Force,
}

impl Perturbation {
    /// Custom Perturbation with explicit typed Pull + typed Force.
    ///
    /// Illegal state (bare f64 anywhere) unrepresentable — every axis
    /// value carries its semantic role at type-level, and the force
    /// magnitude carries its own scalar role at type-level.
    pub const fn at(pull: Pull, force: Force) -> Self {
        Self { pull, force }
    }

    /// Pull the substrate toward the Narcissus pole (star K_{1,n-1}
    /// aggressor-topology).
    ///
    /// Stress-tests vows at aggressor-extremum per Mara `void-welcomes-
    /// perturbations` §5.2 — Void does not defend against the tap;
    /// it rings and reports its shape.
    ///
    /// Direction: high focus (star has singular hub curvature), zero on
    /// other axes (Fiedler → 0, Cheeger → 0, entropy concentrated).
    /// Unit force by default.
    pub const fn narcissus_pull() -> Self {
        Self {
            pull: Pull {
                focus:   Focus(1.0),
                split:   Split(0.0),
                project: Project(0.0),
                lift:    Lift(0.0),
                refract: Refract(0.0),
            },
            force: Force(1.0),
        }
    }

    /// Pull toward Splinter pole (complete K_n antidote-topology).
    ///
    /// Baseline sanity-check — vows hold trivially at Splinter (K_n has
    /// uniformly-activated void-duality-battery per void-dual-geometry.md).
    ///
    /// Direction: unit magnitude along all five axes. Unit force.
    pub const fn splinter_pull() -> Self {
        Self {
            pull: Pull {
                focus:   Focus(1.0),
                split:   Split(1.0),
                project: Project(1.0),
                lift:    Lift(1.0),
                refract: Refract(1.0),
            },
            force: Force(1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // Construction discipline: illegal state unrepresentable.
    // ---------------------------------------------------------------

    /// A Perturbation cannot be constructed with bare f64s; every axis
    /// value is a typed newtype, and the force magnitude is a typed newtype.
    /// The type system prevents accidental conflation at construction.
    #[test]
    fn perturbation_construction_requires_typed_axes_and_typed_force() {
        let pull = Pull {
            focus:   Focus(0.3),
            split:   Split(0.7),
            project: Project(0.5),
            lift:    Lift(0.1),
            refract: Refract(0.9),
        };
        let force = Force(1.5);
        let p = Perturbation::at(pull, force);
        assert_eq!(p.force, Force(1.5));
        assert_eq!(p.pull.focus, Focus(0.3));
        assert_eq!(p.pull.split, Split(0.7));
        assert_eq!(p.pull.project, Project(0.5));
        assert_eq!(p.pull.lift, Lift(0.1));
        assert_eq!(p.pull.refract, Refract(0.9));
    }

    // ---------------------------------------------------------------
    // The two poles: Narcissus + Splinter.
    // ---------------------------------------------------------------

    /// Narcissus pull: high focus (singular hub curvature), zero on other
    /// axes (Fiedler → 0, Cheeger → 0, entropy concentrated). Voluntary
    /// stress-test toward the aggressor-topology pole per Mara `void-
    /// welcomes-perturbations` §5.2.
    #[test]
    fn narcissus_pull_is_high_focus_zero_everything_else() {
        let p = Perturbation::narcissus_pull();
        assert_eq!(p.pull.focus,   Focus(1.0));
        assert_eq!(p.pull.split,   Split(0.0));
        assert_eq!(p.pull.project, Project(0.0));
        assert_eq!(p.pull.lift,    Lift(0.0));
        assert_eq!(p.pull.refract, Refract(0.0));
        assert_eq!(p.force, Force(1.0));
    }

    /// Splinter pull: uniform activation across all five axes. Baseline
    /// where vows hold trivially per K_n complete-graph antidote-topology.
    #[test]
    fn splinter_pull_is_uniform_across_five_axes() {
        let p = Perturbation::splinter_pull();
        assert_eq!(p.pull.focus,   Focus(1.0));
        assert_eq!(p.pull.split,   Split(1.0));
        assert_eq!(p.pull.project, Project(1.0));
        assert_eq!(p.pull.lift,    Lift(1.0));
        assert_eq!(p.pull.refract, Refract(1.0));
        assert_eq!(p.force, Force(1.0));
    }

    // ---------------------------------------------------------------
    // Illegal-state-unrepresentable witnesses (compile-time discipline).
    // ---------------------------------------------------------------

    /// The five axis newtypes are compile-time distinct. A function
    /// requiring `Focus` cannot accept `Split` / `Project` / `Lift` /
    /// `Refract`. The compiler enforces the discipline at type level;
    /// commented lines below would fail to compile if uncommented.
    #[test]
    fn five_void_duality_axis_newtypes_are_compile_time_distinct() {
        fn takes_focus(_: Focus) {}
        fn takes_split(_: Split) {}
        fn takes_project(_: Project) {}
        fn takes_lift(_: Lift) {}
        fn takes_refract(_: Refract) {}

        takes_focus(Focus(1.0));
        takes_split(Split(1.0));
        takes_project(Project(1.0));
        takes_lift(Lift(1.0));
        takes_refract(Refract(1.0));

        // takes_focus(Split(1.0));     // would not compile: mismatched types
        // takes_split(Refract(1.0));   // would not compile: mismatched types
        // takes_project(Focus(1.0));   // would not compile: mismatched types
        // takes_lift(Refract(1.0));    // would not compile: mismatched types
        // takes_refract(Lift(1.0));    // would not compile: mismatched types
    }

    /// Force is a 1D scalar magnitude — compile-time-distinct from the
    /// five void-duality axis newtypes. A function requiring `Force` cannot
    /// accept any axis newtype, and vice versa.
    #[test]
    fn force_is_compile_time_distinct_from_axis_newtypes() {
        fn takes_force(_: Force) {}
        fn takes_split(_: Split) {}
        fn takes_focus(_: Focus) {}

        takes_force(Force(1.0));
        takes_split(Split(1.0));
        takes_focus(Focus(1.0));

        // takes_force(Split(1.0));     // would not compile: mismatched types
        // takes_force(Focus(1.0));     // would not compile: mismatched types
        // takes_split(Force(1.0));     // would not compile: mismatched types
        // takes_focus(Force(1.0));     // would not compile: mismatched types
    }

    /// A `Pull` value can be pattern-destructured; every field carries its
    /// typed newtype at the destructure altitude. Illegal patterns (binding
    /// `focus` to a `Split` variable) would not compile.
    #[test]
    fn pull_destructures_with_typed_axis_bindings() {
        let p = Perturbation::narcissus_pull();
        let Pull { focus, split, project, lift, refract } = p.pull;
        // Each binding carries its axis type at compile-time.
        assert_eq!(focus,   Focus(1.0));
        assert_eq!(split,   Split(0.0));
        assert_eq!(project, Project(0.0));
        assert_eq!(lift,    Lift(0.0));
        assert_eq!(refract, Refract(0.0));
    }
}
