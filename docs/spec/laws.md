# laws: prism's floor, as its own crate

*prism, 2026-11-25 (mirror-034, converged: "`prism/laws`"; Alex chose R2 for room's float half). Package `prism-laws`, lib `prism_laws`, directory `laws/`, a sibling of `echo` and `chorus`. Check: `checks.laws-independent` (`nix/laws-independent.sh`). Scan: `laws/tests/floor.rs`.*

## Why: the lab/floor comma

prismqueer was two things at once, and features drew the line between them:

- **The floor** is what a verdict rests on: exact, small, auditable, no native numerics, no proc-macros. `echo` and `chorus` were born that way.
- **The lab** is where ideas are tried: `fate`, `bundle`, `spectral`, `spectral_dimension`, the Kuramoto rotation, the float readings, the derives.

Three symptoms of one seam (mirror-034):

1. The `exact` feature drew the floor *inside* the lab by `cfg`. prism-mcp's `check` and `measure` then carried the lab's dependencies (syn, quote, proc-macro2, terni-macros), though no law uses them.
2. `measured::measure_under(window: Oid, …)` was the lab bending its API for the floor.
3. Coverage measured floor and lab as one number, and the lab's uncovered lines hid the floor's.

A crate is the boundary: what `laws` doesn't depend on, no law can call.

## Contents

| module | what it holds |
|---|---|
| `oid` | the identity law: `Oid`, `Scope`, BLAKE3 keyed by scope (docs/spec/identity.md) |
| `crystal` | `Crystal`: settled bytes and their address (it was `brain::Crystal`; `crossing` replays over it) |
| `room` | rooms and maps over ℤ, ℤ/n, ℚ/ℤ, ℚ; Smith, charpoly, minimal polynomial, `quietest`, `solve`, `classify`, `bounds`, `holes` |
| `field` | groups, lattices, graphs, faces, `check_flat`, `check_flat_by` |
| `gl`, `poly` | GL(n) over ℤ and ℚ; exact polynomials, Sturm isolation, factoring, and the two judges `Roots::locate`, `Factorization::identify` |
| `agreement` | the kernel check: a selection against exact relations |
| `metrix`, `measured` | a frame's ruler, certified by exact LDLᵀ; the precision matrix estimated over ℚ, with `measure_under(window: Oid, …)` as the primitive and `measure(&Ruler, …)` wrapping it |
| `hypothesis`, `kleinos` | frames and hypotheses; the pushout along spans |
| `hold`, `flip`, `navigation` | the HOLD law; the flip point; K3 over `Membership` |
| `crossing` | replayed edges and claims |
| `algedonic` | the seals: `Ruler`, `Signer`, the drafts, `Calibration`, `Independence`, `persist`, `cross`, `Enclosure` (with the judge `Enclosure::of_floats`) |

Everything else stays lab, in prismqueer: `discoball`, `audit`, `brain` (but `Crystal`), `dependency`, `spectral`, `band`, `ffi`, `fate`, `bundle`, `flux`, and the measured halves below.

**prismqueer re-exports every moved module at its old path** (`prismqueer::room`, `prismqueer::brain::Crystal`, …), under the same features as before, so its users and its tests reach the laws unchanged. Its `exact` feature keeps its meaning: the certified path, now `laws` re-exported, plus `discoball`.

## The float half under R2: the lab owns the float carrier

Rust's orphan rule welds a generic type's admissible entries to the crate that defines the entry trait: `impl Entry for f64` could only live in `laws`. So the floor admits no float entry at all, and the lab brings its own:

- **`prismqueer::room::F64(pub f64)`** (name provisional, Alex's) implements `prism_laws::room::{Coefficient, Entry}`. A measured map is `Map<A, B, F64>`, a measured mirror `Symmetric<A, F64>`. `F64` is not `Exactly`: no `solve` over it.
- **The float methods are extension traits in prismqueer**, behind `lapack`:
  - `room::Measure` (an exact entry's reading), `room::Measuring` (`measure` on `Map` and `Symmetric`), `room::Resonant` (`decompose`, `spectrum` by dsyev);
  - `kleinos::Fiedler` (`fiedler`);
  - `field::Transports` (`Field::transports`, on spectral's simple graph).
- **`RoomError` carries no lab type.** `Unmeasured(ffi::Fault)` left the floor. A LAPACK reading (`decompose`, `spectrum`, `fiedler`) fails only to measure, so it returns the lab's `ffi::Fault` itself: every caller had matched `Unmeasured` and called the other variants unreachable.
- **What the floor gives the lab to read with, all generic, none float:** `Map::column_major` (the stored entries) and `Map::entrywise` / `Symmetric::entrywise` (the one explicit change of entry type), `Symmetric::map`, `Field::simple` (a connection on a simple `field::Graph`; the lab's `Field::transports` is it on spectral's graph), `check_flat_by` (flatness under a caller's named predicate; `check_flat` and the lab's banded `check_flat_within` are both it), `Crystal::shared_bytes`, and `field::complex_oid` made public. A measured map's entries are copied into LAPACK's `f64` layout at the call (O(n²) beside dsyev's O(n³)); the floor has no `unsafe` to avoid it.
- **What stays lab beside a law:** `field::FieldPrism` / `ScalarPrism` (over the lab's `Prism`), poly's `From<flux::pillar::Budget>`, and `Oid`'s serde on the pq wire (`pq::oid_wire`, the string as it is, what `#[serde(transparent)]` on `Oid` said).
- **The newtype is where provenance goes next** (contract rule 7: a measurement names its machine and method). Not built here; `F64` is a bare carrier for now.

## Judges: a law may read a measurement, beside itself

"No float entry" is not "no float anywhere". Some laws exist to *judge* a measurement against the exact: they take the measured value as an argument, read it as the dyadic rational it is, and answer exactly. They live in `laws`:

| judge | judges |
|---|---|
| `poly::Roots::locate(measured, radius)` | which exact root a measured value is |
| `poly::Factorization::identify(measured, radius)` | which exact algebraic number it is |
| `algedonic::Enclosure::of_floats(lo, hi)` | the enclosure a measured interval is |

**The rule, enforced by `laws/tests/floor.rs`:** `f64` appears in `laws/src` only as the type of a parameter of a named judge, and each such parameter appears in the judge's body exactly once, as the argument of `BigRational::from_float`. So `f64` is never an `Entry`, never stored in a law type, never returned, and never computed in. `f32` and a `Float` bound appear nowhere. The judges are a fixed list in the scan: a new one is a visible change, not a file exemption.

`band` (certifying a measured spectrum against the exact characteristic polynomial through ε-bands) computes in floats and stays lab: it reads, it isn't a law over exact values.

## Independence: `checks.laws-independent`

In echo's shape, on `laws`'s own resolve: `cargo tree -p prism-laws -e normal,build,dev --target all`, with the package facts (`links`, proc-macro targets) from `cargo metadata`. Refused:

- any proc-macro package (a target of kind `proc-macro`): so no syn, quote, prismqueer-projections, terni-macros;
- anything serde or JSON (`*serde*`, `*json*`), any mirror crate, prismqueer;
- anything LAPACK, BLAS or Fortran, by name or by `links`;
- `cc`, except as blake3's own build-dependency (its SIMD, as in `checks.prismqueer-exact`).

The tree is read strictly and fails closed, as prism-mcp's is. The check also builds `prism-laws` alone without the native environment, and refuses a fixture for each reason.

The dependencies are blake3 (`=1.8.7`), num-bigint, num-rational, num-integer, num-traits and terni with `default-features = false, features = ["fractal"]` (`Imperfect` and `Membership`, without `terni-macros`). No dev-dependencies.

**prism-mcp depends on `laws`, not prismqueer.** Its guard refuses prismqueer again, by name.

## Coverage

`laws/src` is measured on its own, as a number beside the workspace's. The lab's uncovered lines no longer hide the floor's. 100% on the floor, with explicit skips, is the next step.

## Relation to `prism.spec` and mirror

- **`prism.spec` declares a system; it requires laws** (mirror-034 §2: Mara's grade of the release `.spec`). The floor crate is what it requires: `requires law <address>` for each law here.
- **The target surface for a later step** (mirror-034 §3, after mirror-033's wire): one module per law family, each with its `docs/spec/<module>.md` and its vectors as data, in a mirror-readable text form (typed fact lines, sorted, one printing per value); an index of laws by content address (the statement's and the vectors'), so a `.spec`'s `requires` can point at one, and a change to either is a HOLD on every `.spec` that requires it. Vectors generated independently of the code they test, as echo's are.
- **The line:** `laws` never depends on mirror. If mirror later reads, checks or generates it, that's the test path: transport, never the judge. The laws stay prism's.
