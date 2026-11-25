# features: the exact path, and the readings on top of it

*prism, 2026-11-25 (prism-025 Response §1 and its Log: "the `exact` feature split, which unlocks `check` and `measure` as tools"). The name `exact` is provisional; Alex names it. Code: `prismqueer/Cargo.toml` `[features]`, `prismqueer/src/lib.rs`, and `src/<module>/reading.rs` for each measured half. Tests: `prismqueer/tests/exact.rs`. Check: `checks.prismqueer-exact` (`nix/prismqueer-exact.sh`).*

## Why

Contract rule 7: the certified path is exact. Until now the exact modules (`agreement`, `gl`, `metrix`, `measured`, `hypothesis`, `kleinos`, room's exact half) sat behind `lapack`, because `room` and `field` were gated as a whole and five of room's call sites use `ffi` for eigen *readings*. So `prism-mcp` couldn't offer `check` or `measure` without linking LAPACK, and with it the residue in `lapack-failures.md`, into the tool meant to be the floor.

The split lets the type system build the certified path without Fortran: under `exact` the float readings don't exist, so nothing exact can call one.

## The features

| feature | implies | what it adds | native step |
|---|---|---|---|
| (none) | | the default build: `algedonic`, `hold`, `navigation`, `flip`, `poly`, `audit`, `oid`, … | none |
| `exact` | | the certified path over ℤ and ℚ (below) | none: no `cc` of its own, no LAPACK/BLAS/Fortran, build.rs writes Rust only |
| `lapack` | `exact` | the measured readings: `ffi`, `band`, `spectral`, `nl`, room's eigen readings, and everything built on them | gfortran, LAPACK, BLAS (build.rs, under `CARGO_FEATURE_LAPACK` only) |
| `bundle` | `optics` | `bundle`, `flux` (and `flux::pillar`, the property harness several exact test files use) | none |
| `fate` | `bundle` | `fate` | none |

`lapack = ["exact", "dep:cc"]`, so every build that had `lapack` has everything it had, at the same paths.

## The map: module → feature

**`exact`**, each module whole:

| module | exact half (`exact`) | measured half (`lapack`, `src/<module>/reading.rs`) |
|---|---|---|
| `room` | rooms, maps over ℤ, ℤ/n, ℚ/ℤ, ℚ; Smith, charpoly, minimal polynomial, `quietest`, `solve`, `classify`, `bounds`, `holes` | `Measure` (the one step to `f64`), `Coefficient`/`Entry` for `f64`, `Map::measure`, `Symmetric::measure`, `Symmetric<_, f64>::{decompose, spectrum}`, `Modes`, the crate-private Frobenius norm and singular values; `RoomError::Unmeasured` (an `ffi::Fault`) is cfg'd in place |
| `field` | groups, lattices, graphs, faces, `check_flat`, `VertexId`/`EdgeKey` (now defined here; `spectral::join` re-exports them) | `Measured`, `Log2`, `check_flat_within` (a `Noise` band); `Field::transports` is cfg'd in place (it takes `spectral::join`'s simple graph) |
| `poly` | already ungated: in every build | `Roots::locate`, `Factorization::identify` (a float read against exact roots) |
| `kleinos` | the pushout, spans, commas | `Pushout::fiedler`, `FIEDLER_METHOD` (λ₂ by dsyev, a `band::Measurement`) |
| `gl`, `agreement`, `metrix`, `measured`, `hypothesis`, `discoball` | whole | none |

**`lapack`** (unchanged): `ffi`, `spectral`, `nl`, `band`, `music`, `shard`, `observer`, `chaos`, `crystal_shard`, `observation`, `model`, `assertion`, `reality`, `question`, `choice`, `recursion`, `spectral_dimension`.

- `band` is float throughout (`f64::EPSILON` bands, `Measurement { value: f64 }`). It stays `lapack`.
- `chaos` and `question` have no float in their own source and don't call `ffi`, but `chaos` is terni's `Loss` under another name, which is float underneath. They stay `lapack`: exact underneath is the bar, and they aren't.
- The shard family (`shard`, `crystal_shard`, `model`, `reality`, `choice`, `observation`, `assertion`, `observer`, `recursion`) is built on `spectral`. It stays `lapack`.

## What "exact" means here

- **No float in an exact module.** `tests/exact.rs` scans the source of every exact module (`room`, `field`, `gl`, `poly`, `agreement`, `metrix`, `measured`, `hypothesis`, `kleinos`, `discoball`) for `f64` and `f32` outside comment lines (`to_f64` counts). The measured halves are separate files, `src/<module>/reading.rs`, so the scan needs no cfg parsing: what's in the file is what `exact` compiles. Every `mod reading;` must sit under `#[cfg(feature = "lapack")]`, and every exact module must be declared under `exact` in lib.rs (poly under no feature).
- **No `ffi` reachable.** Under `exact` alone the module `ffi` doesn't exist, so a call to it is a compile error, not a runtime path. `checks.prismqueer-exact` compiles that build.
- **No native step.** build.rs runs in every build, but its Fortran/LAPACK branch only under `CARGO_FEATURE_LAPACK`; the rest generates Rust from Brainfuck sources. The check builds without gfortran, LAPACK or BLAS in the environment.

## The one `cc` that stays: blake3's

blake3 `=1.8.7` has `cc` as an unconditional build-dependency, and its build.rs compiles its own SIMD (NEON C intrinsics on aarch64, assembly on x86-64) unless its `pure` feature is on. Every Oid is BLAKE3, so `exact` can't drop it, and echo, chorus and prism-mcp carry the same `cc` today. It is not a numeric backend and has no `links`.

So the check refuses `cc` reached by any path but blake3's own edge: prismqueer's (what `lapack` adds), or any other crate's. **For Alex:** `blake3/pure` would remove the C compile too (the `cc` crate would still build, unused), at a hashing-speed cost, across the whole workspace through feature unification. That's not done here.

## `checks.prismqueer-exact`

1. `cargo metadata --no-default-features --features prismqueer/exact` (the resolve follows the features asked for), walked from prismqueer by `nix/independent.sh` with `<dev> = none`: normal and build edges at every depth, the library's own closure. prismqueer's dev-dependencies (criterion, jsonschema, serde_json, syn) build its tests and are never linked.
2. Forbidden: `cc` (blake3's edge set aside first), anything LAPACK, BLAS or Fortran by name, and any `links` naming lapack, blas or fortran.
3. Red: the same metadata under `prismqueer/lapack` must be refused (cc), and four fixtures: `exact-cc` (prismqueer's own build edge to cc), `exact-transitive` (cc through another crate), `exact-links` (`links = "lapack"`), `exact-fortran` (a Fortran runtime by name). `exact-clean` (blake3's cc, plus a dev-only cc and openblas-src) must pass.
4. `cargo build` and `cargo test -p prismqueer --no-default-features --features exact`, with no native environment.

## Tests by feature

- Each test file is gated on what it uses. Under `exact` (with `bundle` where it uses `flux::pillar`): `agreement`, `discoball`, `field_multiplicity`, `field_prism`, `hypothesis`, `kleinos`, `measured`, `metrix`, `room_faces`, `room_holes`, `room_rings`, `exact`. One test in `kleinos` (λ₂) and one in `field_multiplicity` (`Field::transports`) stay `lapack`, as do the float tests of `poly` and `poly_factor`.
- Still `lapack`: `field`, `field_faces`, `room`, `one_graph`, `hodge`, `steer`, `window`, `instrument`, `music`, `band`, `loss_laws`, `rotation_splinter`, the spectral-compose battery, `autopoietic_loop_integration`, `lapack_faults`, `build_hygiene`. Each uses `spectral`, `band`, `Log2`/`Noise` or a measured reading.
- `tests/exact.rs` holds the scan (every build) and an end-to-end path under `exact` alone: agreement `In`/`Out`, measured → metrix sealed by its window, a Form's inertia, the triangle's charpoly.

## For prism-mcp

*Done: prism-mcp takes prismqueer under `exact`, and its guard reads its own resolve (`docs/spec/mcp.md` §6.1). The notes below are the plan as it stood.*

- To offer `check` and `measure`, prism-mcp depends on `prismqueer = { path = "../prismqueer", default-features = false, features = ["exact"] }`.
- Its independence check refuses `prismqueer` by name today. It would allow prismqueer exactly under that feature set: the walk from prism-mcp stays, and prismqueer's own closure is held by `checks.prismqueer-exact`.
- **One snag, found here:** `cargo metadata` resolves the workspace's features as one union, dev-dependencies included. prismqueer's dev-dependency `terni = { features = ["serde"] }` turns on terni's `serde` in that union, so the graph shows a *normal* edge terni → serde (and serde_derive) under prismqueer, even under `exact`. A real `cargo build -p prism-mcp` never builds prismqueer's dev-dependencies, so terni comes without serde there; but prism-mcp's guard, reading the union, would refuse serde through prismqueer → terni. Before prism-mcp takes prismqueer, either the guard reads a per-package resolve (e.g. `cargo tree -p prism-mcp -e normal,build` beside the metadata's `links`), or prismqueer's tests reach terni's serde another way. `checks.prismqueer-exact` doesn't forbid serde, so it isn't affected.
- The cost: prismqueer brings in a proc-macro crate (prismqueer-projections, syn) and the num-* stack. That's for Alex and Mara to weigh against "the floor".
