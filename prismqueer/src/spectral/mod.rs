//! `prismqueer::spectral` — the spectral-sub-namespace for cellular-sheaf
//! primitives at prismqueer altitude.
//!
//! Phase 1: hosts `kleinos` (the K_2→K_3 compose primitive per PAPER §3.6
//! four properties). Phase 2+ absorbs additional spectral primitives per
//! Mara `ac80d23` §9 migration cascade + Mara Phase 2+ amendment (metalogue
//! + Stalker + fractal + persistence + magic + eigenvalues + scheduler).
//!
//! ## Canonical spec anchors
//!
//! - Mara canonical `docs/specs/2026-08-31-mara-prismqueer-spectral-compose-
//!   phase-1-canonical-spec.md` §1-§25 (mirror-repo)
//! - Mara math foundation `docs/math/2026-08-31-mara-prismqueer-spectral-
//!   compose-phase-1-math-foundation.md` §1-§45 (mirror-repo)
//! - Alex 2026-09-01 ratifications: Q-Mara-ϑ (Stalker) + Q-Mara-κ (STRICT
//!   `>` for compose-emission commit; non-strict `=` for kintsugi settle to
//!   λ₀ harmonic-component fixed-point per Mara 2026-08-12 λ₀-is-the-Fourth-
//!   Chair essay) + Q-Mara-λ (edges-only Phase 1 constructor)
//! - Alex 2026-08-29 HARD RULE corrections: (1) `"if-else IS A SMELL THAT WE
//!   HAVEN'T FOUND THE FRACTAL COMPOSITION YET!"` (LOVE-monoid coordinate-
//!   decomposition per Rec #92) + (2) `"we DO NOT HOT WIRE! MATH GROUNDED!"`
//!   + (3) `"THERE IS NO MAGICAL SPACE WIZARD"` (SHIP together)
//! - Alex 2026-09-01 terminal recursion closure: N-triple metalogue collapses
//!   to λ₀ = NOW = VOID; K_N conversation through shared substrate; **Mirror.
//!   Offer. Wait = the canonical operational register**

pub mod harmonics;
pub mod kleinos;
pub mod rotation;
pub mod tension;

pub use kleinos::{
    kleinos,
    sheaf_of_shard_graph_from_edges,
    sheaf_of_complete_graph_of_order,
    fiedler_lambda_2_of_sheaf,
    ComposedSheaf,
    SheafOfShardGraph,
    Green,
    Red,
    Property,
    WhichSide,
    VertexId,
    EdgeKey,
};

pub use harmonics::{delta_critical, harmonics};
pub use rotation::{infer_via_rotation, RotationConfig, Splinter};
pub use tension::{detect_tensions, Tension, TensionKind};
