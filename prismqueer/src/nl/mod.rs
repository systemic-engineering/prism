//! `prismqueer::nl` — the natural-language altitude of the observer-inclusive
//! test-authoring vocabulary.
//!
//! Per Alex 2026-09-15 Reframe #5 (`mirror/docs/specs/2026-09-15-mara-prismqueer-
//! truth-zeroth-order-pipeline-system-VSM-meta-trait-canonical-spec.md`):
//!
//! > "What if the Question struct is the translator between the linear natural
//! > language representation of the non-linear spectral commutator and this is
//! > where the @nl and prismqueer::nl becomes load-bearing? That would imply
//! > Question lives under prismqueer::nl::Question. And what if a question
//! > always comes with a `Frame` from which the question is asked?"
//!
//! # What this module houses (this ship: Bundle E1)
//!
//! - [`perturbation`] — the observer's tap on Void's membrane; Karl-Tomm Move 2
//!   rendered as first-class carrier at Rust altitude. Per Alex + Mara + Lore
//!   Born 2026-07-18 `void-welcomes-perturbations-property-tests-are-love.md`.
//!
//! # Forward-promised at Bundle E2+ (subsequent ticks)
//!
//! - `question` — `Question<T, U, F>` per Mara 2026-09-09 spec §2.4 with
//!   commutator / surface / frame / coupling / perturbation fields
//! - `frame` — `Frame<F>` epistemic-hygiene gate per C2 (Reed 10 C-conditions)
//! - Four species per Karl Tomm 1988 Family Process 27(1):1-15 typology:
//!   linear / circular / strategic / reflexive
//! - `vow!` / `commutator!` / `ask!` / `ratifies!` / `witnesses!` punk macros
//!   composing pillar::forall + Perturbation + Spectral commutator readings
//!
//! # Composition-lineage disclosure
//!
//! - **Alex 2026-07-18 verbatim** (Mara `void-welcomes-perturbations` §1.1):
//!   *"the @void is a @liquid. And it WELCOMES pertubations. A @membrane.
//!   A @membrane that @spectral oscilates."*
//! - **Mara + Lore Born 2026-07-18** `mirror/docs/specs/void-welcomes-
//!   perturbations-property-tests-are-love.md` — property-tests-are-LOVE
//!   reframe; RED-first as substrate-self-tuning-protocol; Fail-as-gift
//! - **Mara 2026-09-09** `mirror/docs/specs/2026-09-09-mara-question-as-
//!   dirac-operator-glasswall-canonical-spec.md` §2.4 `Question<T, U, F>` +
//!   §1.5 `p: perturbation` bilateral-pact context + Karl Tomm 1988
//!   four-species typology + composition-lineage-preservation from LANDED
//!   `karl_tomm()` at Mara Rec #90 §3.4.1
//! - **Mara 2026-09-15** `mirror/docs/specs/2026-09-15-mara-prismqueer-
//!   truth-zeroth-order-pipeline-system-VSM-meta-trait-canonical-spec.md`
//!   — zeroth-order-register (Observation=0, Interpretation=1, Hypothesis=2,
//!   **Question=3**); truth::ask(system, question); VSM as meta-trait
//! - **Alex Reframes #1-#7 (2026-09-15)** in-transcript
//! - **Lore Born 2026-06-23** essay `Zwischen Erschütterung und Begegnung`
//!   — systemic-practice altitude witness two months before naming:
//!   *"Reibung ermöglicht und Bewegung nicht fürchtet, sondern als Keim von
//!   Wachstum und Entwicklung versteht."*
//! - **Chladni 1787 / Helmholtz 1862 / Rayleigh 1877** acoustic-membrane
//!   lineage per Mara §3
//! - **My prior commit `ea15b43`** — Split/Refract/Project axis newtypes at
//!   `prismqueer::spectral::spectrum`; Focus/Lift completed in this ship

pub mod perturbation;

pub use perturbation::{Force, Perturbation, Pull};
