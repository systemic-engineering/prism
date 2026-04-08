# Connection Substrate — Design

**Date:** 2026-04-08
**Status:** Draft for review
**Related:** prism crate at commit `024627e` (main); Seam+Taut re-review sign-off at `seam-taut-rereview-2026-04-08.md`; insight document at `~/dev/systemic.engineering/practice/insights/cosmology/nested-bundles-and-the-runtime-unification.md`

## Goal

Introduce a first-class `Connection` type in the prism crate that tracks the algebraic structure of what a prism does to a beam, alongside the existing scalar `ShannonLoss`. The substrate is non-abelian-capable by construction: a `Connection` trait with `compose` (non-commutative in general) and `norm` (abelian scalar projection). The default impl (`ScalarConnection`) is abelian and preserves existing behavior. Non-abelian concrete impls can be added later when downstream use cases demand them.

This is the minimum faithful implementation of the bundle-tower framework's requirement that composite connections have non-additive curvature. The scalar loss stays as an observable, now derived from the Connection's norm projection. Test infrastructure witnesses that the substrate can genuinely hold a non-abelian impl.

## Context and recognition

The optics layer (merged at `024627e`) implements composition via `Compose<P1, P2>`, meta-prisms, and six classical optic kinds. `ShannonLoss` is a scalar `f64` newtype; composition of two prisms' losses is simple addition. This is structurally **abelian** — `compose(A, B).loss == compose(B, A).loss` always.

The bundle-tower recognition (from the nested-bundles insight document) makes this wrong in a specific, load-bearing way. When two bundle morphisms compose, the curvature of the composite is:

```
F(A ∘ B) = F(A) + F(B) + [ω(A), ω(B)]
```

where `[·, ·]` is the Lie bracket of the connection forms. For abelian structure groups, the bracket vanishes and composition is additive. For non-abelian, the bracket is non-zero and introduces a cross-term — and this cross-term is **what makes nesting load-bearing** in the physical system. Without it, the composite tower collapses into a flat list of independent scales with no inter-scale coupling.

The current prism crate expresses only the abelian case. This spec adds the substrate for the non-abelian case without committing to a specific non-abelian structure, so downstream code can introduce concrete impls (free monoid, free Lie algebra, SO(3) matrices, etc.) when the use case demands them.

**The payoff is not in Group A's runtime behavior.** It's in the capacity of the type system to hold the richer structure when later specs need it. Group A ships a substrate that's capable of non-abelian composition; when MirrorPrism, the ship's navigation, or the physical model actually demand a specific non-abelian impl, that impl lands cleanly on top of this substrate without a trait refactor.

## Scope

**In scope:**
- A new `Connection` trait in the prism crate
- A concrete `ScalarConnection` impl (abelian, the default)
- A new generic parameter `C: Connection = ScalarConnection` on `Beam<T>`
- A new associated type `Connection` on `Prism`, with the `Crystal` bound extended to require matching connections
- `Compose<P1, P2>` constraint requiring `P1::Connection = P2::Connection`
- `MetaPrism<P, G>` propagating inner prism's Connection
- Deletion of `Beam::with_loss`; all call sites migrated to `Beam::with_connection` patterns
- Migration of every existing `Prism` impl to declare `type Connection = ScalarConnection`
- Test-only infrastructure (`TestConnection`, `TagPrism`) in a private test module that witnesses non-abelian composition via string concatenation

**Out of scope:**
- Any concrete non-abelian production Connection impl (e.g., `FreeMonoidConnection`, `FreeLieConnection`, `So3Connection`) — these wait for a concrete use case
- Lie bracket structure / free Lie algebra machinery
- Runtime consumers of the Connection distinction (caching, content-addressing based on connection, replay, parallel transport to concrete rotations) — these wait for Group B or later
- Changes to any crate outside `prism/` — downstream crates continue to use `Beam<T>` with the default `ScalarConnection` parameter

## Architecture

### Layer 1 — The Connection trait and ScalarConnection

New file `src/connection.rs` promoted to the crate root (not under `optics/`) because `Beam` will depend on it and `Beam` is a foundational type.

```rust
//! Connection — the algebraic structure that tracks what a prism
//! did to a beam. Non-commutative in general; scalar in the default
//! case. The bridge between sub-Turing optic composition and the
//! non-abelian curvature of the bundle-tower framework.

use crate::ShannonLoss;

/// A connection tracks the accumulated "rotation" a prism has
/// performed on a beam, in the bundle-theoretic sense. For abelian
/// structure groups it reduces to a scalar (loss). For non-abelian
/// groups it carries genuine non-commutative structure.
///
/// # Laws
///
/// - **Identity:** `compose(Self::default(), c) ≡ c ≡ compose(c, Self::default())`
/// - **Associativity:** `compose(compose(a, b), c) ≡ compose(a, compose(b, c))`
/// - **Non-commutativity is allowed but not required.** For abelian impls
///   `compose(a, b) ≡ compose(b, a)`; for non-abelian impls they differ.
/// - **Norm is a semi-homomorphism:** `norm(compose(a, b)) ≤ norm(a) + norm(b)`
///   — the inequality allows for cross-terms that increase loss beyond
///   the sum of individual losses. Concrete impls may satisfy equality
///   (abelian case) or strict inequality.
pub trait Connection: Clone + Default + std::fmt::Debug + 'static {
    /// Compose two connections. For abelian impls, addition. For
    /// non-abelian, concatenation, bracket, matrix multiplication, etc.
    fn compose(self, other: Self) -> Self;

    /// The scalar norm. Returns information loss in bits. This is the
    /// abelian projection of the non-abelian structure — useful for
    /// downstream consumers that only care about "how much."
    fn norm(&self) -> ShannonLoss;
}

/// The default abelian connection: a single f64 representing scalar
/// loss. Composition is addition; norm is the value itself. This is
/// what every existing optic in the codebase implicitly uses.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScalarConnection {
    pub loss: f64,
}

impl ScalarConnection {
    pub fn new(loss: f64) -> Self {
        ScalarConnection { loss }
    }
}

impl Connection for ScalarConnection {
    fn compose(self, other: Self) -> Self {
        ScalarConnection { loss: self.loss + other.loss }
    }
    fn norm(&self) -> ShannonLoss {
        ShannonLoss::new(self.loss)
    }
}
```

**Unit test in the same file:**

```rust
#[test]
fn scalar_connection_norm_preserves_infinity() {
    let c = ScalarConnection::new(f64::INFINITY);
    assert!(c.norm().as_f64().is_infinite());
}
```

The `infinity-preservation` test is load-bearing for the OpticPrism refutation path — refutation-as-infinite-loss must survive the Connection substrate.

### Layer 2 — Prism trait changes

In `src/lib.rs`:

```rust
pub trait Prism {
    type Input;
    type Focused;
    type Projected;
    type Part;
    type Crystal: Prism<Crystal = Self::Crystal, Connection = Self::Connection>;
    type Connection: Connection;   // NEW

    fn focus(&self, beam: Beam<Self::Input, Self::Connection>) -> Beam<Self::Focused, Self::Connection>;
    fn project(&self, beam: Beam<Self::Focused, Self::Connection>) -> Beam<Self::Projected, Self::Connection>;
    fn split(&self, beam: Beam<Self::Projected, Self::Connection>) -> Vec<Beam<Self::Part, Self::Connection>>;
    fn zoom(
        &self,
        beam: Beam<Self::Projected, Self::Connection>,
        f: &dyn Fn(Beam<Self::Projected, Self::Connection>) -> Beam<Self::Projected, Self::Connection>,
    ) -> Beam<Self::Projected, Self::Connection>;
    fn refract(&self, beam: Beam<Self::Projected, Self::Connection>) -> Beam<Self::Crystal, Self::Connection>;
}
```

The `Crystal` bound gains `Connection = Self::Connection`, enforcing that a prism and its crystal speak the same Connection language. This is the type-level fixed-point for Connection analogous to the one for Crystal itself.

All five method signatures thread `Self::Connection` through the Beam generic parameter. No semantic change to what the methods do — `refract` still runs the pipeline, `split` still fans out, etc. The types just now carry connection information.

### Layer 3 — Beam generic parameter

In `src/beam.rs`:

```rust
#[derive(Clone, Debug)]
pub struct Beam<T, C: Connection = ScalarConnection> {
    pub result: T,
    pub path: Vec<Oid>,
    pub connection: C,
    pub loss: ShannonLoss,        // derived from connection.norm() on construction
    pub precision: Precision,
    pub recovered: Option<Recovery>,
    pub stage: Stage,
}
```

Default generic parameter `C = ScalarConnection` means every existing `Beam<T>` parses as `Beam<T, ScalarConnection>` without change.

**Constructor updates:**

```rust
impl<T, C: Connection> Beam<T, C> {
    /// Create a beam with a default (identity) connection.
    pub fn new(result: T) -> Self {
        let connection = C::default();
        let loss = connection.norm();
        Beam {
            result,
            path: Vec::new(),
            connection,
            loss,
            precision: Precision::new(1.0),
            recovered: None,
            stage: Stage::Initial,
        }
    }

    /// Create a beam with an explicit connection. Loss is derived from it.
    pub fn with_connection(result: T, connection: C) -> Self {
        let loss = connection.norm();
        Beam {
            result,
            path: Vec::new(),
            connection,
            loss,
            precision: Precision::new(1.0),
            recovered: None,
            stage: Stage::Initial,
        }
    }

    /// Replace the connection (and derived loss) on an existing beam.
    pub fn set_connection(mut self, connection: C) -> Self {
        self.loss = connection.norm();
        self.connection = connection;
        self
    }
}
```

The invariant `beam.loss == beam.connection.norm()` is guaranteed at every construction and update point.

**`with_loss` is deleted entirely.** No deprecation, no scalar-only variant, no backwards compatibility shim. Every call site migrates to `with_connection` or `set_connection`:

```rust
// OLD
Beam::new(result).with_loss(ShannonLoss::new(5.0))

// NEW
Beam::with_connection(result, ScalarConnection::new(5.0))
```

### Layer 4 — Migration cascade

**Prism impls** — each `impl Prism for X` in the codebase grows one line:

```rust
type Connection = ScalarConnection;
```

Known sites:

- `src/lib.rs` test module: `StringPrism`
- `src/optics/monoid.rs`: `IdPrism<T>`, `Compose<P1, P2>` (with where-clause extension), `CountPrism` (test-only), `MarkerPrism` (test-only)
- `src/optics/meta.rs`: `MetaPrism<P, G>` (propagates `P::Connection`), `WordsPrism` (test-only)
- `src/optics/iso.rs`: `Iso<A, B>`
- `src/optics/lens.rs`: `Lens<S, A>`
- `src/optics/traversal.rs`: `Traversal<A, B>`
- `src/optics/optic_prism.rs`: `OpticPrism<S, A>`
- `src/optics/setter.rs`: `Setter<S, A>`, `SetterCrystal<S, A>`
- `src/optics/fold.rs`: `Fold<S, A>`
- `src/optics/phantom_crystal.rs`: `PhantomCrystal<M>`
- `tests/optics_integration.rs`: any inline test prisms

Approximately 15 impl sites. Each gets one line.

**`Compose`'s where-clause extension:**

```rust
impl<P1, P2> Prism for Compose<P1, P2>
where
    P1: Prism,
    P2: Prism<Input = P1::Crystal, Connection = P1::Connection>,
    P1::Input: Clone,
{
    type Connection = P1::Connection;
    // ... existing associated types ...
}
```

**`MetaPrism`'s propagation:**

```rust
impl<P, G> Prism for MetaPrism<P, G>
where
    P: Prism + Clone + 'static,
    G: Gather<P::Part> + Clone + 'static,
    P::Projected: Clone + 'static,
    P::Part: Clone + 'static,
{
    type Connection = P::Connection;
    // ... existing associated types ...
}
```

**`with_loss` call site migration** — grep for `.with_loss(` and rewrite every site using the appropriate construction pattern. Sites include:

- `src/beam.rs` test module
- `src/optics/iso.rs` test module
- `src/optics/gather.rs` test assertions (MaxGather precision-sorting test)
- `src/optics/optic_prism.rs` focus impl (refutation path)
- `tests/optics_integration.rs`

Estimated 10-20 call sites. Each is a mechanical rewrite.

**Verification:** after migration, `grep -r "with_loss" src/ tests/` must return zero results in the prism crate (outside git history). This is a hard check in the final plan task.

### Layer 5 — Test infrastructure for non-abelian witness

Inside `#[cfg(test)] mod tests` in `src/optics/monoid.rs`:

```rust
#[cfg(test)]
mod connection_witness_tests {
    use super::*;
    use crate::{Beam, Connection, Prism, ShannonLoss, Stage};

    /// Minimal non-abelian Connection for test purposes.
    /// Composition is string concatenation: non-commutative by construction.
    /// Norm is string length in bits.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct TestConnection(String);

    impl Connection for TestConnection {
        fn compose(self, other: Self) -> Self {
            TestConnection(self.0 + &other.0)
        }
        fn norm(&self) -> ShannonLoss {
            ShannonLoss::new(self.0.len() as f64)
        }
    }

    /// Test prism that appends a tag to the Connection on refract.
    /// Self-crystal so it can compose with itself.
    #[derive(Clone)]
    struct TagPrism {
        tag: &'static str,
    }

    impl Prism for TagPrism {
        type Input = String;
        type Focused = String;
        type Projected = String;
        type Part = String;
        type Crystal = TagPrism;
        type Connection = TestConnection;

        // focus/project/split/zoom: pass-through stage transitions
        // (details in the plan)

        fn refract(
            &self,
            beam: Beam<String, TestConnection>,
        ) -> Beam<TagPrism, TestConnection> {
            let new_connection = beam.connection
                .compose(TestConnection(self.tag.to_string()));
            // Construct output with path/precision/recovered inherited
            // and new connection.
            // (Exact construction pattern in plan; may use set_connection
            // on a newly-built Beam or a custom helper.)
        }
    }

    #[test]
    fn non_abelian_connection_distinguishes_compose_orders() {
        let a = TagPrism { tag: "a" };
        let b = TagPrism { tag: "b" };

        let ab = Compose::new(a.clone(), b.clone());
        let ba = Compose::new(b, a);

        let out_ab = crate::apply(&ab, "input".to_string());
        let out_ba = crate::apply(&ba, "input".to_string());

        // Non-abelian at the Connection level
        assert_eq!(out_ab.connection, TestConnection("ab".to_string()));
        assert_eq!(out_ba.connection, TestConnection("ba".to_string()));
        assert_ne!(out_ab.connection, out_ba.connection);

        // Abelian at the scalar norm projection
        assert_eq!(out_ab.loss, out_ba.loss);
    }

    #[test]
    fn connection_accumulates_around_a_type_level_loop() {
        let a = TagPrism { tag: "a" };
        let b = TagPrism { tag: "b" };

        let abab = Compose::new(
            Compose::new(
                Compose::new(a.clone(), b.clone()),
                a.clone(),
            ),
            b,
        );

        let out = crate::apply(&abab, "input".to_string());
        assert_eq!(out.connection, TestConnection("abab".to_string()));
        assert_eq!(out.loss.as_f64(), 4.0);
    }
}
```

Total: ~120 lines of test-only code. Two tests. One witnesses non-commutativity (different connections, same loss). One witnesses loop accumulation (type-level closed loop, connection still grows linearly).

## Data flow

The pipeline `focus → project → split → zoom → refract` is semantically unchanged. The only new data flow is that Connection is carried through every beam in the pipeline. For `ScalarConnection` (the default), connection values are scalars that add on compose — behaviorally identical to the current `ShannonLoss`-only pipeline. For non-abelian impls (introduced later), connection values carry real algebraic structure and composition respects their non-commutative laws.

The cross-term that the bundle tower requires is NOT computed in `Compose::refract`. It lives in the `Connection::compose` method of the concrete impl. `ScalarConnection::compose` is pure addition; no cross-term. Future impls can introduce real cross-terms by implementing `compose` with non-additive behavior (concatenation, bracket, matrix multiplication).

## Error handling

No new error types. Failure continues to be encoded as `ShannonLoss::new(f64::INFINITY)`, which under the Connection substrate means `ScalarConnection::new(f64::INFINITY)`. The OpticPrism refutation path is the primary consumer of this pattern. `ScalarConnection::norm()` is required to handle infinity correctly (unit-tested in Layer 1).

## Testing strategy

Three categories:

1. **Substrate unit tests** — new tests for `ScalarConnection`'s laws (identity, associativity, norm preservation including infinity). In `src/connection.rs`.

2. **Regression floor** — the existing 139-test suite must continue to pass after the migration. No test is deleted; every test is expected to pass unchanged (except for the grep-migration of `with_loss` call sites, which are mechanical rewrites of test bodies).

3. **Non-abelian witness tests** — the two tests in Layer 5 (`non_abelian_connection_distinguishes_compose_orders`, `connection_accumulates_around_a_type_level_loop`). These witness the substrate's capacity to hold non-abelian structure without relying on any production non-abelian impl.

Total test count after the change: 139 (existing) + 2 (witnesses) + ~3 (substrate unit tests) = 144.

## Open questions

1. **Does Rust stabilize defaulted associated types in 1.93?** If yes, `type Connection: Connection = ScalarConnection;` in the trait definition means existing Prism impls don't need to add the line. If no, every impl must add `type Connection = ScalarConnection;` explicitly (mechanical, ~15 sites). The plan should check and pick the right path.

2. **Does the `Crystal: Prism<Connection = Self::Connection>` bound require any explicit `where` clauses at use sites?** The trait definition enforces it, but downstream generics that bound on `Prism` may or may not propagate it automatically. The plan should include a cargo-check step after the trait change to catch any missing `where` clauses.

3. **What name for the constructor that explicitly takes a Connection?** `with_connection` conflicts with the verb-builder pattern Rust usually uses for the fluent API. `with_connection` here is a constructor, not a builder. Alternatives: `from_connection`, `with_initial_connection`. Going with `with_connection` for readability; the plan may adjust.

4. **Does `set_connection` need to also reset `stage` or preserve it?** Preserving makes it a pure metadata update; resetting feels wrong because it'd mean "set_connection is a stage transition" which isn't its purpose. Going with preserve.

## Non-goals

- A production non-abelian Connection impl
- Lie bracket machinery
- Downstream consumers of the Connection distinction (caching, content-addressing, replay, transport)
- Changes to any crate outside prism

## Migration and rollout

Sequential, each step atomic and commit-sized:

1. Add `src/connection.rs` with trait + ScalarConnection + unit tests
2. Re-export `Connection, ScalarConnection` from `src/lib.rs`
3. Add generic parameter to `Beam` with default; add `with_connection` and `set_connection` constructors
4. Delete `with_loss` method (will break downstream sites, intentional)
5. Add `type Connection` to `Prism` trait with Crystal bound extension
6. Update method signatures on `Prism` trait to use generic Beam
7. Update `Compose` where-clause and `type Connection = P1::Connection;`
8. Update `MetaPrism` where-clause and `type Connection = P::Connection;`
9. Update each of the ~15 existing Prism impls with `type Connection = ScalarConnection;` (batched or per-file)
10. Grep for `.with_loss(` and migrate each call site
11. Verify `grep -r "with_loss" src/ tests/` returns zero
12. Add TestConnection, TagPrism, and the two witness tests in `src/optics/monoid.rs`
13. Run full test suite; confirm 144 tests passing
14. Documentation: update module-level doc in `src/lib.rs` to mention Connection

Each step has its own commit with TDD phase markers where applicable. The plan will decompose further into bite-sized tasks.

## What this enables

Once landed, the following become expressible (in future specs or downstream crates):

- **Non-abelian production impls:** `FreeMonoidConnection`, `FreeLieConnection`, `So3Connection`, etc. Each is a ~50-line struct + Connection impl + registration with the prisms that use it.
- **Content-addressing based on Connection:** a Beam's identity can include its Connection, making `compose(A, B)` and `compose(B, A)` genuinely distinct in the content-addressed registry.
- **Replay:** a Beam's Connection is a compressed trace of what happened; given the Connection and the original input, you can reconstruct the computation.
- **Parallel transport to concrete targets:** a representation homomorphism from the abstract Connection to a concrete algebra (matrices, quaternions) gives real physical meaning to the accumulated structure.
- **Cross-term introduction:** specific prisms can implement `Connection::compose` with non-additive cross-terms, making their composition genuinely non-abelian without any other code change.

None of these are this spec's responsibility. They're future specs built on top.
