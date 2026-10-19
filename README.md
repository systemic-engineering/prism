# prism

[![CI](https://github.com/systemic-engineering/prism/actions/workflows/ci.yml/badge.svg)](https://github.com/systemic-engineering/prism/actions/workflows/ci.yml)

prism is the numerical and algebraic core under the [mirror](https://github.com/systemic-engineering/mirror) research compiler. It is a Rust workspace of three crates. **prismqueer** provides typed transformation pipelines (`Prism`, `Beam`, optics) and the spectral graph machinery mirror reasons with: Laplacian heat flow ("settle"), a Hodge–Dirac operator, commutators, and histories up to reordering. All of it is backed by LAPACK and returns results that carry their own loss. **terni** provides `Imperfect<T, E, L>`, a `Result` with a third state, Partial(value, loss). **prismqueer-projections** provides the derive macros.

## Status

**Research code. Pre-1.0, no API stability.** Published versions trail the repository: `prismqueer` 0.1.1, `terni` 0.7.0 and `prismqueer-projections` 0.1.0 on crates.io.

Most recent work is in `prismqueer` (September 2026), developed as RED → GREEN test pairs: `spectral::settle`, `spectral::instrument` (Hodge–Dirac), `time` (trace monoid), and `flux::pillar::search` / `steer`. The latest milestone, "O3", has prismqueer's own spectral operator steer the property search that tests that operator.

Verified on 2026-10-19 (branch `reed/settle-loop-1`, macOS arm64, `nix develop`):

| Command | Result |
|---|---|
| `cargo test -p prismqueer --features lapack,bundle` | 637 passed, 18 failed, 1 ignored |
| `cargo test --workspace --features prismqueer/lapack,prismqueer/bundle,prismqueer/lambda,prismqueer/pq` | 999 passed, 18 failed, 9 ignored |
| `just test` (`--features optics` only) | does not compile: `tests/red_spectral_compose_paper_3_6_four_properties.rs` needs `lapack` |
| `--features fate` | the library builds; its test module does not compile (stale imports in `fate/runtime.rs` tests) |

The 18 failures are deliberate. `tests/red_narcissus_battery_five_op_collapse.rs` (9) and `tests/red_trust_chain_flux_void.rs` (9) are RED batteries: they specify behaviour that has not been implemented yet.

The dev shell (`flake.nix`) currently takes its Rust toolchain from a local flake path (`path:/Users/alexwolf/.flakes`), so `nix develop` works only on the author's machine. Elsewhere you need stable Rust, `gfortran` and LAPACK/BLAS. That route has not been tested.

## Quickstart

```sh
git clone https://github.com/systemic-engineering/prism
cd prism
git submodule update --init          # imperfect/ (terni)
nix develop                          # Rust, gfortran, LAPACK/BLAS

cargo test -p prismqueer --features lapack,bundle     # 637 passed, 18 failed (RED), 1 ignored
```

Minimal use of the pipeline API (verified to compile and run against the current tree):

```rust
use prismqueer::{apply, Beam, Optic, Prism};

// focus splits the input into chars, project counts them, settle renders the count.
struct CountPrism;

impl Prism for CountPrism {
    type Input     = Optic<(), String>;
    type Focused   = Optic<String, Vec<char>>;
    type Projected = Optic<Vec<char>, usize>;
    type Refracted = Optic<usize, String>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let s = beam.result().ok().unwrap().clone();
        beam.next(s.chars().collect())
    }
    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let n = beam.result().ok().unwrap().len();
        beam.next(n)
    }
    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        let n = *beam.result().ok().unwrap();
        beam.next(format!("{n} chars"))
    }
}

let result = apply(&CountPrism, Optic::ok((), "hello".to_string()));
assert_eq!(result.result().ok(), Some(&"5 chars".to_string()));
```

Each stage's output type is the next stage's input type. A mismatch is a compile error.

## Structure

```
prismqueer/            the core crate
  src/beam.rs, optics/   Prism / Beam / Optic; classical optics (feature `optics`)
  src/bundle.rs          principal-bundle tower: Fiber → Connection → Gauge → Transport (feature `bundle`)
  src/spectral/          (feature `lapack`) settle, instrument (Hodge–Dirac), commutator, kleinos (graph composition), tension
  src/time/              Progression: histories in the trace monoid, Foata normal form
  src/flux.rs            property verdicts; pillar::search and pillar::steer (property search that keeps counterexamples)
  src/fate/              the five-mode selector (feature `fate`)
  src/oid.rs, store.rs, merkle.rs   content addressing
  src/pq/                typed query DSL with JSON Schema (feature `pq`)
  native/                Fortran wrappers compiled by build.rs for the LAPACK path
  tests/                 24 integration test files, RED batteries included
imperfect/             submodule: terni, the Imperfect<T, E, L> type
projections/           prismqueer-projections: #[derive(Prism)], #[derive(Lambda)], declaration!{}
beam/                  Gleam package with the Prism types for the BEAM (April 2026, dormant)
docs/math/FLOOR.md     mathematical architecture of prismqueer
docs/architecture.md   how the layers feed each other (loss flows up, decisions flow down)
docs/specs/            pq, lambda and coincidence-hash specs
```

## Core ideas

- **Measurable loss.** Every stage returns `Imperfect`: Success, Partial(value, loss) or Failure(error, loss). Loss types implement a `Loss` trait with algebraic laws, and `tests/loss_laws.rs` audits every public loss against those laws.
- **Settle = where the recursion stops moving.** `spectral::settle` runs heat flow `e^{-tL}` on a graph Laplacian in closed form from the eigenbasis. It returns Success if the input was already harmonic, Partial with what is still moving if not, and Failure if the eigensolver could not certify the answer. It fails closed: an unmeasured answer is never reported as settled.
- **Hodge–Dirac operator.** `spectral::instrument` builds `D = δ + δ*` on vertex ⊕ edge values. `ker D = H⁰ ⊕ H¹` separates agreements that settle from disagreements no assignment of vertex values can remove.
- **Histories up to reordering.** `time::Progression` stores a history as an element of a trace monoid in Foata normal form, so independent steps commute and every observer's ordering gets the same address.
- **Content addressing.** OIDs are content hashes; `Store` and `MerkleTree` are typed over them.
- **`fate`, a small fixed selector.** It is 425 `u8` parameters: five weight sets, each a 5-way linear classifier over 16 features (5×16 weights + 5 biases). It is interpreted by a fixed Brainfuck program over a 256-byte tape. The f64 training form has 450 parameters. Code: `prismqueer/src/fate/weights.rs`, `runtime.rs`.
- **Sub-Turing by design.** The intent is that inference runs in a state space bounded up front. This is a design constraint, argued in `docs/math/FLOOR.md` §1.3. It is not machine-checked.

The mathematics is developed in [`docs/math/FLOOR.md`](./docs/math/FLOOR.md) and in mirror's [`docs/math/`](https://github.com/systemic-engineering/mirror/tree/main/docs/math). The module docs cite the specific sections they implement.

## How it's built

Same workflow as mirror. Named AI agents with SSH-signed commit identities work under Alex Wolf's architectural decisions. Implementation goes in RED → GREEN commit pairs: a failing test is committed first, and the next commit makes it pass. Commits per author (`git shortlog -sne`, as of 2026-10-19): Reed 261 (implementation, tests), Mara 16 (math, `FLOOR.md`), Glint 4, Taut 2, Seam 1. Adversarial reviews from the April 2026 optics migration are in `docs/seam-taut-review-2026-04-08.md` and `docs/seam-taut-rereview-2026-04-08.md`.

## License

Apache-2.0 across the workspace. See `prismqueer/LICENSE` and `projections/LICENSE`; terni declares Apache-2.0 in its `Cargo.toml`. mirror itself is layered (Apache-2.0 compiler, SEL for corpus and deployments; see its [LICENSE.md](https://github.com/systemic-engineering/mirror/blob/main/LICENSE.md)).

## Related

- [mirror](https://github.com/systemic-engineering/mirror): the compiler that composes over prismqueer.
- [imperfect](https://github.com/systemic-engineering/imperfect): terni, standalone.
