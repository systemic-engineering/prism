# Lambda Feature Spec for prism-core

**Status:** Research / Design
**Feature flag:** `lambda`
**Date:** 2026-04-15

---

## Thesis

The compiler is lambda calculus on content-addressed trees. Each compilation
step is beta reduction. The five optic operations are the reduction rules.
The Crystal is the normal form. Imperfect is the reduction result.

This document maps the claim precisely, shows what is real mathematics versus
speculation, and specifies what a `lambda` feature flag in prism-core would
provide.

---

## 1. The Mapping

### 1.1 Term Variants to Optic Operations

| Lambda Calculus | prism-core Operation | Semantics |
|----------------|---------------------|-----------|
| **Variable** (`x`) | **Focus** | Look up a binding. Select what matters from context. |
| **Abstraction** (`λx. body`) | **Project** | Extract structure. Threshold cut — the body only makes sense under substitution. |
| **Application** (`f x`) | `smap` / pipeline composition | Apply a transformation to a value. The functor map. |
| **Case/Match** | **Split** (Metal) / `smap` branching | Pattern match. Branch on structure. |
| **Value** (normal form) | **Refract** | The settled result. No more reductions possible. Crystal. |

The three-stage Prism trait maps to a specific reduction pattern:

```
Focus   = bind the input (introduce the variable)
Project = apply the transformation (the lossy step — beta reduction)
Refract = produce the output (reach normal form)
```

This is structurally sound. A Prism IS a single beta-reduction step with
explicit loss tracking. The pipeline `focus | project | refract` IS
`bind | reduce | normalize`.

### 1.2 What Makes This More Than Analogy

The correspondence is structural, not metaphorical, because:

1. **Composition is associative.** Prism composition is a monoid (documented
   in lib.rs). Lambda term composition is associative. Same algebraic
   structure.

2. **Failure is a fixpoint.** Dark beams propagate unchanged through `smap`
   and `next` — the closure is never called. In lambda calculus, a stuck term
   (type error, infinite loop) is a fixpoint of the reduction relation.
   Imperfect::Failure IS a stuck term.

3. **Content addressing makes reduction confluent.** Same input bytes produce
   the same Oid. Same term, same reduction, same normal form. The
   Church-Rosser theorem (confluence of beta reduction) is enforced by the
   content-addressing scheme — there is no evaluation order ambiguity because
   the Oid is the same regardless of reduction path.

4. **Loss tracks non-confluence.** When reduction IS confluent, loss is zero
   (Luminosity::Light). When information is lost during projection, loss is
   nonzero (Luminosity::Dimmed). This is the honest middle: the reduction
   succeeded but something didn't survive. Lambda calculus has no native
   concept of partial success. Imperfect adds it. This is the extension, not
   the analogy.

### 1.3 Where the Mapping Breaks

**Honest boundaries:**

- **Lambda calculus is Turing-complete.** Unrestricted beta reduction can
  diverge. prism-core's Prism trait does not diverge — each step is a
  finite function call. The `reduce_bounded` operation (below) addresses
  this, but it is a restriction of lambda calculus, not lambda calculus
  itself.

- **Lambda calculus has no loss.** The extension from `Result<T, E>` to
  `Imperfect<T, E, L>` is not lambda calculus. It is a three-valued logic
  layered on top. The claim is: the reduction rules are lambda calculus;
  the result tracking is Imperfect. Two layers, not one.

- **Metal's five instructions are not the five Term variants.** Metal
  operates on a byte tape with a data pointer. Lambda calculus operates
  on tree-structured terms. The mapping (Section 5) is a compilation
  target, not an equivalence. A Term compiles TO Metal instructions.
  Metal instructions do not compose as lambda terms.

- **Split and Zoom are not in the Prism trait.** The Prism trait has three
  operations: Focus, Project, Refract. Split and Zoom exist only in Metal
  (as instructions) and in `smap` (as user-space operations on beams).
  The five-way mapping requires looking past the trait boundary.

---

## 2. The Term Type

```rust
/// A term in the lambda calculus. Content-addressed via MerkleTree.
///
/// Feature-gated: available only with `features = ["lambda"]`.
#[derive(Clone, Debug)]
pub enum Term<T: MerkleTree> {
    /// Variable reference. Focus — look at a binding.
    /// The Oid identifies which binding.
    Var(Oid),

    /// Abstraction. Project — extract/abstract a pattern.
    /// Parameter Oid + body term.
    Abs(Oid, Box<Term<T>>),

    /// Application. Zoom — apply function to argument.
    /// Function term + argument term.
    App(Box<Term<T>>, Box<Term<T>>),

    /// Case expression. Split — pattern match on structure.
    /// Scrutinee + list of (pattern, body) arms.
    Case(Box<Term<T>>, Vec<(Pattern<T>, Term<T>)>),

    /// Value. Refract — normal form. A settled MerkleTree node.
    Val(T),
}

/// A pattern for Case expressions.
#[derive(Clone, Debug)]
pub enum Pattern<T: MerkleTree> {
    /// Match a specific value by Oid.
    Literal(Oid),
    /// Bind a variable (wildcard with name).
    Bind(Oid),
    /// Constructor pattern: tag + sub-patterns.
    Constructor(Oid, Vec<Pattern<T>>),
}
```

### 2.1 Term is a MerkleTree

Term<T> must implement MerkleTree. The data at each node is the variant
tag + immediate payload. The children are the sub-terms.

```
Term::Var(x)       → data = "Var", children = []
Term::Abs(x, body) → data = "Abs:x", children = [body]
Term::App(f, a)    → data = "App", children = [f, a]
Term::Case(s, arms)→ data = "Case", children = [s, arm_bodies...]
Term::Val(t)       → data = "Val", children = t.children()
```

The Oid of a Term incorporates all of this recursively. Same term = same Oid.
This means: **terms are content-addressed.** Two programs that are
alpha-equivalent (differ only in variable names) will have different Oids
unless you normalize names first. This is a design choice — de Bruijn indices
would give alpha-equivalence for free, but at the cost of readability.

### 2.2 Term is Addressable

```rust
impl<T: MerkleTree> Addressable for Term<T> {
    fn oid(&self) -> Oid {
        match self {
            Term::Var(x) => Oid::hash(format!("Var:{}", x).as_bytes()),
            Term::Abs(x, body) => {
                Oid::hash(format!("Abs:{}:{}", x, body.oid()).as_bytes())
            }
            Term::App(f, a) => {
                Oid::hash(format!("App:{}:{}", f.oid(), a.oid()).as_bytes())
            }
            Term::Case(s, arms) => {
                let arms_oid: String = arms.iter()
                    .map(|(p, b)| format!("{}:{}", p.oid(), b.oid()))
                    .collect::<Vec<_>>()
                    .join(",");
                Oid::hash(format!("Case:{}:[{}]", s.oid(), arms_oid).as_bytes())
            }
            Term::Val(t) => t.oid(),
        }
    }
}
```

### 2.3 Term is Storable

Because Term<T: MerkleTree> implements Addressable and can implement
MerkleTree, it is a valid `Store::Tree` type. Terms can be persisted to
and retrieved from any Store implementation. This means:

- **Compilation results are cacheable.** Same source Oid = same compiled
  output Oid. The Store becomes a compilation cache for free.
- **Intermediate forms are addressable.** Every reduction step produces a
  Term with a known Oid. The reduction trace is a sequence of Oids.

---

## 3. The Reduction Engine

### 3.1 Beta Reduction

```rust
/// Substitute all free occurrences of `var` in `term` with `replacement`.
fn substitute<T: MerkleTree>(
    term: Term<T>,
    var: &Oid,
    replacement: &Term<T>,
) -> Term<T>;

/// One step of beta reduction.
/// (λx. body) arg  →  body[x := arg]
fn beta_reduce_step<T: MerkleTree>(
    term: Term<T>,
) -> Imperfect<Term<T>, ReductionError, ReductionLoss>;

/// Reduce to normal form. May not terminate for unrestricted terms.
fn reduce<T: MerkleTree>(
    term: Term<T>,
) -> Imperfect<Term<T>, ReductionError, ReductionLoss>;

/// Reduce with a step budget. The sub-Turing gate.
/// Returns Partial with ReductionLoss if budget is exhausted before
/// reaching normal form.
fn reduce_bounded<T: MerkleTree>(
    term: Term<T>,
    budget: usize,
) -> Imperfect<Term<T>, ReductionError, ReductionLoss>;

/// Check if a term is in normal form (no redexes).
fn is_normal<T: MerkleTree>(term: &Term<T>) -> bool;
```

### 3.2 ReductionError and ReductionLoss

```rust
/// What can go wrong during reduction.
#[derive(Debug, Clone)]
pub enum ReductionError {
    /// Unbound variable — Focus found nothing.
    UnboundVariable(Oid),
    /// Type mismatch in application — function is not an Abs.
    NotAFunction(Oid),
    /// No matching arm in Case.
    NonExhaustiveMatch(Oid),
}

/// What can be lost during reduction.
#[derive(Debug, Clone)]
pub struct ReductionLoss {
    /// Steps consumed.
    pub steps: usize,
    /// Budget remaining (None if unbounded).
    pub budget_remaining: Option<usize>,
}
```

ReductionLoss implements `terni::Loss`. The `combine` operation adds steps
and takes the minimum remaining budget. `is_zero` is true when steps == 0.
`total` is steps = usize::MAX, budget = Some(0).

### 3.3 The Budget as Sub-Turing Gate

`reduce_bounded(term, budget)` is the critical function. It reduces the
term for at most `budget` beta-reduction steps. Three outcomes:

| Outcome | Imperfect variant | Meaning |
|---------|------------------|---------|
| Normal form reached | `Success(normal_form)` | Crystal. The term settled. |
| Budget exhausted | `Partial(current_form, loss)` | Not yet normal. Luminosity::Dimmed. |
| Stuck term | `Failure(error, loss)` | Cannot reduce. Luminosity::Dark. |

This is the key design: **lambda calculus is Turing-complete, but
`reduce_bounded` is not.** The budget makes every reduction decidable.
A compiler that uses `reduce_bounded` is a total function — it always
terminates, and the Imperfect result tells you exactly what happened.

This mirrors the conversation crate's design: grammar is sub-Turing by
default, Turing-complete by exception at the boundary.

### 3.4 Reduction Strategy

The reduction strategy matters. Options:

- **Call-by-value** (strict, innermost-first): reduce arguments before
  applying. Matches Rust's evaluation order. Simple to implement.
  Does not find all normal forms (some terms have a normal form under
  call-by-name but diverge under call-by-value).

- **Call-by-name** (lazy, outermost-first): apply first, reduce arguments
  only when needed. Finds normal forms whenever they exist (by the
  standardization theorem). More expensive.

- **Call-by-need** (lazy with sharing): like call-by-name but caches
  results. The content-addressed Store provides this for free — if you
  reduce a sub-term and store the result, looking it up by Oid before
  reducing again IS call-by-need.

**Recommendation:** Call-by-need, using the Store as the sharing mechanism.
The content-addressed Oid of each sub-term acts as the memoization key.
`store.has(sub_term.oid())` checks if this reduction has been done before.
`store.get(reduced_oid)` retrieves the cached result. This falls out of
the existing primitives — no new mechanism needed.

---

## 4. The Compiler as Lambda Composition

### 4.1 Pipeline as Function Composition

If prism-core provides `Term<T>` + `reduce`, then the compiler is:

```rust
fn compile(source: &str) -> Imperfect<Crystal, CompileError, CompileLoss> {
    // Each compiler phase is a lambda.
    // Applying a phase to its input IS beta reduction.

    let source_term = Term::Val(source_node);

    // Phase 1: parse
    let parsed = reduce_bounded(
        Term::App(Box::new(parse_lambda.clone()), Box::new(source_term)),
        PARSE_BUDGET,
    )?;

    // Phase 2: resolve
    let resolved = reduce_bounded(
        Term::App(Box::new(resolve_lambda.clone()), Box::new(parsed)),
        RESOLVE_BUDGET,
    )?;

    // Phase 3: check properties
    let checked = reduce_bounded(
        Term::App(Box::new(check_lambda.clone()), Box::new(resolved)),
        CHECK_BUDGET,
    )?;

    // Phase 4: emit
    let crystal = reduce_bounded(
        Term::App(Box::new(emit_lambda.clone()), Box::new(checked)),
        EMIT_BUDGET,
    )?;

    crystal
}
```

Each `reduce_bounded(App(f, x), budget)` IS `f(x)` with a termination
guarantee. The pipeline IS function composition. The budget per phase
gives fine-grained control over resource allocation.

### 4.2 What This Buys

1. **Phases are interchangeable.** Any lambda with the right type can be
   a compiler phase. Swap `parse_lambda` for a different parser —
   the rest of the pipeline doesn't change.

2. **Phases are content-addressed.** The parse lambda has an Oid. The
   resolve lambda has an Oid. The composition of all four has an Oid.
   If the compiler hasn't changed and the source hasn't changed, the
   result Oid is known without running the compiler.

3. **Phases compose.** `compose(f, g) = λx. f(g(x))` is a Term. You
   can build `compile = compose(emit, compose(check, compose(resolve, parse)))`.
   The composed term has its own Oid. Composition IS a lambda operation,
   not special syntax.

4. **Partial results are meaningful.** If resolve runs out of budget,
   you get `Partial(partially_resolved_ast, loss)`. The loss tells you
   how much budget was consumed. The AST tells you how far resolution got.
   This is not possible with `Result<T, E>`.

### 4.3 The Self-Reference

The compiler-as-lambda can compile ITSELF:

```rust
let compiler_source = Term::Val(compiler_ast_node);
let self_compiled = reduce_bounded(
    Term::App(Box::new(compile_lambda.clone()), Box::new(compiler_source)),
    SELF_COMPILE_BUDGET,
)?;
```

If `self_compiled.oid() == compile_lambda.oid()`, the compiler is a
fixpoint of itself. This is not guaranteed — it depends on whether
the compiler's representation is stable under self-application.
But the FRAMEWORK supports it. Whether any particular compiler
achieves it is an empirical question.

---

## 5. Macros as Lambda Terms

### 5.1 A Macro is an Abstraction

```rust
// A macro that wraps an expression in a logging call
let log_macro = Term::Abs(
    expr_param,
    Term::App(
        Box::new(Term::Val(log_function_node)),
        Box::new(Term::Var(expr_param.clone())),
    ),
);

// Apply the macro to an AST node
let expanded = reduce_bounded(
    Term::App(Box::new(log_macro), Box::new(Term::Val(ast_node))),
    MACRO_BUDGET,
)?;
```

A macro IS a `Term::Abs`. Macro expansion IS beta reduction. The budget
limits how complex a macro can be — preventing macro-generated infinite
expansion.

### 5.2 Pattern-Matching Macros

```rust
// A macro that transforms match expressions
let match_macro = Term::Abs(
    input_param,
    Term::Case(
        Box::new(Term::Var(input_param.clone())),
        vec![
            (Pattern::Constructor(if_oid, vec![Pattern::Bind(cond), Pattern::Bind(body)]),
             transform_if_to_match(cond, body)),
            (Pattern::Bind(fallthrough),
             Term::Var(fallthrough)),
        ],
    ),
);
```

Case expressions in the macro body give pattern-matching macros. The
macro inspects the AST node's structure and transforms accordingly.
This is exactly what Rust proc macros do, but expressed as a lambda
term with content-addressed identity.

### 5.3 Macro Composition

```rust
// Compose two macros: apply log_macro then match_macro
let combined = Term::Abs(
    x,
    Term::App(
        Box::new(match_macro.clone()),
        Box::new(Term::App(
            Box::new(log_macro.clone()),
            Box::new(Term::Var(x.clone())),
        )),
    ),
);
```

Macro composition IS function composition. The composed macro has its own
Oid, distinct from either input macro's Oid. The Store caches it. This
means: **macro expansion is deterministic and cacheable by construction.**

---

## 6. Metal Instruction Mapping

The five Metal instructions are the compilation target for Term variants.
This is a COMPILATION, not an equivalence.

```
Term::Var(x)        → Focus(n)        Read the binding from tape
Term::Abs(x, body)  → Project(t)      Set threshold (the abstraction barrier)
Term::App(f, a)     → Zoom(off, val)  Apply weight (the function application)
Term::Case(s, arms) → Split(n)        Scan cells, branch on nonzero
Term::Val(t)        → Refract          Output the settled value
```

### 6.1 How Compilation Would Work

A `Term<T>` compiles to a `MetalPrism` by traversing the term tree and
emitting instructions:

```rust
fn compile_to_metal<T: MerkleTree>(term: &Term<T>) -> MetalPrism {
    let mut program = Vec::new();
    emit_instructions(term, &mut program);
    MetalPrism::new(program)
}

fn emit_instructions<T: MerkleTree>(term: &Term<T>, program: &mut Vec<Instruction>) {
    match term {
        Term::Var(_) => {
            // Read the variable's value from the tape
            program.push(Instruction::Focus(1));
        }
        Term::Abs(_, body) => {
            // The abstraction sets up a threshold context
            program.push(Instruction::Project(1));
            emit_instructions(body, program);
        }
        Term::App(f, a) => {
            // Evaluate argument, apply function
            emit_instructions(a, program);
            program.push(Instruction::Zoom(0, 1));
            emit_instructions(f, program);
        }
        Term::Case(scrutinee, arms) => {
            emit_instructions(scrutinee, program);
            program.push(Instruction::Split(arms.len()));
            // Each arm emits its body
            for (_, body) in arms {
                emit_instructions(body, program);
            }
        }
        Term::Val(_) => {
            program.push(Instruction::Refract);
        }
    }
}
```

### 6.2 Limitations

This compilation sketch is deliberately incomplete. The real issues:

- **Variable lookup requires an environment.** Metal's tape is flat — there
  is no stack or environment. Implementing variable binding on a flat tape
  requires a calling convention (where do bindings live on the tape?).

- **Metal is forward-only.** The data pointer never moves backward. This
  means closures (lambdas that capture variables from outer scope) require
  copying values forward on the tape. Not impossible, but not free.

- **Split is argmax, not pattern matching.** Metal's Split finds the last
  nonzero cell — it does not match on structure. Real Case compilation
  would need to encode constructor tags as byte values and use
  Project (threshold) to select the right arm.

The mapping shows the SHAPE of the compilation. Making it complete would
require extending Metal (at minimum: a backward-move instruction or an
explicit environment region on the tape).

---

## 7. Type System Connections

### 7.1 Curry-Howard Correspondence

The Curry-Howard correspondence says: types are propositions, programs
are proofs, computation is proof normalization.

In prism-core terms:

| Curry-Howard | prism-core |
|-------------|-----------|
| Type (proposition) | MerkleTree::Data (the node's payload type) |
| Term of a type (proof) | Term<T> that reduces to Val(t) where t has that Data |
| Type checking | Checking that Term<T>'s structure is consistent |
| Proof normalization | `reduce` — beta reduction to normal form |
| Verified proof | Crystal — a settled prism with Luminosity::Light |

**What this means concretely:** if you can construct a `Term<T>` that
reduces to `Success(Val(crystal))`, you have PROVEN that the
transformation from input to output is valid. The Crystal IS the proof
certificate. Its Oid IS the proof's identity.

**What this does NOT mean:** prism-core does not currently have a type
system for Term<T>. The Curry-Howard correspondence is available as a
design direction, not a current feature. Building it would require:

1. A type language for Term (simple types, polymorphic types, or
   dependent types — see Section 7.3).
2. A type checker that verifies Term against its type before reduction.
3. The guarantee that well-typed terms don't get stuck (progress +
   preservation theorems).

### 7.2 Linear Types and pure/real

The `pure != real` distinction (from the mirror architecture) maps to
linear logic:

- **Pure:** can be duplicated, discarded, used multiple times. Classical
  logic / structural rules.
- **Real:** must be used exactly once. Linear logic / resource-sensitive.

In lambda calculus terms:
- A pure value is a term that can be substituted freely (contraction and
  weakening are allowed).
- A real value is a linear variable — must appear exactly once in the body
  of an abstraction.

If Term<T> were extended with linearity annotations:

```rust
enum Term<T: MerkleTree> {
    Var(Oid, Usage),        // Usage: Linear | Unrestricted
    Abs(Oid, Usage, Box<Term<T>>),
    // ...
}

enum Usage {
    Linear,       // real — use exactly once
    Unrestricted, // pure — use any number of times
}
```

Then the reduction engine could enforce: linear variables are substituted
exactly once. Attempting to duplicate a linear variable is a ReductionError.
Attempting to discard one is also a ReductionError.

**Status:** This is speculation. The mirror crate has pure/real as a
concept. prism-core does not enforce it. Linear lambda calculus is
well-studied (Girard 1987, Wadler 1990). The integration would be
straightforward but non-trivial.

### 7.3 System F and Beyond

**System F** (Girard 1972, Reynolds 1974) adds polymorphism to lambda
calculus: terms can abstract over TYPES as well as values. In prism-core,
this would mean Term<T> could express "for any MerkleTree type T, this
transformation works." This is generic prisms — which prism-core already
has at the Rust type level through the `Prism` trait's associated types.

**Calculus of Constructions** (Coquand and Huet 1988) adds dependent
types: types that depend on values. This would mean: the type of a
Term's output can depend on the value of its input. This is the territory
of proof assistants (Coq, Lean, Agda). It is powerful but dramatically
increases complexity.

**Recommendation for prism-core:** Simply-typed lambda calculus is
sufficient for the compiler use case. Each compiler phase has a fixed
input type and output type. Polymorphism (System F) is nice-to-have
for generic prisms but can be done at the Rust level. Dependent types
are overkill for a compiler pipeline.

### 7.4 Cartesian Closed Categories

The connection between lambda calculus and Cartesian Closed Categories
(CCCs) is well-established (Lambek and Scott 1986):

- **Products** (pairs): `(A, B)` — the product of two types. In prism-core,
  a MerkleTree node with two children.
- **Exponentials** (function types): `A → B` — the type of functions
  from A to B. In prism-core, `Term::Abs` with parameter type A and
  body type B.
- **Terminal object** (unit): a type with one value. In prism-core,
  `Oid::dark()` — the unique "nothing" value.

The optics library (prism-core's `optics` feature) already lives in this
space: Lens = get/set pair (product projection), Prism = partial
injection/projection, Traversal = iteration over a product. The framework
crate's composition table (Lens then Prism = AffineTraversal, etc.) IS
the morphism composition table of a specific category.

Adding Term<T> makes this explicit: the lambdas ARE the morphisms. The
MerkleTree types ARE the objects. Composition IS function composition.
The category IS Cartesian Closed because products (tree nodes with
children) and exponentials (Abs terms) exist and compose correctly.

**Status:** Real math, not speculation. The structure is already there.
The lambda feature names it.

---

## 8. Feature Flag Design

### 8.1 Cargo.toml

```toml
[features]
default = []
optics = []
bundle = ["optics"]
lambda = []
lapack = ["dep:cc"]
```

The `lambda` feature has no additional dependencies. It uses only
existing prism-core primitives: Oid, MerkleTree, Store, Imperfect,
Luminosity.

### 8.2 Module Structure

```
prism-core/src/
├── lambda/
│   ├── mod.rs       — Term<T>, Pattern<T>, public API
│   ├── reduce.rs    — beta_reduce_step, reduce, reduce_bounded
│   ├── substitute.rs — substitution, free variable analysis
│   └── normal.rs    — is_normal, normal form detection
└── lib.rs           — #[cfg(feature = "lambda")] pub mod lambda;
```

### 8.3 Public API

```rust
// prism-core with lambda feature
pub use lambda::{Term, Pattern, ReductionError, ReductionLoss};
pub use lambda::{reduce, reduce_bounded, is_normal, substitute};
```

### 8.4 Downstream Usage

```toml
# In mirror's Cargo.toml
[dependencies]
prism-core = { path = "../prism/core", features = ["lambda"] }
```

With `lambda` enabled, mirror's compiler becomes:

- **parse:** a `Term::Abs` that maps source text to MirrorAST
- **resolve:** a `Term::Abs` that maps MirrorAST to resolved MirrorAST
- **properties:** a `Term::Abs` that checks invariants, emits verdicts
- **emit:** a `Term::Abs` that maps resolved AST to target code
- **pipeline:** composition of all four, itself a Term with an Oid
- **cache:** Store lookup by composition Oid + source Oid

---

## 9. The Self-Reference Property

The lambda calculus in prism-core uses all existing primitives:

| Primitive | How Term Uses It |
|-----------|-----------------|
| **Oid** | Every Term has an Oid. Content-addressed identity. |
| **MerkleTree** | Term IS a tree. Children are sub-terms. |
| **Store** | Terms are storable. Reduction results are cacheable. |
| **Imperfect** | Reduction returns Imperfect. Three outcomes. |
| **Luminosity** | Success = Light (normal form). Partial = Dimmed (budget exhausted). Failure = Dark (stuck). |
| **Crystal** | A Term that has reached normal form IS a Crystal. |
| **Beam** | A Term flowing through a reduction pipeline IS a Beam. |
| **Prism** | A single reduction step IS a Prism (focus the redex, project/reduce, refract the result). |
| **Loss** | ReductionLoss tracks steps consumed. Combines across phases. |

The lambda feature does not add a foreign concept. It names a structure
that was already implicit in the existing primitives. The reduction
engine uses Imperfect, the terms use Oid, the cache uses Store.
Everything composes because everything speaks the same protocol.

---

## 10. Prior Art

### 10.1 Direct Ancestors

- **Church (1936):** The lambda calculus. Untyped. Turing-complete. The
  foundation.
- **Curry-Howard (1958/1969):** Types are propositions, programs are
  proofs. The bridge from computation to logic.
- **de Bruijn (1972):** Nameless representation of lambda terms. Solves
  alpha-equivalence. prism-core's Oid-based variable references are a
  different solution to the same problem — but NOT equivalent to de Bruijn
  indices. De Bruijn indices give alpha-equivalence by construction.
  Oid-based names require explicit alpha-normalization.

### 10.2 Type-Theoretic Connections

- **Girard (1972) / Reynolds (1974):** System F. Polymorphic lambda
  calculus. Relevant to generic prisms but not required for the compiler
  use case.
- **Coquand and Huet (1988):** Calculus of Constructions. Dependent types.
  The foundation of Coq/Lean. Overkill for prism-core but theoretically
  within reach if Term<T> is extended with type-level terms.
- **Girard (1987):** Linear logic. Resource-sensitive computation. Maps
  to pure/real distinction. Well-studied, non-trivial to implement.

### 10.3 Implementation Ancestors

- **Haskell's GHC Core:** A compiler intermediate representation based on
  System FC (System F with coercions). Compiler phases ARE transformations
  on Core terms. This is the closest existing implementation to what the
  lambda feature proposes.
- **Nix's expression language:** A lazy, purely functional language that
  evaluates to derivations (build specifications). Content-addressed store
  (the Nix store). Reduction with a fixed evaluation budget. Very close
  to `reduce_bounded` + Store.
- **Unison's content-addressed AST:** Variables are referenced by hash,
  not by name. Same structural idea as Term<T> with Oid-based variables.

### 10.4 Categorical Connections

- **Lambek and Scott (1986):** The correspondence between typed lambda
  calculus and Cartesian Closed Categories. The mathematical foundation
  for why optics and lambda calculus are the same structure viewed from
  different angles.
- **Milewski (2014-2019):** Category Theory for Programmers. Accessible
  treatment of the CCC connection and its programming implications.

---

## 11. What Is Real vs. Speculation

### Real (proven by the existing codebase)

- Prism composition is a monoid. (Documented, tested, lib.rs lines 19-26.)
- Beam is a semifunctor with dark-beam fixpoint. (Tested extensively,
  beam.rs.)
- Content addressing via Oid + MerkleTree produces deterministic identity.
  (Tested, oid.rs, merkle.rs.)
- Imperfect provides three-valued results with loss tracking. (From terni
  crate, used throughout.)
- Metal's five instructions execute on a byte tape. (Tested, metal.rs.)
- Detector compiles to Metal programs. (Tested, coincidence.rs.)
- Store provides content-addressed persistence. (Trait defined, store.rs.)

### Structurally Sound (follows from the above)

- Term<T> as a MerkleTree that implements Addressable.
- Beta reduction returning Imperfect.
- reduce_bounded as a sub-Turing gate.
- Compiler phases as Term::Abs with composition via Term::App.
- Caching via Store using Term Oids.
- The Curry-Howard correspondence at the structural level (types as
  propositions, reductions as proof normalization).

### Speculation (requires design work and proof)

- Whether call-by-need via Store is actually efficient in practice.
- Whether Term<T> compilation to Metal is feasible without extending
  Metal (likely needs at least backward-move or an environment region).
- Whether the type system should be simple types, System F, or linear.
- Whether self-compilation produces a fixpoint (empirical question).
- Whether alpha-normalization via Oid is sufficient or whether de Bruijn
  indices are needed.
- Whether the pure/real distinction maps cleanly onto linear lambda
  calculus in practice.

---

## 12. Implementation Sequence

If this moves from research to implementation:

1. **Term<T> + Addressable + MerkleTree** — the type, its identity, its
   tree structure. No reduction yet. Test: construct terms, verify Oids
   are deterministic, verify MerkleTree properties.

2. **substitute + is_normal** — the substitution engine and normal form
   detection. Test: alpha-equivalence cases, capture-avoiding substitution.

3. **beta_reduce_step** — single-step reduction. Test: simple beta
   reduction, stuck terms produce ReductionError.

4. **reduce_bounded** — bounded reduction with Imperfect results. Test:
   budget exhaustion produces Partial, normal forms produce Success,
   stuck terms produce Failure.

5. **Store integration** — caching reduced terms. Test: reduce, store,
   retrieve by Oid, verify cache hit avoids re-reduction.

6. **Compiler integration** — express mirror's compiler phases as Term::Abs.
   This is the payoff. Test: compile a .mirror file through the lambda
   pipeline, verify same output as the current non-lambda compiler.

Each step is independently testable and independently valuable. Step 1
alone proves the structural claim. Step 4 delivers the sub-Turing gate.
Step 6 delivers the compiler-as-lambda-composition.
