# Implementation Plan: #[derive(Lambda)]

**Status:** Plan
**Feature flag:** `lambda` (prism-core), none (prism-derive — derives are always available)
**Date:** 2026-04-15

---

## Goal

The mirror compiler becomes named lambda composition. Each compiler phase
is a struct with `#[derive(Lambda)]` and `#[oid("@X")]`. Phases compose
with `.then()`. The pipeline IS content-addressed lambda terms reduced
at compile time. No handler functions. No strings.

---

## Dependency Order

```
Arc 1: Lambda<T> enum + reduce          (prism-core, feature = "lambda")
Arc 2: Composable trait + Composed<T>   (prism-core, feature = "lambda")
Arc 3: #[derive(Lambda)] proc macro     (prism-derive)
Arc 4: Named lambdas in mirror          (mirror crate)
Arc 5: Static pipelines + CLI dispatch  (mirror crate)
Arc 6: Codegen surface                  (mirror craft boot --code rust)
```

Each arc is independently shippable. Each arc has its own red/green/refactor cycle.

---

## Arc 1: Lambda<T> in prism-core

### 1.1 The enum

**File:** `/Users/alexwolf/dev/projects/prism/core/src/lambda/mod.rs`

```rust
use crate::merkle::MerkleTree;
use crate::oid::{Addressable, Oid};

/// A lambda term over content-addressed trees.
///
/// Four variants. No strings. Oid for identity.
#[derive(Clone, Debug)]
pub enum Lambda<T: MerkleTree> {
    /// Variable binding. The Oid identifies which binding.
    Bind(Oid),

    /// Abstraction. Parameter Oid + body.
    Abs(AbsLambda<T>),

    /// Application. Function + argument.
    Apply(ApplyLambda<T>),

    /// Case. Scrutinee + arms.
    Case(CaseLambda<T>),
}

#[derive(Clone, Debug)]
pub struct AbsLambda<T: MerkleTree> {
    pub param: Oid,
    pub body: Box<Lambda<T>>,
}

#[derive(Clone, Debug)]
pub struct ApplyLambda<T: MerkleTree> {
    pub function: Box<Lambda<T>>,
    pub argument: Box<Lambda<T>>,
}

#[derive(Clone, Debug)]
pub struct CaseLambda<T: MerkleTree> {
    pub scrutinee: Box<Lambda<T>>,
    pub arms: Vec<(Oid, Lambda<T>)>,
}
```

Val is absent. A value IS `Lambda::Bind(oid)` where the oid resolves to
a `T` in the Store. Values are bindings that already settled. Not a
separate variant.

### 1.2 Factory methods

```rust
impl<T: MerkleTree> Lambda<T> {
    pub fn bind(oid: Oid) -> Self { Lambda::Bind(oid) }

    pub fn abs(param: Oid, body: Lambda<T>) -> Self {
        Lambda::Abs(AbsLambda { param, body: Box::new(body) })
    }

    pub fn apply(function: Lambda<T>, argument: Lambda<T>) -> Self {
        Lambda::Apply(ApplyLambda {
            function: Box::new(function),
            argument: Box::new(argument),
        })
    }

    pub fn case(scrutinee: Lambda<T>, arms: Vec<(Oid, Lambda<T>)>) -> Self {
        Lambda::Case(CaseLambda {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }
}
```

### 1.3 Addressable + MerkleTree

Lambda<T> implements `Addressable`. The Oid is computed recursively:

```rust
impl<T: MerkleTree> Addressable for Lambda<T> {
    fn oid(&self) -> Oid {
        match self {
            Lambda::Bind(x) => Oid::hash(format!("Bind:{}", x).as_bytes()),
            Lambda::Abs(a) => Oid::hash(
                format!("Abs:{}:{}", a.param, a.body.oid()).as_bytes()
            ),
            Lambda::Apply(a) => Oid::hash(
                format!("Apply:{}:{}", a.function.oid(), a.argument.oid()).as_bytes()
            ),
            Lambda::Case(c) => {
                let arms: String = c.arms.iter()
                    .map(|(p, b)| format!("{}:{}", p, b.oid()))
                    .collect::<Vec<_>>()
                    .join(",");
                Oid::hash(format!("Case:{}:[{}]", c.scrutinee.oid(), arms).as_bytes())
            }
        }
    }
}
```

Lambda<T> implements `MerkleTree` with `Data = LambdaTag` (an enum of
Bind/Abs/Apply/Case with the immediate Oid payload, no recursion).

### 1.4 Reduction

**File:** `/Users/alexwolf/dev/projects/prism/core/src/lambda/reduce.rs`

```rust
use terni::Imperfect;

#[derive(Debug, Clone)]
pub enum ReductionError {
    UnboundVariable(Oid),
    NotAFunction(Oid),
    NonExhaustiveMatch(Oid),
}

#[derive(Debug, Clone, Default)]
pub struct ReductionLoss {
    pub steps: usize,
    pub budget_remaining: Option<usize>,
}

impl terni::Loss for ReductionLoss {
    fn is_zero(&self) -> bool { self.steps == 0 }
    fn zero() -> Self { Self::default() }
    fn combine(&self, other: &Self) -> Self {
        ReductionLoss {
            steps: self.steps + other.steps,
            budget_remaining: match (self.budget_remaining, other.budget_remaining) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
        }
    }
    fn total() -> Self {
        ReductionLoss { steps: usize::MAX, budget_remaining: Some(0) }
    }
}

pub fn reduce<T: MerkleTree>(
    term: Lambda<T>,
) -> Imperfect<Lambda<T>, ReductionError, ReductionLoss>;

pub fn reduce_bounded<T: MerkleTree>(
    term: Lambda<T>,
    budget: usize,
) -> Imperfect<Lambda<T>, ReductionError, ReductionLoss>;
```

### 1.5 Module wiring

**File:** `/Users/alexwolf/dev/projects/prism/core/src/lib.rs`

Add after the `bundle` feature gate:

```rust
#[cfg(feature = "lambda")]
pub mod lambda;
```

**File:** `/Users/alexwolf/dev/projects/prism/core/Cargo.toml`

```toml
[features]
default = []
optics = []
bundle = ["optics"]
lambda = []
lapack = ["dep:cc"]
```

### 1.6 TDD tasks

| # | Red | Green | File |
|---|-----|-------|------|
| 1 | `Lambda::bind(oid).oid()` is deterministic | Implement `Addressable` | `lambda/mod.rs` |
| 2 | `Lambda::abs(p, body).oid()` depends on param AND body | Recursive oid | `lambda/mod.rs` |
| 3 | Same term = same oid, different term = different oid | Content addressing | `lambda/mod.rs` |
| 4 | `Lambda::apply(f, x)` constructs correctly | Factory method | `lambda/mod.rs` |
| 5 | `Lambda<T>` implements `MerkleTree` | `data()`, `children()` | `lambda/mod.rs` |
| 6 | `ReductionLoss::zero().is_zero()` is true | Loss impl | `lambda/reduce.rs` |
| 7 | `ReductionLoss::combine` adds steps | Combine impl | `lambda/reduce.rs` |
| 8 | `reduce_bounded(App(Abs(x, Bind(x)), val), 10)` = val | Beta reduction | `lambda/reduce.rs` |
| 9 | `reduce_bounded(Bind(unbound), 10)` = `Failure(UnboundVariable)` | Error path | `lambda/reduce.rs` |
| 10 | `reduce_bounded(deeply_nested, 2)` = `Partial` with budget info | Budget exhaustion | `lambda/reduce.rs` |

---

## Arc 2: Composable trait + Composed

### 2.1 The trait

**File:** `/Users/alexwolf/dev/projects/prism/core/src/lambda/compose.rs`

```rust
use crate::lambda::Lambda;
use crate::merkle::MerkleTree;
use crate::oid::{Addressable, Oid};

/// A named lambda that can compose with other named lambdas.
pub trait Composable: Addressable + Sized {
    type Tree: MerkleTree;

    /// Chain: apply self, then apply next.
    fn then<C: Composable<Tree = Self::Tree>>(self, next: C) -> Composed<Self::Tree>;

    /// Wrap an input in a Lambda::Apply with this composable as the function.
    fn apply_to(&self, input: Lambda<Self::Tree>) -> Lambda<Self::Tree>;

    /// This composable as a Lambda::Abs.
    fn as_lambda(&self) -> Lambda<Self::Tree>;
}
```

### 2.2 Composed IS a lambda

```rust
/// A composition of named lambdas. IS a Lambda::Abs that chains Apply calls.
///
/// `Parse.then(Resolve)` produces a Composed whose `as_lambda()` is:
/// `Abs(x, Apply(resolve_lambda, Apply(parse_lambda, Bind(x))))`
///
/// Not a Vec. A lambda.
pub struct Composed<T: MerkleTree> {
    lambda: Lambda<T>,
    oid: Oid,
}

impl<T: MerkleTree> Addressable for Composed<T> {
    fn oid(&self) -> Oid { self.oid.clone() }
}

impl<T: MerkleTree> Composable for Composed<T> {
    type Tree = T;

    fn then<C: Composable<Tree = T>>(self, next: C) -> Composed<T> {
        // Compose: Abs(x, Apply(next, Apply(self, Bind(x))))
        let param = Oid::hash(b"compose_param");
        let chained = Lambda::abs(
            param.clone(),
            Lambda::apply(
                next.as_lambda(),
                Lambda::apply(self.lambda, Lambda::bind(param)),
            ),
        );
        let oid = chained.oid();
        Composed { lambda: chained, oid }
    }

    fn apply_to(&self, input: Lambda<T>) -> Lambda<T> {
        Lambda::apply(self.lambda.clone(), input)
    }

    fn as_lambda(&self) -> Lambda<T> {
        self.lambda.clone()
    }
}
```

### 2.3 Key design decision

Composed is NOT `Vec<Box<dyn Composable>>`. Composed IS `Lambda::Abs`.
The composition IS a lambda term. It has an Oid. It is content-addressed.
Two pipelines with the same steps in the same order have the same Oid.
The Store caches the result.

### 2.4 TDD tasks

| # | Red | Green | File |
|---|-----|-------|------|
| 1 | `a.then(b).oid()` is deterministic | Composed::oid from lambda.oid() | `lambda/compose.rs` |
| 2 | `a.then(b).oid() != b.then(a).oid()` | Order matters | `lambda/compose.rs` |
| 3 | `a.then(b).then(c).oid() == a.then(b.then(c)).oid()` — NO, this should DIFFER | Composition is left-associated | `lambda/compose.rs` |
| 4 | `composed.apply_to(input)` wraps in Apply | Apply construction | `lambda/compose.rs` |
| 5 | `composed.as_lambda()` returns the Abs term | Lambda extraction | `lambda/compose.rs` |
| 6 | Reducing `composed.apply_to(val)` applies each step in order | End-to-end | `lambda/compose.rs` |

---

## Arc 3: #[derive(Lambda)] proc macro

### 3.1 What it generates

**File:** `/Users/alexwolf/dev/projects/prism/derive/src/lib.rs`

New derive macro, next to existing `#[derive(Prism)]`:

```rust
#[proc_macro_derive(Lambda, attributes(oid))]
pub fn derive_lambda(input: TokenStream) -> TokenStream { ... }
```

For a struct like:

```rust
#[derive(Lambda)]
#[oid("@parse")]
pub struct Parse;
```

The macro generates:

```rust
// 1. Addressable (reuses extract_oid_name from Prism derive)
impl prism_core::Addressable for Parse {
    fn oid(&self) -> prism_core::Oid {
        prism_core::Oid::hash("@parse".as_bytes())
    }
}

// 2. Display
impl ::std::fmt::Display for Parse {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "@parse")
    }
}

// 3. Composable — the new part
impl prism_core::lambda::Composable for Parse {
    type Tree = <Parse as prism_core::lambda::LambdaImpl>::Tree;

    fn then<C: prism_core::lambda::Composable<Tree = Self::Tree>>(
        self, next: C,
    ) -> prism_core::lambda::Composed<Self::Tree> {
        let param = prism_core::Oid::hash(b"compose_param");
        let lambda = prism_core::lambda::Lambda::abs(
            param.clone(),
            prism_core::lambda::Lambda::apply(
                next.as_lambda(),
                prism_core::lambda::Lambda::apply(
                    self.as_lambda(),
                    prism_core::lambda::Lambda::bind(param),
                ),
            ),
        );
        let oid = prism_core::Addressable::oid(&lambda);
        prism_core::lambda::Composed::new(lambda, oid)
    }

    fn apply_to(
        &self, input: prism_core::lambda::Lambda<Self::Tree>,
    ) -> prism_core::lambda::Lambda<Self::Tree> {
        prism_core::lambda::Lambda::apply(self.as_lambda(), input)
    }

    fn as_lambda(&self) -> prism_core::lambda::Lambda<Self::Tree> {
        prism_core::lambda::Lambda::abs(
            self.oid(),
            prism_core::lambda::Lambda::bind(self.oid()),
        )
    }
}
```

### 3.2 The LambdaImpl trait

The derive needs to know what `T` in `Lambda<T>` is. A companion trait:

**File:** `/Users/alexwolf/dev/projects/prism/core/src/lambda/mod.rs`

```rust
/// Implemented by types that have a lambda body.
///
/// The default `as_lambda()` is identity (pass-through).
/// Override to provide a real compiler phase body.
pub trait LambdaImpl: Addressable {
    type Tree: MerkleTree;

    /// The lambda body. Default: identity (Abs(self_oid, Bind(self_oid))).
    fn lambda_body(&self) -> Lambda<Self::Tree> {
        Lambda::abs(self.oid(), Lambda::bind(self.oid()))
    }
}
```

The derive generates a default `LambdaImpl` that returns identity.
The mirror crate overrides it for each phase with the real body.

### 3.3 Sharing extract_oid_name

Both `#[derive(Prism)]` and `#[derive(Lambda)]` need `extract_oid_name`.
Extract it into a shared helper in `prism-derive`. Both derives call it.

### 3.4 Feature gating

The derive itself is NOT feature-gated. It's always available in
prism-derive. But the generated code references `prism_core::lambda::*`,
which only exists when `feature = "lambda"` is enabled. If lambda is
not enabled, using `#[derive(Lambda)]` is a compile error at the
generated code level. This is correct — no phantom availability.

### 3.5 TDD tasks

| # | Red | Green | File |
|---|-----|-------|------|
| 1 | `#[derive(Lambda)] #[oid("@test")] struct Test;` compiles | Basic derive | `derive/src/lib.rs` |
| 2 | `Test.oid() == Oid::hash("@test".as_bytes())` | Addressable from oid attr | `derive/src/lib.rs` |
| 3 | `format!("{}", Test) == "@test"` | Display | `derive/src/lib.rs` |
| 4 | `Test.as_lambda()` returns Abs | LambdaImpl default | `derive/src/lib.rs` |
| 5 | `Test.then(Test2).oid()` is deterministic | Composable | `derive/src/lib.rs` |
| 6 | Missing `#[oid(...)]` panics at compile time | Validation | `derive/src/lib.rs` |
| 7 | `#[oid("no-at")]` panics | @ prefix validation | `derive/src/lib.rs` |

---

## Arc 4: Named lambdas for the mirror compiler

### 4.1 Phase structs

**File:** `/Users/alexwolf/dev/projects/mirror/src/lambda_phases.rs` (new)

```rust
use prism::DerivePrism as Prism;  // for Addressable + Display
use prism::lambda::{Lambda, LambdaImpl, Composable, Composed};
use crate::mirror_ast::MirrorAST;
use crate::declaration::MirrorFragment;

// MirrorFragment implements MerkleTree (via fragmentation crate).
// Lambda<MirrorFragment> is the term type for the mirror compiler.

#[derive(Debug, Clone, Lambda)]
#[oid("@parse")]
pub struct Parse;

#[derive(Debug, Clone, Lambda)]
#[oid("@resolve")]
pub struct Resolve;

#[derive(Debug, Clone, Lambda)]
#[oid("@emit")]
pub struct Emit;

#[derive(Debug, Clone, Lambda)]
#[oid("@kintsugi")]
pub struct Kintsugi;

#[derive(Debug, Clone, Lambda)]
#[oid("@strict")]
pub struct Strict;

#[derive(Debug, Clone, Lambda)]
#[oid("@properties")]
pub struct Properties;

#[derive(Debug, Clone, Lambda)]
#[oid("@classify")]
pub struct Classify;
```

### 4.2 Implementing LambdaImpl for each phase

Each phase overrides `lambda_body()` with a Lambda::Abs that wraps
the real Rust function. The Rust function IS the reduction body.

```rust
impl LambdaImpl for Parse {
    type Tree = MirrorFragment;

    fn lambda_body(&self) -> Lambda<MirrorFragment> {
        // The body calls parse_form internally.
        // The lambda wraps it: Abs(@parse, <reduction that calls parse_form>)
        // The actual Rust function is the reduction engine for this lambda.
        Lambda::abs(self.oid(), Lambda::bind(self.oid()))
    }
}
```

The key insight: the lambda body does NOT re-implement parsing in lambda
calculus. It wraps `parse_form` as a native reduction rule. The lambda
is the IDENTITY of the phase. The Rust function is the BODY. reduce()
dispatches to the Rust function when it encounters an Apply with a
known phase Oid.

This requires a `NativeBinding` mechanism in the reduction engine:

**File:** `/Users/alexwolf/dev/projects/prism/core/src/lambda/reduce.rs`

```rust
use std::collections::HashMap;

type NativeFn<T> = Box<dyn Fn(Lambda<T>) -> Imperfect<Lambda<T>, ReductionError, ReductionLoss>>;

pub struct Environment<T: MerkleTree> {
    bindings: HashMap<Oid, NativeFn<T>>,
}

impl<T: MerkleTree> Environment<T> {
    pub fn new() -> Self {
        Environment { bindings: HashMap::new() }
    }

    pub fn bind(&mut self, oid: Oid, f: NativeFn<T>) {
        self.bindings.insert(oid, f);
    }

    pub fn reduce(
        &self, term: Lambda<T>, budget: usize,
    ) -> Imperfect<Lambda<T>, ReductionError, ReductionLoss> {
        // When Apply(Abs(oid, _), arg) and oid is in bindings:
        // call bindings[oid](arg) instead of substitution.
        // This is the native FFI for lambda reduction.
        reduce_with_env(term, budget, &self.bindings)
    }
}
```

### 4.3 Wiring into MirrorRuntime

**File:** `/Users/alexwolf/dev/projects/mirror/src/mirror_runtime.rs`

`MirrorRuntime::new()` builds an `Environment<MirrorFragment>` with
each phase's native binding registered:

```rust
impl MirrorRuntime {
    pub fn lambda_env(&self) -> Environment<MirrorFragment> {
        let mut env = Environment::new();

        env.bind(Parse.oid(), Box::new(|input| {
            // Unwrap the input Lambda to get source, call parse_form
            // Wrap result back into Lambda
            todo!("wire parse_form")
        }));

        env.bind(Resolve.oid(), Box::new(|input| {
            todo!("wire resolve")
        }));

        // ... etc for each phase

        env
    }
}
```

### 4.4 TDD tasks

| # | Red | Green | File |
|---|-----|-------|------|
| 1 | `Parse.oid()` is not dark | Derive works in mirror | `lambda_phases.rs` |
| 2 | `Parse.oid() != Resolve.oid()` | Unique per phase | `lambda_phases.rs` |
| 3 | `Parse.then(Resolve).oid()` is deterministic | Composition | `lambda_phases.rs` |
| 4 | `Parse.then(Resolve).oid() != Resolve.then(Parse).oid()` | Order matters | `lambda_phases.rs` |
| 5 | `env.reduce(Parse.apply_to(source_lambda), 100)` parses | Native binding | `lambda_phases.rs` |
| 6 | `env.reduce(Parse.then(Resolve).apply_to(source), 100)` pipeline | End-to-end | `lambda_phases.rs` |
| 7 | Same source + same pipeline = same result Oid | Content addressing | `lambda_phases.rs` |

---

## Arc 5: Static pipelines + CLI dispatch

### 5.1 Pre-composed pipelines

**File:** `/Users/alexwolf/dev/projects/mirror/src/lambda_phases.rs`

```rust
use std::sync::LazyLock;

/// The standard compilation pipeline: parse -> resolve -> properties -> emit.
pub static CRAFT: LazyLock<Composed<MirrorFragment>> = LazyLock::new(||
    Parse.then(Resolve).then(Properties).then(Emit)
);

/// Kintsugi pipeline: parse -> resolve -> kintsugi (loss-tolerant emit).
pub static KINTSUGI_PIPELINE: LazyLock<Composed<MirrorFragment>> = LazyLock::new(||
    Parse.then(Resolve).then(Kintsugi)
);

/// Classification pipeline: parse -> resolve -> classify.
pub static CLASSIFY_PIPELINE: LazyLock<Composed<MirrorFragment>> = LazyLock::new(||
    Parse.then(Resolve).then(Classify)
);

/// Strict pipeline: parse -> resolve -> strict -> properties -> emit.
pub static STRICT_PIPELINE: LazyLock<Composed<MirrorFragment>> = LazyLock::new(||
    Parse.then(Resolve).then(Strict).then(Properties).then(Emit)
);
```

### 5.2 CLI dispatch by Oid

**File:** `/Users/alexwolf/dev/projects/mirror/src/cli.rs` (or wherever CLI dispatch lives)

```rust
use prism::Oid;

fn dispatch(command: &Oid, env: &Environment<MirrorFragment>) -> ... {
    let pipeline = if *command == CRAFT.oid() {
        CRAFT.as_lambda()
    } else if *command == KINTSUGI_PIPELINE.oid() {
        KINTSUGI_PIPELINE.as_lambda()
    } else if *command == CLASSIFY_PIPELINE.oid() {
        CLASSIFY_PIPELINE.as_lambda()
    } else if *command == STRICT_PIPELINE.oid() {
        STRICT_PIPELINE.as_lambda()
    } else {
        return Err(...)
    };

    env.reduce(Lambda::apply(pipeline, source_lambda), COMPILE_BUDGET)
}
```

No match on strings. Oid dispatch. The pipeline is a lambda. dispatch()
is `reduce(Apply(pipeline, source))`.

### 5.3 TDD tasks

| # | Red | Green | File |
|---|-----|-------|------|
| 1 | `CRAFT.oid()` is not dark and is deterministic | Static pipeline | tests |
| 2 | `CRAFT.oid() != KINTSUGI_PIPELINE.oid()` | Unique pipelines | tests |
| 3 | Dispatching `CRAFT.oid()` runs parse+resolve+properties+emit | End-to-end | tests |
| 4 | Dispatching unknown Oid returns error | Error path | tests |

---

## Arc 6: Codegen surface

### 6.1 How boot generates derives

`mirror craft boot --code rust` reads boot grammars and generates
the `generated.rs` file. Currently it generates `#[derive(Prism)]` structs
with `#[oid("@X")]` for each grammar.

Extend this: for each grammar that declares actions (compilation phases),
also generate `#[derive(Lambda)]` structs. The grammar's action becomes
the lambda's identity.

### 6.2 Generated output shape

**File:** `/Users/alexwolf/dev/projects/mirror/src/generated.rs`

Current (unchanged):
```rust
#[derive(Debug, Clone, Prism)]
#[oid("@prism")]
pub struct PrismGrammar;
```

New addition:
```rust
#[derive(Debug, Clone, Lambda)]
#[oid("@parse")]
pub struct ParseLambda;

#[derive(Debug, Clone, Lambda)]
#[oid("@resolve")]
pub struct ResolveLambda;

// Pre-composed pipeline
pub static CRAFT_PIPELINE: LazyLock<Composed<MirrorFragment>> = LazyLock::new(||
    ParseLambda.then(ResolveLambda).then(EmitLambda)
);
```

### 6.3 Plugin composition

External crates can define their own lambdas and compose with mirror's:

```rust
// In a plugin crate
#[derive(Lambda)]
#[oid("@my_transform")]
pub struct MyTransform;

impl LambdaImpl for MyTransform {
    type Tree = MirrorFragment;
    fn lambda_body(&self) -> Lambda<MirrorFragment> { ... }
}

// Compose with mirror's standard pipeline
let custom = Parse.then(Resolve).then(MyTransform).then(Emit);
```

This works because:
- `Composable` is a trait, not a struct. Any `#[derive(Lambda)]` type implements it.
- `Composed<T>` is generic over the tree type, not the phase type.
- The Oid of the composition includes the plugin's Oid. Different plugin = different pipeline Oid.

### 6.4 TDD tasks

| # | Red | Green | File |
|---|-----|-------|------|
| 1 | Generated lambda structs compile | Codegen template | codegen tests |
| 2 | Generated pipeline Oid matches hand-written | Deterministic | codegen tests |
| 3 | Plugin lambda composes with standard phases | Cross-crate composition | integration tests |
| 4 | Plugin changes pipeline Oid | Content addressing | integration tests |

---

## File inventory

### prism-core (new files)

| File | Contents |
|------|----------|
| `core/src/lambda/mod.rs` | `Lambda<T>`, `AbsLambda<T>`, `ApplyLambda<T>`, `CaseLambda<T>`, factory methods, `Addressable`, `MerkleTree`, `LambdaImpl` trait |
| `core/src/lambda/reduce.rs` | `ReductionError`, `ReductionLoss`, `Environment<T>`, `reduce`, `reduce_bounded`, `reduce_with_env` |
| `core/src/lambda/compose.rs` | `Composable` trait, `Composed<T>` struct |

### prism-derive (modified)

| File | Change |
|------|--------|
| `derive/src/lib.rs` | Add `#[proc_macro_derive(Lambda)]`, extract `extract_oid_name` as shared helper |

### mirror (new + modified)

| File | Change |
|------|--------|
| `src/lambda_phases.rs` | (new) Phase structs, `LambdaImpl` overrides, static pipelines |
| `src/mirror_runtime.rs` | Add `lambda_env()`, wire native bindings |
| `src/generated.rs` | Add lambda derives to codegen output |
| `src/lib.rs` | `pub mod lambda_phases;` |

---

## How #[derive(Lambda)] extends #[derive(Prism)]

They share the `#[oid("@X")]` attribute and the `extract_oid_name` helper.
They generate the same `Addressable` and `Display` impls. The difference:

| | #[derive(Prism)] | #[derive(Lambda)] |
|---|---|---|
| **Addressable** | yes | yes (same) |
| **Display** | yes | yes (same) |
| **Field optics** | `#[lens]`, `#[prism]`, `#[traversal]`, `#[iso]` | no |
| **Composable** | no | yes |
| **LambdaImpl** | no | yes (default identity) |
| **Use case** | Grammar types, data carriers | Compiler phases, transformations |

A struct CAN derive both:

```rust
#[derive(Prism, Lambda)]
#[oid("@parse")]
pub struct Parse;
```

This gives it field optics AND lambda composition. The `Addressable` impl
is generated once (by whichever derive runs first — Rust deduplicates).

---

## How the mirror compiler becomes named lambda composition

### Before (current)

```rust
// mirror_runtime.rs
pub fn compile_source(&self, source: &str)
    -> Imperfect<CompiledShatter, MirrorRuntimeError, MirrorLoss>
{
    parse_form(source).map(|fragment| CompiledShatter { fragment })
}
```

One function. Pipeline is implicit. Adding a phase means editing the function body.

### After

```rust
// lambda_phases.rs
pub fn compile_source(
    env: &Environment<MirrorFragment>,
    source: &str,
) -> Imperfect<Lambda<MirrorFragment>, ReductionError, ReductionLoss> {
    let source_lambda = Lambda::bind(Oid::hash(source.as_bytes()));
    env.reduce(CRAFT.apply_to(source_lambda), COMPILE_BUDGET)
}
```

The pipeline is `CRAFT` — a static `Composed<MirrorFragment>`. It IS
`Parse.then(Resolve).then(Properties).then(Emit)`. Adding a phase:
define a new `#[derive(Lambda)]` struct, `.then()` it into the pipeline.

The Environment binds each phase's Oid to its Rust implementation.
reduce() encounters `Apply(@parse, source)`, looks up `@parse` in the
environment, calls `parse_form`. Then encounters `Apply(@resolve, parsed)`,
calls the resolver. Each step returns `Imperfect`. Loss accumulates.

---

## Invariants

1. **No strings for identity.** Phase names are Oids, not `&str`.
2. **Composition is content-addressed.** Same phases in same order = same Oid.
3. **Each phase is independently testable.** `env.reduce(Parse.apply_to(input), budget)` tests parsing alone.
4. **Loss accumulates through composition.** Each reduce step's ReductionLoss combines with the next.
5. **The pipeline IS a lambda term.** Not a Vec, not a trait object. A `Lambda::Abs`.
6. **Plugins compose without modifying core.** Define a lambda, `.then()` it.
7. **The Store caches by pipeline Oid + source Oid.** Same compiler + same source = cache hit.
