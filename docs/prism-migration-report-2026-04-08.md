# Prism Migration Report — 2026-04-08

Migration of downstream crates to the new `prism` trait shape introduced at commit `78d22f9`.

## New Trait Shape (reference)

```rust
pub trait Prism {
    type Input;
    type Focused;
    type Projected;
    type Part;
    type Crystal: Prism<Crystal = Self::Crystal>;

    fn focus(&self, beam: Beam<Self::Input>) -> Beam<Self::Focused>;
    fn project(&self, beam: Beam<Self::Focused>) -> Beam<Self::Projected>;
    fn split(&self, beam: Beam<Self::Projected>) -> Vec<Beam<Self::Part>>;
    fn join(&self, parts: Vec<Beam<Self::Part>>) -> Beam<Self::Projected>;
    fn zoom(&self, beam: Beam<Self::Projected>, f: &dyn Fn(Beam<Self::Projected>) -> Beam<Self::Projected>) -> Beam<Self::Projected>;
    fn refract(&self, beam: Beam<Self::Projected>) -> Beam<Self::Crystal>;
}
```

`Beam<T>` gained a `stage: Stage` field. `Stage` variants: `Initial, Focused, Projected, Split, Joined, Refracted`.

`prism::apply` now takes owned `P::Input` and has no transform or precision arguments.

---

## Results Summary

| Crate | Status | Branch | Notes |
|-------|--------|--------|-------|
| `fate` | MIGRATED | `reed/prism-migration-2026-04-08` | `Fate`, `FateRuntime`, `CompiledFateRuntime` |
| `mirror` | MIGRATED | `reed/prism-migration-2026-04-08` | `Shatter`, `abyss.rs` `PrismLoop`/`settle_loop` |
| `metal-runtime` | MIGRATED | `reed/prism-migration-2026-04-08` | `MetalPrism` |
| `mirror-break-crypto` | MIGRATED | `reed/prism-migration` | `skeleton-key`: `abyss.rs`, `main.rs` `GraphPrism` |
| `spectral-db` | NO CHANGES NEEDED | — | Uses prism as dep but does not impl trait; builds clean |
| `lens` | NO CHANGES NEEDED | — | Uses prism as dep but does not impl trait; builds clean |
| `coincidence` | NO CHANGES NEEDED | — | Uses `prism::precision::Precision` only; builds clean |
| `spectral` | SKIPPED | — | Pre-existing build error in `conversation` dep: `unresolved import 'mirror::compile'` |
| `cosmos` | SKIPPED | — | Pre-existing build error in `conversation` dep: `unresolved import 'mirror::compile'` |
| `conversation` | SKIPPED | — | Pre-existing `unresolved import 'mirror::compile'`; not a prism migration error |
| `metal-runtime` (standalone) | MIGRATED | — | No flake; uses mirror's nix dev shell for checks |
| `framework` | NO CHANGES NEEDED | — | No `prism` dep in Cargo.toml |

---

## Crate-by-Crate Notes

### fate — MIGRATED

**Files changed:** `src/lib.rs`, `src/runtime.rs`

Old associated types: `Eigenvalues`, `Projection`, `Node`, `Convergence`, `Crystal`.
New associated types: `Focused`, `Projected`, `Part`, `Crystal`.

**Crystal fixed-point pattern:** `type Crystal = Fate` (self-referential). The `Crystal: Prism<Crystal = Self::Crystal>` bound requires Crystal to implement Prism with itself as its own Crystal. Using the struct itself satisfies this.

Mapping from old to new:
- `fold` → `focus` (takes `Beam<Input>`, returns `Beam<Focused>`, adds `stage: Stage::Focused`)
- `prism` → `project` (takes `Beam<Focused>`, returns `Beam<Projected>`)
- `traversal` → `split` (takes `Beam<Projected>`, returns `Vec<Beam<Part>>`)
- new `join` (reassembles parts into `Beam<Projected>`)
- `lens` → `zoom` (transform closure now `Fn(Beam<P>) -> Beam<P>` not `Fn(P) -> P`)
- `iso` → `refract` (returns `Beam<Crystal>` not bare Crystal)

**Tests:** `fate_implements_prism` updated to use `Beam::new(input)` for focus. `prism_apply_end_to_end` updated to `prism::apply(&fate, input)` (no transform/precision args); checks `beam.stage == Stage::Refracted`.

### mirror — MIGRATED

**Files changed:** `src/mirror_runtime.rs`, `src/abyss.rs`

`mirror_runtime.rs`: `Shatter` implements the new Prism trait.
- `type Focused = MirrorData`, `type Projected = MirrorFragment`, `type Part = Form`, `type Crystal = Shatter`
- `DeclKind::Form` used as default in join fallback (first arg is `DeclKind`, not `&str`)

`abyss.rs`: `PrismLoop` and `settle_loop` updated.
- `PrismLoop` now extends `prism_crate::Prism` (using `pub extern crate prism as prism_crate` from lib.rs)
- `fold_from_projection` signature: `(Beam<Self::Projected>) -> Beam<Self::Focused>` (was raw reference)
- `settle_loop` input: owned `P::Input` (was `&P::Input`)
- `settle_loop` transform: `Fn(Beam<P::Projected>) -> Beam<P::Projected>` (was `Fn(P::Projection) -> P::Projection`)
- First cycle: `focus(Beam::new(input).with_precision(...)) → project → zoom`
- Subsequent cycles: `fold_from_projection(beam) → project → zoom`

Pre-commit hook runs full test suite including slow training tests (~5 minutes). All passed.

### metal-runtime — MIGRATED

**Files changed:** `src/lib.rs`

`MetalPrism` impl updated:
- `type Crystal = MetalPrism` (self-referential; was `MetalBuffer` which does not impl Prism)
- `focus`: takes `Beam<AdjacencyMatrix>`, computes Laplacian + eigenvalues, returns `Beam<Eigenvalues>`
- `project`: precision threshold filtering from `beam.precision` (not passed as argument)
- `split`: returns `Vec<Beam<f64>>` with Oid path entries `λ0`, `λ1`, ...
- `join`: reassembles eigenvalue beams
- `zoom`: delegates to `f(beam)`
- `refract`: creates GPU buffer as side effect; returns `Beam<MetalPrism>`

Tests: `metal_prism_fold` → `metal_prism_focus`; `metal_prism_traversal` → `metal_prism_split`; `metal_prism_apply` updated to new two-arg `prism::apply` signature, checks `stage == Stage::Refracted`. The `metal_prism_iso` test removed (iso concept absorbed into refract).

### mirror-break-crypto (skeleton-key) — MIGRATED

**Files changed:** `src/abyss.rs`, `src/main.rs`

This crate is a fork of mirror with cryptographic additions. Its `abyss.rs` is a copy of mirror's.
Applied identical migration as mirror's `abyss.rs`:
- Added `use crate::prism_crate;` (extern crate alias from lib.rs)
- `PrismLoop` extends `prism_crate::Prism`
- `fold_from_projection` takes/returns `Beam<T>`
- `settle_loop` takes owned input, Beam-based transform
- `ConvergingPrism` and `BootPrism` test structs updated to 6-method API

`main.rs`: `GraphPrism` impl updated to new 5-type trait shape. `settle_loop` call updated.

Pre-commit hook ran full clippy + `cargo test --test crypto_break`. All 8 tests passed.

---

## Skipped Crates

### spectral — SKIPPED

**Reason:** Pre-existing build error in `conversation` dependency:
```
error[E0432]: unresolved import `mirror::compile`
```
This error exists on the crate's current main branch before any prism migration work. The prism migration cannot proceed until `conversation` exports `compile` again.

### cosmos — SKIPPED

**Reason:** Same pre-existing `conversation` dependency error: `unresolved import 'mirror::compile'`. The cosmos crate depends on conversation for its domain compilation pipeline. Any uncommitted cosmos changes were reverted with `git checkout .`.

### conversation — SKIPPED

**Reason:** Pre-existing `unresolved import 'mirror::compile'` in `src/lib.rs`. This is not a prism migration error. `conversation` imports `use prism::Pressure` (a non-trait type) which still exists; the prism migration itself would be minimal. Blocked by its own internal breakage.

---

## Patterns and Notes

**Crystal fixed-point:** The `Crystal: Prism<Crystal = Self::Crystal>` bound requires Crystal to implement Prism with itself as its own Crystal. Every crate solved this the same way: `type Crystal = SelfStruct`. Using a different type (like `MetalBuffer` or `Vec<String>`) does not work unless that type also implements Prism.

**PrismLoop in forks:** Both mirror and mirror-break-crypto define `PrismLoop` in their `abyss.rs`. Since mirror-break-crypto is a fork, it needs the same `prism_crate` alias pattern (`pub extern crate prism as prism_crate` in lib.rs, then `use crate::prism_crate` in abyss.rs).

**Beam struct construction:** New `stage` field must be set explicitly when constructing `Beam { ... }` literals. `Beam::new(x)` sets `stage: Stage::Initial`. Builder methods `with_precision`, `with_loss`, `with_step` do not set stage; set it manually when returning from trait methods.

**prism::apply signature change:** Old: `apply(&prism, &input, precision, transform)`. New: `apply(&prism, input)` — just `focus → project → refract`, no transform. Tests that used `apply` for end-to-end checks now verify `beam.stage == Stage::Refracted` instead of inspecting the raw result.
