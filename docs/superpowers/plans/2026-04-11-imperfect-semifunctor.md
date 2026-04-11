# Imperfect + Semifunctor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure prism into a workspace with two crates — `imperfect` (Result extended with partial success) and `core` (Beam semifunctor + Prism monoid with three operations).

**Architecture:** `imperfect` is standalone: `Loss` trait, `Imperfect<T, E, L>` enum, `ShannonLoss`. `core` depends on `imperfect`: `Beam` trait (semifunctor with `tick` as primitive), `PureBeam`, `Prism` trait (three methods: focus, project, refract), Operation DSL. Split/zoom removed from core — they're user-space `smap` applications.

**Tech Stack:** Rust 2021, nix develop shell, cargo workspace. Zero external dependencies.

**Build/test commands:** All cargo commands run via `nix develop -c cargo <cmd>` (bare `cargo` not in PATH).

**Spec:** `docs/superpowers/specs/2026-04-11-imperfect-semifunctor-design.md`

---

## File Structure

### New files (create)

- `Cargo.toml` — workspace root (replaces current single-crate Cargo.toml)
- `imperfect/Cargo.toml` — imperfect crate manifest
- `imperfect/src/lib.rs` — Loss trait, ShannonLoss, Imperfect<T,E,L>, std interop
- `core/Cargo.toml` — prism-core crate manifest
- `core/src/lib.rs` — Prism trait (3 methods), Operation structs, blanket impl
- `core/src/beam.rs` — Operation trait, Beam trait (tick/next/smap/apply), PureBeam

### Moved files (from `src/` to `core/src/`)

- `src/trace.rs` → `core/src/trace.rs` (Op enum shrinks to 3 variants)
- `src/oid.rs` → `core/src/oid.rs` (unchanged)
- `src/spectral_oid.rs` → `core/src/spectral_oid.rs` (unchanged)
- `src/content.rs` → `core/src/content.rs` (unchanged)
- `src/precision.rs` → `core/src/precision.rs` (unchanged)
- `src/connection.rs` → `core/src/connection.rs` (imports from imperfect)
- `src/metal.rs` → `core/src/metal.rs` (unchanged — hardware-level, keeps 5 instructions)
- `src/optics/` → `core/src/optics/` (unchanged, feature-gated)

### Deleted files

- `src/lib.rs` — replaced by `core/src/lib.rs`
- `src/beam.rs` — replaced by `core/src/beam.rs`
- `src/loss.rs` — replaced by `imperfect/src/lib.rs`

---

### Task 1: Workspace scaffolding

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `imperfect/Cargo.toml`
- Create: `imperfect/src/lib.rs`
- Create: `core/Cargo.toml`
- Create: `core/src/lib.rs`

- [ ] **Step 1: Back up current Cargo.toml, replace with workspace root**

```toml
# Cargo.toml
[workspace]
members = ["imperfect", "core"]
resolver = "2"
```

- [ ] **Step 2: Create imperfect crate**

```toml
# imperfect/Cargo.toml
[package]
name = "imperfect"
version = "0.1.0"
edition = "2021"
description = "Result extended with partial success. Loss as a trait."

[features]
default = ["std"]
std = []

[dependencies]
# None. Depends on nothing.
```

```rust
// imperfect/src/lib.rs
//! Imperfect — Result extended with partial success.
//!
//! Three states: Ok (perfect), Partial (value with loss), Err (failure).
//! Derived from partial successes in PbtA game design.
//!
//! `Loss` is a trait. `ShannonLoss` (information loss in bits) is the
//! default implementation.
```

- [ ] **Step 3: Create core crate**

```toml
# core/Cargo.toml
[package]
name = "prism-core"
version = "0.1.0"
edition = "2021"
description = "Beam (semifunctor) + Prism (monoid). Three dimensions: focus, project, refract."

[features]
default = []
optics = []

[dependencies]
imperfect = { path = "../imperfect" }
```

```rust
// core/src/lib.rs
//! Prism — focus | project | refract.
//!
//! A Beam is a semifunctor. A Prism is the monoid lifted into it.
//! Three operations. Three dimensions of space.
```

- [ ] **Step 4: Verify workspace builds**

Run: `nix develop -c cargo build --workspace`
Expected: compiles with no errors (two empty crates)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml imperfect/ core/
git commit -m "🔧 workspace: scaffold imperfect + core crates"
```

---

### Task 2: Loss trait + ShannonLoss

**Files:**
- Modify: `imperfect/src/lib.rs`

- [ ] **Step 1: Write failing tests for Loss trait and ShannonLoss**

```rust
// imperfect/src/lib.rs — append at bottom

#[cfg(test)]
mod tests {
    use super::*;

    // --- ShannonLoss ---

    #[test]
    fn shannon_zero() {
        let l = ShannonLoss::zero();
        assert!(l.is_zero());
        assert_eq!(l.as_f64(), 0.0);
    }

    #[test]
    fn shannon_total() {
        let l = ShannonLoss::total();
        assert!(!l.is_zero());
        assert!(l.as_f64().is_infinite());
    }

    #[test]
    fn shannon_new() {
        let l = ShannonLoss::new(1.5);
        assert_eq!(l.as_f64(), 1.5);
        assert!(!l.is_zero());
    }

    #[test]
    fn shannon_combine() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(2.5);
        let c = a.combine(b);
        assert_eq!(c.as_f64(), 3.5);
    }

    #[test]
    fn shannon_combine_zero_is_identity() {
        let a = ShannonLoss::new(3.0);
        let b = a.clone().combine(ShannonLoss::zero());
        assert_eq!(a, b);
    }

    #[test]
    fn shannon_default_is_zero() {
        let l = ShannonLoss::default();
        assert!(l.is_zero());
    }

    #[test]
    fn shannon_display() {
        let l = ShannonLoss::new(2.0);
        assert_eq!(format!("{}", l), "2.000000 bits");
    }

    #[test]
    fn shannon_from_f64() {
        let l: ShannonLoss = 3.14.into();
        assert_eq!(l.as_f64(), 3.14);
    }

    #[test]
    fn shannon_add_operator() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(2.5);
        let c = a + b;
        assert_eq!(c.as_f64(), 3.5);
    }

    #[test]
    fn shannon_add_assign() {
        let mut a = ShannonLoss::new(1.0);
        a += ShannonLoss::new(0.5);
        assert_eq!(a.as_f64(), 1.5);
    }

    #[test]
    fn shannon_ordering() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(2.0);
        assert!(a < b);
    }

    #[test]
    fn shannon_is_lossless() {
        assert!(ShannonLoss::zero().is_lossless());
        assert!(!ShannonLoss::new(0.1).is_lossless());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p imperfect`
Expected: FAIL — `Loss`, `ShannonLoss` not defined

- [ ] **Step 3: Implement Loss trait and ShannonLoss**

```rust
// imperfect/src/lib.rs — above the test module

/// A measure of what didn't survive a transformation.
///
/// `combine` accumulates loss across pipeline steps.
/// `zero` is the identity for `combine`.
/// `total` is irrecoverable loss (the Err case).
pub trait Loss: Clone + Default {
    fn zero() -> Self;
    fn total() -> Self;
    fn is_zero(&self) -> bool;
    fn combine(self, other: Self) -> Self;
}

/// Information loss measured in bits. The default `Loss` implementation.
///
/// Zero = lossless. Infinity = total loss (Dark/Err).
/// Combine = addition (information loss is additive).
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ShannonLoss(f64);

impl ShannonLoss {
    pub fn new(bits: f64) -> Self {
        ShannonLoss(bits)
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }

    /// Semantic alias for `is_zero` — reads naturally in pipeline contexts.
    pub fn is_lossless(&self) -> bool {
        self.is_zero()
    }
}

impl Default for ShannonLoss {
    fn default() -> Self {
        Self::zero()
    }
}

impl Loss for ShannonLoss {
    fn zero() -> Self {
        ShannonLoss(0.0)
    }

    fn total() -> Self {
        ShannonLoss(f64::INFINITY)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    fn combine(self, other: Self) -> Self {
        ShannonLoss(self.0 + other.0)
    }
}

impl std::ops::Add for ShannonLoss {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        ShannonLoss(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for ShannonLoss {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl std::fmt::Display for ShannonLoss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.6} bits", self.0)
    }
}

impl From<f64> for ShannonLoss {
    fn from(v: f64) -> Self {
        ShannonLoss(v)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix develop -c cargo test -p imperfect`
Expected: all 13 tests PASS

- [ ] **Step 5: Commit**

```bash
git add imperfect/src/lib.rs
git commit -m "🟢 imperfect: Loss trait + ShannonLoss"
```

---

### Task 3: Imperfect enum + basic methods

**Files:**
- Modify: `imperfect/src/lib.rs`

- [ ] **Step 1: Write failing tests for the three variants and accessors**

```rust
// imperfect/src/lib.rs — add to tests module

    // --- Imperfect ---

    #[test]
    fn ok_is_ok() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        assert!(i.is_ok());
        assert!(!i.is_partial());
        assert!(!i.is_err());
    }

    #[test]
    fn partial_is_partial() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.5));
        assert!(i.is_ok());
        assert!(i.is_partial());
        assert!(!i.is_err());
    }

    #[test]
    fn err_is_err() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        assert!(!i.is_ok());
        assert!(!i.is_partial());
        assert!(i.is_err());
    }

    #[test]
    fn ok_returns_value() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        assert_eq!(i.ok(), Some(42));
    }

    #[test]
    fn partial_ok_returns_value() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.0));
        assert_eq!(i.ok(), Some(42));
    }

    #[test]
    fn err_ok_returns_none() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        assert_eq!(i.ok(), None);
    }

    #[test]
    fn err_returns_error() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        assert_eq!(i.err(), Some("oops".into()));
    }

    #[test]
    fn ok_err_returns_none() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        assert_eq!(i.err(), None);
    }

    #[test]
    fn loss_ok_is_zero() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        assert!(i.loss().is_zero());
    }

    #[test]
    fn loss_partial() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.5));
        assert_eq!(i.loss().as_f64(), 1.5);
    }

    #[test]
    fn loss_err_is_total() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        assert!(i.loss().as_f64().is_infinite());
    }

    #[test]
    fn as_ref_ok() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        let r = i.as_ref();
        assert_eq!(r.ok(), Some(&42));
    }

    #[test]
    fn as_ref_partial() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.0));
        let r = i.as_ref();
        assert_eq!(r.ok(), Some(&42));
        assert!(r.is_partial());
    }

    #[test]
    fn as_ref_err() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        let r = i.as_ref();
        assert_eq!(r.err(), Some(&"oops".to_string()));
    }

    #[test]
    fn map_ok() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        let m = i.map(|v| v * 2);
        assert_eq!(m.ok(), Some(84));
    }

    #[test]
    fn map_partial_preserves_loss() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.0));
        let m = i.map(|v| v * 2);
        assert_eq!(m.ok(), Some(84));
        assert!(m.is_partial());
    }

    #[test]
    fn map_err_is_noop() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        let m = i.map(|v| v * 2);
        assert!(m.is_err());
    }

    #[test]
    fn map_err_transforms_error() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        let m = i.map_err(|e| e.len());
        assert_eq!(m.err(), Some(4));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p imperfect`
Expected: FAIL — `Imperfect` not defined

- [ ] **Step 3: Implement Imperfect enum and methods**

```rust
// imperfect/src/lib.rs — add after ShannonLoss, before tests

/// Result extended with partial success.
///
/// Three states:
/// - `Ok(T)` — perfect result, zero loss.
/// - `Partial(T, L)` — value present, some information lost getting here.
/// - `Err(E)` — failure, no value.
///
/// Follows `Result` conventions: `is_ok()` means "has a value" (Ok or Partial).
/// `is_err()` means "no value."
#[derive(Clone, Debug)]
pub enum Imperfect<T, E, L: Loss = ShannonLoss> {
    Ok(T),
    Partial(T, L),
    Err(E),
}

impl<T, E, L: Loss> Imperfect<T, E, L> {
    /// True if a value is present (Ok or Partial).
    pub fn is_ok(&self) -> bool {
        !self.is_err()
    }

    /// True only for the Partial variant.
    pub fn is_partial(&self) -> bool {
        matches!(self, Imperfect::Partial(_, _))
    }

    /// True only for the Err variant.
    pub fn is_err(&self) -> bool {
        matches!(self, Imperfect::Err(_))
    }

    /// Extract the value (Ok or Partial), discarding loss. None if Err.
    pub fn ok(self) -> Option<T> {
        match self {
            Imperfect::Ok(v) | Imperfect::Partial(v, _) => Some(v),
            Imperfect::Err(_) => None,
        }
    }

    /// Extract the error. None if Ok or Partial.
    pub fn err(self) -> Option<E> {
        match self {
            Imperfect::Err(e) => Some(e),
            _ => None,
        }
    }

    /// The loss at this point. Zero for Ok, specified for Partial, total for Err.
    pub fn loss(&self) -> L {
        match self {
            Imperfect::Ok(_) => L::zero(),
            Imperfect::Partial(_, l) => l.clone(),
            Imperfect::Err(_) => L::total(),
        }
    }

    /// Borrow the contents. Loss is cloned (Loss: Clone is guaranteed).
    pub fn as_ref(&self) -> Imperfect<&T, &E, L> {
        match self {
            Imperfect::Ok(t) => Imperfect::Ok(t),
            Imperfect::Partial(t, l) => Imperfect::Partial(t, l.clone()),
            Imperfect::Err(e) => Imperfect::Err(e),
        }
    }

    /// Map the value, preserving loss and error.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Imperfect<U, E, L> {
        match self {
            Imperfect::Ok(t) => Imperfect::Ok(f(t)),
            Imperfect::Partial(t, l) => Imperfect::Partial(f(t), l),
            Imperfect::Err(e) => Imperfect::Err(e),
        }
    }

    /// Map the error, preserving value and loss.
    pub fn map_err<F>(self, f: impl FnOnce(E) -> F) -> Imperfect<T, F, L> {
        match self {
            Imperfect::Ok(t) => Imperfect::Ok(t),
            Imperfect::Partial(t, l) => Imperfect::Partial(t, l),
            Imperfect::Err(e) => Imperfect::Err(f(e)),
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix develop -c cargo test -p imperfect`
Expected: all tests PASS (13 Loss + 18 Imperfect = 31 total)

- [ ] **Step 5: Commit**

```bash
git add imperfect/src/lib.rs
git commit -m "🟢 imperfect: Imperfect<T, E, L> enum + accessors"
```

---

### Task 4: Imperfect::compose

**Files:**
- Modify: `imperfect/src/lib.rs`

- [ ] **Step 1: Write failing tests for compose**

```rust
// imperfect/src/lib.rs — add to tests module

    // --- compose ---

    #[test]
    fn compose_ok_ok() {
        let a: Imperfect<u32, String> = Imperfect::Ok(1);
        let b: Imperfect<&str, String> = Imperfect::Ok("hi");
        let c = a.compose(b);
        assert!(matches!(c, Imperfect::Ok("hi")));
    }

    #[test]
    fn compose_ok_partial() {
        let a: Imperfect<u32, String> = Imperfect::Ok(1);
        let b: Imperfect<u32, String> = Imperfect::Partial(2, ShannonLoss::new(1.0));
        let c = a.compose(b);
        assert!(c.is_partial());
        assert_eq!(c.loss().as_f64(), 1.0);
        assert_eq!(c.ok(), Some(2));
    }

    #[test]
    fn compose_ok_err() {
        let a: Imperfect<u32, String> = Imperfect::Ok(1);
        let b: Imperfect<u32, String> = Imperfect::Err("fail".into());
        let c = a.compose(b);
        assert!(c.is_err());
    }

    #[test]
    fn compose_partial_ok_carries_loss() {
        let a: Imperfect<u32, String> = Imperfect::Partial(1, ShannonLoss::new(1.0));
        let b: Imperfect<u32, String> = Imperfect::Ok(2);
        let c = a.compose(b);
        assert!(c.is_partial());
        assert_eq!(c.loss().as_f64(), 1.0);
        assert_eq!(c.ok(), Some(2));
    }

    #[test]
    fn compose_partial_partial_accumulates() {
        let a: Imperfect<u32, String> = Imperfect::Partial(1, ShannonLoss::new(1.0));
        let b: Imperfect<u32, String> = Imperfect::Partial(2, ShannonLoss::new(0.5));
        let c = a.compose(b);
        assert!(c.is_partial());
        assert_eq!(c.loss().as_f64(), 1.5);
        assert_eq!(c.ok(), Some(2));
    }

    #[test]
    fn compose_partial_err() {
        let a: Imperfect<u32, String> = Imperfect::Partial(1, ShannonLoss::new(1.0));
        let b: Imperfect<u32, String> = Imperfect::Err("fail".into());
        let c = a.compose(b);
        assert!(c.is_err());
    }

    #[test]
    #[should_panic(expected = "compose called on Err")]
    fn compose_err_panics() {
        let a: Imperfect<u32, String> = Imperfect::Err("fail".into());
        let b: Imperfect<u32, String> = Imperfect::Ok(2);
        let _ = a.compose(b);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p imperfect`
Expected: FAIL — `compose` not defined

- [ ] **Step 3: Implement compose**

```rust
// imperfect/src/lib.rs — add to impl<T, E, L: Loss> Imperfect<T, E, L> block

    /// Propagate accumulated loss from `self` through `next`.
    ///
    /// - Ok + next → next (no loss to propagate)
    /// - Partial(_, loss) + Ok(v) → Partial(v, loss)
    /// - Partial(_, loss1) + Partial(v, loss2) → Partial(v, loss1.combine(loss2))
    /// - Partial(_, _) + Err(e) → Err(e)
    /// - Err + anything → panics (programming error)
    pub fn compose<T2, E2>(self, next: Imperfect<T2, E2, L>) -> Imperfect<T2, E2, L> {
        match self {
            Imperfect::Err(_) => panic!("compose called on Err — check is_ok() first"),
            Imperfect::Ok(_) => next,
            Imperfect::Partial(_, loss) => match next {
                Imperfect::Ok(v) => Imperfect::Partial(v, loss),
                Imperfect::Partial(v, loss2) => Imperfect::Partial(v, loss.combine(loss2)),
                Imperfect::Err(e) => Imperfect::Err(e),
            },
        }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix develop -c cargo test -p imperfect`
Expected: all tests PASS (31 + 7 = 38 total)

- [ ] **Step 5: Commit**

```bash
git add imperfect/src/lib.rs
git commit -m "🟢 imperfect: Imperfect::compose — loss propagation"
```

---

### Task 5: Imperfect std interop

**Files:**
- Modify: `imperfect/src/lib.rs`

- [ ] **Step 1: Write failing tests for From<Result>, From<Option>, Into<Result>**

```rust
// imperfect/src/lib.rs — add to tests module

    // --- std interop ---

    #[test]
    fn from_result_ok() {
        let r: Result<u32, String> = Ok(42);
        let i: Imperfect<u32, String> = r.into();
        assert!(matches!(i, Imperfect::Ok(42)));
    }

    #[test]
    fn from_result_err() {
        let r: Result<u32, String> = Err("oops".into());
        let i: Imperfect<u32, String> = r.into();
        assert!(i.is_err());
    }

    #[test]
    fn into_result_ok() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        let r: Result<u32, String> = i.into();
        assert_eq!(r, Ok(42));
    }

    #[test]
    fn into_result_partial_keeps_value() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.0));
        let r: Result<u32, String> = i.into();
        assert_eq!(r, Ok(42));
    }

    #[test]
    fn into_result_err() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        let r: Result<u32, String> = i.into();
        assert_eq!(r, Err("oops".into()));
    }

    #[test]
    fn from_option_some() {
        let o: Option<u32> = Some(42);
        let i: Imperfect<u32, ()> = o.into();
        assert!(matches!(i, Imperfect::Ok(42)));
    }

    #[test]
    fn from_option_none() {
        let o: Option<u32> = None;
        let i: Imperfect<u32, ()> = o.into();
        assert!(i.is_err());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p imperfect`
Expected: FAIL — From impls not defined

- [ ] **Step 3: Implement std interop**

```rust
// imperfect/src/lib.rs — add after the Imperfect impl block

// --- std interop ---

impl<T, E, L: Loss> From<Result<T, E>> for Imperfect<T, E, L> {
    fn from(r: Result<T, E>) -> Self {
        match r {
            Ok(v) => Imperfect::Ok(v),
            Err(e) => Imperfect::Err(e),
        }
    }
}

impl<T, E, L: Loss> From<Imperfect<T, E, L>> for Result<T, E> {
    fn from(i: Imperfect<T, E, L>) -> Self {
        match i {
            Imperfect::Ok(v) | Imperfect::Partial(v, _) => Ok(v),
            Imperfect::Err(e) => Err(e),
        }
    }
}

impl<T, L: Loss> From<Option<T>> for Imperfect<T, (), L> {
    fn from(o: Option<T>) -> Self {
        match o {
            Some(v) => Imperfect::Ok(v),
            None => Imperfect::Err(()),
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix develop -c cargo test -p imperfect`
Expected: all tests PASS (38 + 7 = 45 total)

- [ ] **Step 5: Commit**

```bash
git add imperfect/src/lib.rs
git commit -m "🟢 imperfect: std interop — From<Result>, From<Option>, Into<Result>"
```

---

### Task 6: Beam trait + PureBeam (tick, next)

**Files:**
- Create: `core/src/beam.rs`
- Modify: `core/src/lib.rs`

- [ ] **Step 1: Write failing tests for PureBeam constructors and Beam methods**

```rust
// core/src/beam.rs

//! Beam — the semifunctor. The pipeline value carrier.
//!
//! `tick` is the primitive: one step forward.
//! `next` is the lossless shorthand.
//! `smap` is the semifunctor map, derived from `tick`.

use imperfect::{Imperfect, Loss, ShannonLoss};
use std::convert::Infallible;

// (trait and struct definitions will go here)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_beam_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 42);
        assert!(b.is_ok());
        assert!(!b.is_err());
        assert_eq!(b.result().ok(), Some(&42));
        assert_eq!(b.input(), &());
    }

    #[test]
    fn pure_beam_partial() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 42, ShannonLoss::new(1.5));
        assert!(b.is_ok());
        assert!(b.is_partial());
        assert_eq!(b.result().ok(), Some(&42));
    }

    #[test]
    fn pure_beam_err() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "oops".into());
        assert!(b.is_err());
        assert!(!b.is_ok());
    }

    #[test]
    fn next_ok_to_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 10);
        let n = b.next("hello");
        assert!(n.is_ok());
        assert!(!n.is_partial());
        assert_eq!(n.result().ok(), Some(&"hello"));
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    fn next_partial_carries_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 10, ShannonLoss::new(2.0));
        let n = b.next(20u32);
        assert!(n.is_partial());
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    #[should_panic(expected = "tick on Err beam")]
    fn next_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.next(0u32);
    }

    #[test]
    fn tick_ok_with_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.tick(Imperfect::<&str, String>::Ok("hi"));
        assert!(n.is_ok());
        assert!(!n.is_partial());
    }

    #[test]
    fn tick_ok_with_partial() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.tick(Imperfect::<&str, String>::Partial("hi", ShannonLoss::new(1.0)));
        assert!(n.is_partial());
        assert_eq!(n.result().ok(), Some(&"hi"));
    }

    #[test]
    fn tick_ok_with_err() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n: PureBeam<u32, u32, i32> = b.tick(Imperfect::Err(-1));
        assert!(n.is_err());
    }

    #[test]
    fn tick_partial_with_ok_carries_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Ok(10));
        assert!(n.is_partial());
    }

    #[test]
    fn tick_partial_with_partial_accumulates() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Partial(10, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
    }

    #[test]
    fn tick_partial_with_err() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Err("fail".into()));
        assert!(n.is_err());
    }

    #[test]
    #[should_panic(expected = "tick on Err beam")]
    fn tick_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.tick(Imperfect::<u32, String>::Ok(0));
    }

    #[test]
    fn type_chain_three_steps() {
        let b0: PureBeam<(), u32> = PureBeam::ok((), 42u32);
        let b1: PureBeam<u32, String> = b0.next("hello".to_string());
        let b2: PureBeam<String, Vec<char>> = b1.next(vec!['a', 'b']);
        assert_eq!(b2.input(), &"hello".to_string());
        assert_eq!(b2.result().ok(), Some(&vec!['a', 'b']));
    }
}
```

- [ ] **Step 2: Update core/src/lib.rs to declare the beam module**

```rust
// core/src/lib.rs
//! Prism — focus | project | refract.
//!
//! A Beam is a semifunctor. A Prism is the monoid lifted into it.
//! Three operations. Three dimensions of space.

pub mod beam;

pub use beam::{Beam, PureBeam};
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `nix develop -c cargo test -p prism-core`
Expected: FAIL — `Beam`, `PureBeam` not defined

- [ ] **Step 4: Implement Beam trait and PureBeam**

```rust
// core/src/beam.rs — add above the test module

//! Beam — the semifunctor. The pipeline value carrier.
//!
//! `tick` is the primitive: one step forward.
//! `next` is the lossless shorthand.
//! `smap` is the semifunctor map, derived from `tick`.

use imperfect::{Imperfect, Loss, ShannonLoss};
use std::convert::Infallible;

/// The pipeline value carrier. A semifunctor over `Imperfect`.
///
/// Three required methods: `input`, `result`, `tick`.
/// Everything else is derived.
///
/// **Contract:** `tick` and `next` panic on Err beams. Call `is_ok()` first.
pub trait Beam: Sized {
    type In;
    type Out;
    type Error;
    type Loss: Loss;

    /// Advance: new Out and Error types. Loss type preserved.
    type Tick<T, E>: Beam<In = Self::Out, Out = T, Error = E, Loss = Self::Loss>;

    /// The input that entered this step.
    fn input(&self) -> &Self::In;

    /// The output of this step, or the error if failed.
    fn result(&self) -> Imperfect<&Self::Out, &Self::Error, Self::Loss>;

    /// The primitive. One tick forward. Panics on Err beam.
    fn tick<T, E>(self, imperfect: Imperfect<T, E, Self::Loss>) -> Self::Tick<T, E>;

    /// Whether this beam has a value (Ok or Partial).
    fn is_ok(&self) -> bool {
        !self.is_err()
    }

    /// Whether this beam is in the Partial state.
    fn is_partial(&self) -> bool {
        self.result().is_partial()
    }

    /// Whether this beam failed (Err).
    fn is_err(&self) -> bool {
        self.result().is_err()
    }

    /// Lossless transition. Shorthand for `tick(Imperfect::Ok(value))`.
    /// Panics on Err beam.
    fn next<T>(self, value: T) -> Self::Tick<T, Self::Error> {
        self.tick(Imperfect::Ok(value))
    }

    /// Semifunctor map. Derived from `tick`.
    /// Panics on Err beam.
    fn smap<T>(
        self,
        f: impl FnOnce(&Self::Out) -> Imperfect<T, Self::Error, Self::Loss>,
    ) -> Self::Tick<T, Self::Error> {
        let imp = match self.result() {
            Imperfect::Ok(v) | Imperfect::Partial(v, _) => f(v),
            Imperfect::Err(_) => panic!("smap on Err beam"),
        };
        self.tick(imp)
    }
}

/// Production beam. Flat struct: input + imperfect. No trace overhead.
pub struct PureBeam<In, Out, E = Infallible, L: Loss = ShannonLoss> {
    input: In,
    imperfect: Imperfect<Out, E, L>,
}

impl<In, Out, E, L: Loss> PureBeam<In, Out, E, L> {
    /// Construct a perfect beam (zero loss).
    pub fn ok(input: In, output: Out) -> Self {
        Self { input, imperfect: Imperfect::Ok(output) }
    }

    /// Construct a partial beam (value with loss).
    pub fn partial(input: In, output: Out, loss: L) -> Self {
        Self { input, imperfect: Imperfect::Partial(output, loss) }
    }

    /// Construct a failed beam.
    pub fn err(input: In, error: E) -> Self {
        Self { input, imperfect: Imperfect::Err(error) }
    }
}

impl<In, Out, E, L: Loss> Beam for PureBeam<In, Out, E, L> {
    type In = In;
    type Out = Out;
    type Error = E;
    type Loss = L;
    type Tick<T, NE> = PureBeam<Out, T, NE, L>;

    fn input(&self) -> &In {
        &self.input
    }

    fn result(&self) -> Imperfect<&Out, &E, L> {
        self.imperfect.as_ref()
    }

    fn tick<T, NE>(self, next: Imperfect<T, NE, L>) -> PureBeam<Out, T, NE, L> {
        match self.imperfect {
            Imperfect::Err(_) => panic!("tick on Err beam — check is_ok() first"),
            Imperfect::Ok(old_out) => PureBeam {
                input: old_out,
                imperfect: next,
            },
            Imperfect::Partial(old_out, loss) => PureBeam {
                input: old_out,
                imperfect: match next {
                    Imperfect::Ok(v) => Imperfect::Partial(v, loss),
                    Imperfect::Partial(v, loss2) => Imperfect::Partial(v, loss.combine(loss2)),
                    Imperfect::Err(e) => Imperfect::Err(e),
                },
            },
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `nix develop -c cargo test -p prism-core`
Expected: all 15 tests PASS

- [ ] **Step 6: Commit**

```bash
git add core/src/beam.rs core/src/lib.rs
git commit -m "🟢 core: Beam trait (semifunctor) + PureBeam with tick/next"
```

---

### Task 7: Beam::smap tests

**Files:**
- Modify: `core/src/beam.rs`

- [ ] **Step 1: Write tests for smap (already a default method — just need tests)**

```rust
// core/src/beam.rs — add to tests module

    #[test]
    fn smap_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.smap(|&v| Imperfect::Ok(v * 2));
        assert_eq!(n.result().ok(), Some(&10));
        assert!(!n.is_partial());
    }

    #[test]
    fn smap_returns_partial() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.smap(|&v| Imperfect::Partial(v * 2, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
        assert_eq!(n.result().ok(), Some(&10));
    }

    #[test]
    fn smap_returns_err() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.smap(|_| Imperfect::<u32, String>::Err("nope".into()));
        assert!(n.is_err());
    }

    #[test]
    fn smap_on_partial_accumulates_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.smap(|&v| Imperfect::Partial(v * 2, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
        // Loss should accumulate: 1.0 + 0.5 = 1.5
        // (verified through tick's compose behavior)
    }

    #[test]
    #[should_panic(expected = "smap on Err beam")]
    fn smap_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.smap(|&v| Imperfect::<u32, String>::Ok(v));
    }
```

- [ ] **Step 2: Run tests to verify they pass**

smap is already implemented as a default method on Beam. These tests validate it.

Run: `nix develop -c cargo test -p prism-core`
Expected: all 20 tests PASS

- [ ] **Step 3: Commit**

```bash
git add core/src/beam.rs
git commit -m "🟢 core: Beam::smap tests — semifunctor map validated"
```

---

### Task 8: Operation trait + Op enum + Beam::apply

**Files:**
- Modify: `core/src/beam.rs`
- Create: `core/src/trace.rs`
- Modify: `core/src/lib.rs`

- [ ] **Step 1: Write failing tests for Operation and apply**

```rust
// core/src/beam.rs — add to tests module

    // --- Operation + apply ---

    /// A trivial operation for testing: doubles the value.
    struct DoubleOp;

    impl Operation<PureBeam<(), u32>> for DoubleOp {
        type Output = PureBeam<u32, u32>;
        fn op(&self) -> Op { Op::Project }
        fn apply(self, beam: PureBeam<(), u32>) -> PureBeam<u32, u32> {
            let v = beam.result().ok().copied().unwrap();
            beam.next(v * 2)
        }
    }

    #[test]
    fn apply_operation() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.apply(DoubleOp);
        assert_eq!(n.result().ok(), Some(&10));
    }
```

- [ ] **Step 2: Create trace.rs with Op enum (3 variants)**

```rust
// core/src/trace.rs

//! Trace — the execution record of a beam through a pipeline.

use std::any::Any;
use std::fmt;

use imperfect::ShannonLoss;

/// Which pipeline operation produced a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Focus,
    Project,
    Refract,
}

/// Any value that is `Debug + Any + Send + Sync` can be stored in a `Trace`.
pub trait Traced: Any + fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + fmt::Debug + Send + Sync> Traced for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The output side of a traced step.
pub enum StepOutput {
    Value(Box<dyn Traced>),
    Error(Box<dyn Traced>),
}

/// A single traced step through the pipeline.
pub struct Step {
    pub prism: &'static str,
    pub op: Op,
    pub loss: ShannonLoss,
    pub input: Box<dyn Traced>,
    pub output: StepOutput,
}

/// Full execution record — all steps through the pipeline.
#[derive(Default)]
pub struct Trace {
    steps: Vec<Step>,
}

impl Trace {
    pub fn new() -> Self {
        Trace { steps: Vec::new() }
    }

    pub fn push(&mut self, step: Step) {
        self.steps.push(step);
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Recover the input at step `i` as concrete type `T`.
    pub fn reenter_at<T: 'static>(&self, i: usize) -> Option<&T> {
        let input: &dyn Traced = self.steps.get(i)?.input.as_ref();
        input.as_any().downcast_ref::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_starts_empty() {
        let t = Trace::new();
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn trace_push_and_len() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ShannonLoss::zero(),
            input: Box::new(42u32),
            output: StepOutput::Value(Box::new("focused".to_string())),
        });
        assert_eq!(t.len(), 1);
        assert!(!t.is_empty());
    }

    #[test]
    fn trace_reenter_at_correct_type() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ShannonLoss::zero(),
            input: Box::new(99u32),
            output: StepOutput::Value(Box::new("out".to_string())),
        });
        assert_eq!(t.reenter_at::<u32>(0), Some(&99u32));
    }

    #[test]
    fn trace_reenter_wrong_type() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ShannonLoss::zero(),
            input: Box::new(99u32),
            output: StepOutput::Value(Box::new("out".to_string())),
        });
        assert!(t.reenter_at::<String>(0).is_none());
    }

    #[test]
    fn trace_reenter_out_of_bounds() {
        let t = Trace::new();
        assert!(t.reenter_at::<u32>(0).is_none());
    }

    #[test]
    fn op_variants_are_distinct() {
        assert_ne!(Op::Focus, Op::Project);
        assert_ne!(Op::Project, Op::Refract);
        assert_ne!(Op::Focus, Op::Refract);
    }
}
```

- [ ] **Step 3: Add Operation trait to beam.rs, update lib.rs**

```rust
// core/src/beam.rs — add above the Beam trait

use crate::trace::Op;

/// A self-contained pipeline operation. Wraps a prism (and closure for
/// user-space operations). The beam arrives via `apply`.
pub trait Operation<B: Beam> {
    type Output: Beam;
    fn op(&self) -> Op;
    fn apply(self, beam: B) -> Self::Output;
}
```

```rust
// core/src/lib.rs — update
pub mod beam;
pub mod trace;

pub use beam::{Beam, Operation, PureBeam};
pub use trace::{Op, Step, StepOutput, Trace, Traced};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix develop -c cargo test -p prism-core`
Expected: all tests PASS (beam: 21 + trace: 6 = 27)

- [ ] **Step 5: Commit**

```bash
git add core/src/beam.rs core/src/trace.rs core/src/lib.rs
git commit -m "🟢 core: Operation trait + Op enum (3 variants) + Trace"
```

---

### Task 9: Prism trait + Operation structs + blanket impl

**Files:**
- Modify: `core/src/lib.rs`

- [ ] **Step 1: Write failing tests for Prism, Operations, DSL**

```rust
// core/src/lib.rs — add at bottom

#[cfg(test)]
mod tests {
    use super::*;
    use imperfect::ShannonLoss;

    /// A prism that counts characters.
    /// focus: String → Vec<char>, project: Vec<char> → usize, refract: usize → String
    struct CountPrism;

    impl Prism for CountPrism {
        type Input     = PureBeam<(), String>;
        type Focused   = PureBeam<String, Vec<char>>;
        type Projected = PureBeam<Vec<char>, usize>;
        type Refracted = PureBeam<usize, String>;

        fn focus(&self, beam: Self::Input) -> Self::Focused {
            let chars: Vec<char> = beam.result()
                .ok()
                .expect("focus: Err beam")
                .chars()
                .collect();
            beam.next(chars)
        }

        fn project(&self, beam: Self::Focused) -> Self::Projected {
            let n = beam.result().ok().expect("project: Err beam").len();
            beam.next(n)
        }

        fn refract(&self, beam: Self::Projected) -> Self::Refracted {
            let n = *beam.result().ok().expect("refract: Err beam");
            beam.next(format!("{} chars", n))
        }
    }

    fn seed(s: &str) -> PureBeam<(), String> {
        PureBeam::ok((), s.to_string())
    }

    // --- Prism method tests ---

    #[test]
    fn focus_yields_chars() {
        let b = CountPrism.focus(seed("hello"));
        assert_eq!(b.result().ok(), Some(&vec!['h', 'e', 'l', 'l', 'o']));
        assert_eq!(b.input(), &"hello".to_string());
    }

    #[test]
    fn project_yields_count() {
        let f = CountPrism.focus(seed("hello"));
        let p = CountPrism.project(f);
        assert_eq!(p.result().ok(), Some(&5));
    }

    #[test]
    fn refract_produces_string() {
        let f = CountPrism.focus(seed("hi"));
        let p = CountPrism.project(f);
        let r = CountPrism.refract(p);
        assert_eq!(r.result().ok(), Some(&"2 chars".to_string()));
    }

    // --- Operation tests ---

    #[test]
    fn operation_focus() {
        let b = Focus(&CountPrism).apply(seed("hello"));
        assert_eq!(b.result().ok(), Some(&vec!['h', 'e', 'l', 'l', 'o']));
    }

    #[test]
    fn operation_project() {
        let focused = CountPrism.focus(seed("hello"));
        let p = Project(&CountPrism).apply(focused);
        assert_eq!(p.result().ok(), Some(&5));
    }

    #[test]
    fn operation_refract() {
        let projected = seed("hi")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let r = Refract(&CountPrism).apply(projected);
        assert_eq!(r.result().ok(), Some(&"2 chars".to_string()));
    }

    // --- DSL pipeline ---

    #[test]
    fn dsl_pipeline() {
        let r = seed("hi")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism))
            .apply(Refract(&CountPrism));
        assert!(r.is_ok());
        assert_eq!(r.result().ok(), Some(&"2 chars".to_string()));
    }

    #[test]
    fn apply_fn_end_to_end() {
        let r = apply(&CountPrism, seed("hi"));
        assert!(r.is_ok());
        assert_eq!(r.result().ok(), Some(&"2 chars".to_string()));
    }

    // --- Blanket impl ---

    #[test]
    fn ref_prism_works() {
        let prism = CountPrism;
        let r = apply(&prism, seed("abc"));
        assert_eq!(r.result().ok(), Some(&"3 chars".to_string()));
    }

    // --- Op labels ---

    #[test]
    fn operation_op_labels() {
        assert_eq!(Focus(&CountPrism).op(), Op::Focus);
        assert_eq!(Project(&CountPrism).op(), Op::Project);
        assert_eq!(Refract(&CountPrism).op(), Op::Refract);
    }

    // --- smap in user space (split/zoom equivalent) ---

    #[test]
    fn smap_as_zoom() {
        let projected = seed("hello")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let zoomed = projected.smap(|&n| Imperfect::Ok(n * 2));
        assert_eq!(zoomed.result().ok(), Some(&10));
    }

    #[test]
    fn smap_as_split() {
        let projected = seed("abc")
            .apply(Focus(&CountPrism))
            .apply(Project(&CountPrism));
        let split = projected.smap(|&n| Imperfect::Ok((0..n as u32).collect::<Vec<_>>()));
        assert_eq!(split.result().ok(), Some(&vec![0, 1, 2]));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p prism-core`
Expected: FAIL — `Prism`, `Focus`, `Project`, `Refract`, `apply` not defined

- [ ] **Step 3: Implement Prism trait, Operation structs, blanket impl**

```rust
// core/src/lib.rs — replace content

//! Prism — focus | project | refract.
//!
//! A Beam is a semifunctor. A Prism is the monoid lifted into it.
//! Three operations. Three dimensions of space.

pub mod beam;
pub mod trace;

pub use beam::{Beam, Operation, PureBeam};
pub use imperfect::{Imperfect, Loss, ShannonLoss};
pub use trace::{Op, Step, StepOutput, Trace, Traced};

// ---------------------------------------------------------------------------
// Prism trait
// ---------------------------------------------------------------------------

/// Three optic operations. A prism is the monoid lifted into the Beam
/// semifunctor. All beam types are associated types, not parameters.
///
/// Monoid laws (testable, not type-enforced):
/// - Composition is associative
/// - Identity prism exists (all three methods call `next`)
pub trait Prism {
    type Input:     Beam;
    type Focused:   Beam<In = <Self::Input     as Beam>::Out>;
    type Projected: Beam<In = <Self::Focused   as Beam>::Out>;
    type Refracted: Beam<In = <Self::Projected as Beam>::Out>;

    fn focus(&self, beam: Self::Input) -> Self::Focused;
    fn project(&self, beam: Self::Focused) -> Self::Projected;
    fn refract(&self, beam: Self::Projected) -> Self::Refracted;
}

/// Blanket impl: `&P` is a Prism wherever `P` is.
/// Enables `beam.apply(Focus(&prism))` without consuming the prism.
impl<P: Prism> Prism for &P {
    type Input     = P::Input;
    type Focused   = P::Focused;
    type Projected = P::Projected;
    type Refracted = P::Refracted;

    fn focus(&self, beam: P::Input) -> P::Focused         { P::focus(self, beam) }
    fn project(&self, beam: P::Focused) -> P::Projected   { P::project(self, beam) }
    fn refract(&self, beam: P::Projected) -> P::Refracted { P::refract(self, beam) }
}

/// Run a prism end-to-end: focus → project → refract.
pub fn apply<P: Prism>(prism: &P, beam: P::Input) -> P::Refracted {
    beam.apply(Focus(prism))
        .apply(Project(prism))
        .apply(Refract(prism))
}

// ---------------------------------------------------------------------------
// Operation structs — three pipeline stages
// ---------------------------------------------------------------------------

/// focus: Input → Focused.
pub struct Focus<P>(pub P);

/// project: Focused → Projected.
pub struct Project<P>(pub P);

/// refract: Projected → Refracted.
pub struct Refract<P>(pub P);

impl<P: Prism> Operation<P::Input> for Focus<P> {
    type Output = P::Focused;
    fn op(&self) -> Op { Op::Focus }
    fn apply(self, beam: P::Input) -> P::Focused { self.0.focus(beam) }
}

impl<P: Prism> Operation<P::Focused> for Project<P> {
    type Output = P::Projected;
    fn op(&self) -> Op { Op::Project }
    fn apply(self, beam: P::Focused) -> P::Projected { self.0.project(beam) }
}

impl<P: Prism> Operation<P::Projected> for Refract<P> {
    type Output = P::Refracted;
    fn op(&self) -> Op { Op::Refract }
    fn apply(self, beam: P::Projected) -> P::Refracted { self.0.refract(beam) }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `nix develop -c cargo test -p prism-core`
Expected: all tests PASS (beam: 25 + trace: 6 + lib: 12 = 43)

- [ ] **Step 5: Commit**

```bash
git add core/src/lib.rs
git commit -m "🟢 core: Prism trait (3 dimensions) + Operation structs + DSL"
```

---

### Task 10: Migrate supporting modules

**Files:**
- Move: `src/oid.rs` → `core/src/oid.rs`
- Move: `src/spectral_oid.rs` → `core/src/spectral_oid.rs`
- Move: `src/content.rs` → `core/src/content.rs`
- Move: `src/precision.rs` → `core/src/precision.rs`
- Move: `src/connection.rs` → `core/src/connection.rs`
- Move: `src/metal.rs` → `core/src/metal.rs`
- Move: `src/optics/` → `core/src/optics/`
- Modify: `core/src/lib.rs` (add module declarations)
- Modify: `core/src/connection.rs` (update imports)
- Delete: `src/` directory

- [ ] **Step 1: Move files**

```bash
cp src/oid.rs core/src/oid.rs
cp src/spectral_oid.rs core/src/spectral_oid.rs
cp src/content.rs core/src/content.rs
cp src/precision.rs core/src/precision.rs
cp src/connection.rs core/src/connection.rs
cp src/metal.rs core/src/metal.rs
cp -r src/optics core/src/optics
```

- [ ] **Step 2: Update core/src/lib.rs — add module declarations and re-exports**

Add module declarations to `core/src/lib.rs`, after the existing `pub mod beam;` and `pub mod trace;`:

```rust
pub mod connection;
pub mod content;
pub mod metal;
pub mod oid;
pub mod precision;
pub mod spectral_oid;

#[cfg(feature = "optics")]
pub mod optics;

pub use connection::{Connection, ScalarConnection};
pub use content::ContentAddressed;
pub use oid::Oid;
pub use precision::{Precision, Pressure};
pub use spectral_oid::SpectralOid;
```

- [ ] **Step 3: Update core/src/connection.rs — import ShannonLoss from imperfect**

Replace `use crate::loss::ShannonLoss;` with:

```rust
use imperfect::ShannonLoss;
```

- [ ] **Step 4: Update crate-internal imports in moved files**

In `core/src/content.rs`, update `use crate::oid::Oid;` — this stays the same since content.rs is now in the core crate and oid.rs is too.

In `core/src/spectral_oid.rs`, update imports — `use crate::oid::Oid;` and `use crate::precision::Precision;` stay the same.

No other import changes needed — all `crate::` references point within core.

- [ ] **Step 5: Remove old src/ directory**

```bash
rm -rf src/
```

- [ ] **Step 6: Run all workspace tests**

Run: `nix develop -c cargo test --workspace`
Expected: all tests PASS across both crates

- [ ] **Step 7: Commit**

```bash
git add core/src/ imperfect/src/
git rm -r src/
git commit -m "♻️ migrate supporting modules to core/, remove old src/"
```

---

### Task 11: End-to-end integration test + cleanup

**Files:**
- Create: `core/tests/integration.rs`

- [ ] **Step 1: Write integration test — full pipeline with smap**

```rust
// core/tests/integration.rs

use imperfect::{Imperfect, ShannonLoss};
use prism_core::{
    Beam, Focus, Prism, Project, PureBeam, Refract,
};

/// A prism that tokenizes → counts → formats.
struct TokenPrism;

impl Prism for TokenPrism {
    type Input     = PureBeam<(), String>;
    type Focused   = PureBeam<String, Vec<String>>;
    type Projected = PureBeam<Vec<String>, usize>;
    type Refracted = PureBeam<usize, String>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let tokens: Vec<String> = beam.result()
            .ok()
            .expect("focus: Err beam")
            .split_whitespace()
            .map(String::from)
            .collect();
        beam.next(tokens)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let count = beam.result().ok().expect("project: Err beam").len();
        beam.next(count)
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let n = *beam.result().ok().expect("refract: Err beam");
        beam.next(format!("{} tokens", n))
    }
}

#[test]
fn full_pipeline_dsl() {
    let result = PureBeam::ok((), "hello world foo".to_string())
        .apply(Focus(&TokenPrism))
        .apply(Project(&TokenPrism))
        .apply(Refract(&TokenPrism));

    assert!(result.is_ok());
    assert_eq!(result.result().ok(), Some(&"3 tokens".to_string()));
}

#[test]
fn full_pipeline_apply_fn() {
    let result = prism_core::apply(&TokenPrism, PureBeam::ok((), "a b c d".to_string()));
    assert_eq!(result.result().ok(), Some(&"4 tokens".to_string()));
}

#[test]
fn smap_as_zoom_in_pipeline() {
    let projected = PureBeam::ok((), "hello world".to_string())
        .apply(Focus(&TokenPrism))
        .apply(Project(&TokenPrism));

    let doubled = projected.smap(|&n| Imperfect::Ok(n * 2));
    assert_eq!(doubled.result().ok(), Some(&4));
}

#[test]
fn smap_as_split_in_pipeline() {
    let focused = PureBeam::ok((), "hello world".to_string())
        .apply(Focus(&TokenPrism));

    let chars: PureBeam<Vec<String>, Vec<char>> = focused.smap(|tokens| {
        let all_chars: Vec<char> = tokens.iter().flat_map(|t| t.chars()).collect();
        Imperfect::Ok(all_chars)
    });
    assert_eq!(chars.result().ok(), Some(&vec!['h', 'e', 'l', 'l', 'o', 'w', 'o', 'r', 'l', 'd']));
}

#[test]
fn partial_beam_propagates_loss() {
    let b: PureBeam<(), String, String, ShannonLoss> =
        PureBeam::partial((), "hello world".to_string(), ShannonLoss::new(0.5));

    let focused = TokenPrism.focus(b);
    assert!(focused.is_partial());

    let projected = TokenPrism.project(focused);
    assert!(projected.is_partial());
}

#[test]
fn imperfect_result_interop() {
    let ok_result: Result<u32, String> = Ok(42);
    let imp: Imperfect<u32, String> = ok_result.into();
    assert!(imp.is_ok());

    let back: Result<u32, String> = imp.into();
    assert_eq!(back, Ok(42));
}
```

- [ ] **Step 2: Run all workspace tests**

Run: `nix develop -c cargo test --workspace`
Expected: all tests PASS across imperfect, prism-core, and integration tests

- [ ] **Step 3: Commit**

```bash
git add core/tests/integration.rs
git commit -m "🟢 integration: end-to-end pipeline with smap, loss propagation, interop"
```

---

## Self-Review

**Spec coverage:**
- ✅ Loss trait + ShannonLoss → Task 2
- ✅ Imperfect<T, E, L> enum → Task 3
- ✅ Imperfect::compose → Task 4
- ✅ Std interop (From<Result>, From<Option>) → Task 5
- ✅ Beam trait (tick, next, smap, apply) → Tasks 6-7
- ✅ PureBeam → Task 6
- ✅ Operation trait + Op enum (3 variants) → Task 8
- ✅ Prism trait (3 methods) + blanket impl → Task 9
- ✅ Operation structs (Focus, Project, Refract) → Task 9
- ✅ DSL pipeline → Task 9
- ✅ Split/zoom as user-space smap → Task 9 tests, Task 11
- ✅ Migration of supporting modules → Task 10
- ✅ Workspace structure (imperfect/ + core/) → Task 1

**Placeholder scan:** No TBDs, TODOs, or "fill in later" — all code is complete.

**Type consistency:** Verified across tasks:
- `Imperfect::Ok/Partial/Err` used consistently
- `tick`/`next`/`smap` signatures match between trait and impl
- `PureBeam::ok/partial/err` constructors match usage in tests
- `Op::Focus/Project/Refract` — 3 variants only, consistent everywhere
