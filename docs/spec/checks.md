# The check map: every gate check, its inputs, and whether it is hermetic

*prism, 2026-11-17. prism-020 ask 3 (mirror-Reed's handoff; prism-Reed's Response §2, items 3 and 4). Observation only: this describes the gate as it is at `8de5883`. Nothing here is fixed yet; the to-fix table at the end is the list.*

## The rule this map is checked against

A verdict is cacheable iff it is a deterministic function of its key (Mara's admission test, prism-020 item 4):

```
key = (tool, H(declared inputs), environment, binary)
```

So for each check the questions are:

1. **Tool:** what runs.
2. **Declared inputs:** for a nix check, the `lib.fileset` its source is built from (`flake.nix`). For a `just` recipe, whatever the recipe names.
3. **Files actually read:** found by grepping the tests and build script for `include_str!`, `fs::read*`, `read_dir`, `env::var`/`env!`, `SystemTime`, `Instant`, thread and random sources, network and `HOME`.
4. **Hermetic?** yes, no or unknown. "Yes" means the verdict is a function of the key: every file read is declared, and nothing else (clock, scheduling, load, address layout) can change the answer.

**The darwin sandbox is off.** On the Mac a nix derivation can read any absolute path and reach the network; only convention stops it. Each row says what the Linux sandbox would catch. **The sandbox catches undeclared files and network. It doesn't catch the clock, load or address randomisation.** Those are the to-fix list.

## The filesets (flake.nix)

| Name | Contents |
|---|---|
| `packageFileset` | `Cargo.toml`, `Cargo.lock`, and in each of `prismqueer/`, `imperfect/`, `projections/` every `*.rs`, `*.toml`, `*.f90`, `*.bf`, **minus** `tests/` and `target/` |
| `testFileset` (`testSrc`) | `packageFileset` ∪ each crate's `tests/**/*.rs` ∪ `docs/spec/*.json` |
| `manifestFileset` | `Cargo.toml`, `Cargo.lock`, every member's `Cargo.toml` (for `memberSrc` stubs) |

`imperfect/` (terni) includes `imperfect/macros/` (terni-macros), because `crateFiles` walks the whole directory.

**The checks' environment** (`nativeEnv` + `gate`): rust `1.94.0` from the rust-overlay pin, gfortran, lapack and blas from the nixpkgs pin, `FC=gfortran`, `LAPACK_DIR`/`BLAS_DIR` as store paths, `OPENBLAS_NUM_THREADS=1`, `--locked --workspace --features lapack,fate`, and cargo's dev/test profiles.

## Flake checks (`checks.<system>.*`; systems aarch64-darwin, aarch64-linux, x86_64-linux)

| Check | Tool | Declared inputs | Actually reads | Hermetic? | Why |
|---|---|---|---|---|---|
| `fmt` | `craneLib.cargoFmt` (rustfmt 1.94.0) | `testFileset` | every `*.rs` in it; no `rustfmt.toml` exists | **yes** | A pure function of the sources and the toolchain. |
| `clippy` | `craneLib.cargoClippy --all-targets -- --deny warnings`, over `gateDeps` | `testFileset` + `nativeEnv` | sources; `build.rs` reads `src/fate/brainfuck/*.bf`, `native/{spectral,prism}.f90`, `FC`, `LAPACK_DIR`, `BLAS_DIR`, `AR`, `OUT_DIR`, and runs `gfortran -print-file-name` | **yes** | Everything build.rs reads is in the fileset or the store. Linux sandbox: would catch a build.rs reaching outside the store; none does. |
| `test` | `craneLib.cargoTest` (`cargo test` of the workspace; prismqueer's one `suite` binary, terni's tests, unit tests) | `testFileset` + `nativeEnv` | `docs/spec/{gate,identity,ruler,imperfect}-vectors.json` (via `CARGO_MANIFEST_DIR`); `prismqueer/tests/` listing and `main.rs`, `prismqueer/Cargo.toml` (`suite_manifest`); the `prismqueer/` listing (`build_hygiene`); `PRISM_SPEC_UPDATE` (`spec_vectors`; unset in nix); **the wall clock** (`flux::Sample::new`, `Fate::excited`); **`Instant` timings asserted** (`fate::runtime` tests) | **no** | Every file read is declared, so the Linux sandbox would find nothing to catch. The verdict still isn't a function of the key: clock-seeded tests and wall-time assertions (to-fix 1–4). |
| `coverage` | `craneLib.cargoLlvmCov --fail-under-lines 94 --lcov`, over `coverageDeps` | as `test` | as `test`, instrumented | **no** | Same causes as `test`. Also, the lcov output is not reproducible: clock-seeded runs take different branches, so the measured line set can move between runs (to-fix 5). The timing assertions are likelier to trip under instrumentation and load. |
| `doctest` | `craneLib.cargoDocTest` | as `test` | doc comments in the sources; no doctest reads a file, the clock or the environment (grep) | **yes** | |
| `ratchet` | nix evaluation: `builtins.readFile ./.github/workflows/ci.yml`, matched against `coverageFloor` | `flake.nix`, `.github/workflows/ci.yml` (read at evaluation, from the flake's own source) | the same | **yes** | An evaluation-time constant; the derivation is `touch $out`. |
| `invariance` | nix evaluation: `fs.intersection packageFileset probes` must be empty | `flake.nix` (the filesets and the probe list) | the same | **yes** | Evaluation only. |

## Justfile gate recipes

| Recipe | Tool | Declared inputs | Actually reads | Hermetic? | Why |
|---|---|---|---|---|---|
| `check` | `nix develop -c bash -c 'cargo fmt --check && cargo clippy --workspace --all-targets --features lapack,fate -- --deny warnings'` | **none**: the working tree, as cargo finds it | as `fmt` + `clippy`, but from the working tree: untracked `*.rs` files too, and the incremental `target/` (or `PRISM_TARGET_DIR`) | **no (unkeyed)** | Deterministic in practice, but its key is the working tree plus the devshell, plus whatever the caller's environment adds (`RUSTFLAGS`, `CARGO_*`). It is the fast path of `checks.fmt` + `checks.clippy`, not a cacheable verdict. |
| `test` (plain) | `nix develop -c cargo test --workspace --features lapack,fate` | **none**: the working tree | as `checks.test`, from the working tree; **and the caller's environment**. With `PRISM_SPEC_UPDATE` set, `spec_vectors` *writes* `docs/spec/imperfect-vectors.json` and then passes against what it wrote | **no** | All of `checks.test`'s causes, plus an unkeyed tree and environment (to-fix 6). |
| `test coverage` | `nix eval --impure builtins.currentSystem`; `nix build --dry-run` (hit/miss probe); `nix build checks.<sys>.coverage checks.<sys>.doctest` | the flake's checks, so `testFileset` (taken from git: tracked files, with their uncommitted edits) | as `coverage` + `doctest` | **no** | Inherits `coverage`'s causes. The cache key itself is sound: a hit is served by content address. |
| `pre-push` | `test coverage` | as `test coverage` | as `test coverage` | **no** | The same recipe. |
| `invariance` | `nix/invariance.sh`: copies `git ls-files --recurse-submodules` to a `mktemp` dir, evaluates the packages' and `checks.test`'s drvPaths before and after six probe mutations, then `nix flake check --no-build --all-systems` with IFD off | the tracked tree; `flake.lock` | the same; a temp dir; flake inputs (from the store, or **the network** on a cold store) | **yes** (verdict) | Evaluation only, pinned by `flake.lock`. A cold store fetches the locked inputs, which are content-addressed, so the network changes the time, not the answer. |
| `flake-check` | `nix flake check -L` | every flake check for the current system | as above | **no** | Contains `test` and `coverage`. |

The bench recipes (`bench-gate`, `bench-suite`, `bench`) time things by design and aren't gates.

## Outside the local gate

- **CI** (`.github/workflows/ci.yml`) runs `systemic-engineering/ci/.github/workflows/rust-nix-check.yml@main` and `systemic-engineering/ci/actions/nix-setup@main`. Both are **unpinned** (`@main`), so CI's tool can change without a prism commit. That is a coupling gap, not a hermeticity one, and it's listed in `prism.spec`.

## To fix (not fixed in this step)

| # | Where | What varies | Affects | Fix direction |
|---|---|---|---|---|
| 1 | `prismqueer/src/flux.rs`, `Sample::new` (and `Default`) | seeded from `SystemTime::now()` + a process counter | `tests/pbt_surface.rs` (8 call sites, e.g. `sample_draw_integer_covers_full_range_across_samples`, which passes on a probability, not a proof) | Tests take `Sample::from_bytes` with fixed seeds; `new` stays for exploration outside the gate. |
| 2 | `prismqueer/src/fate/mod.rs`, `Fate::excited` | seeded from `SystemTime::now()` XOR the function's address (ASLR) | unit tests `excited_produces_valid_fate`, `excited_differs_from_untrained`, `two_excited_instances_differ`, `excited_explores_different_models` | A seeded constructor (`excited_from(seed)`) for tests. `two_excited_instances_differ` can fail outright: two calls inside one clock tick (macOS `SystemTime` is microsecond-grained) seed alike. |
| 3 | `prismqueer/src/fate/runtime.rs`, `runtime_performance` | asserts `per_call < 1 ms` from `Instant` | `checks.test`, `checks.coverage` | A timing claim is a bench, not a test: move it to `benches/`, or assert an operation count. |
| 4 | `prismqueer/src/fate/runtime.rs`, `compiled_performance` | asserts compiled ≥ 2× interpreted from `Instant` | as 3; likeliest to trip under coverage instrumentation at high load | as 3 |
| 5 | `checks.coverage` output | the lcov line set moves with 1 and 2 | the ratchet's margin (94 against ~95.3) | Falls out of 1 and 2. |
| 6 | `just test` (plain) | `PRISM_SPEC_UPDATE` from the caller's environment turns `spec_vectors` into a writer | the dev loop only (nix doesn't pass it) | Regenerate through a recipe or a separate binary; the test only compares. |
| 7 | `prismqueer/src/named.rs`, `benchmark_derive_prism_hash` | reads `Instant`, **asserts nothing** on it | nothing (prints only) | Harmless; listed so the grep comes up clean. Move to `benches/` with 3 and 4. |

**Not found:** no test reads a file outside `testFileset`; no network; no `HOME`; no threads spawned by tests; no random-number crate. `HashMap`/`HashSet` (per-process `RandomState`) appear only in test helpers keyed lookups and set-size assertions, which don't depend on iteration order. `fate/brainfuck/test_fate.rs` reads `fate.bf` from the working directory, but it is a standalone `rustc` program that no crate compiles. `fate/train.rs` reads a path it is given, under `fate-training`, which the gate doesn't enable.

**Unknown:** LAPACK/OpenBLAS floating-point results are deterministic for a fixed library build and `OPENBLAS_NUM_THREADS=1`, which the checks fix. Across systems (darwin vs linux, different BLAS kernels) the bits can differ; a test with a tight float tolerance could pass on one and fail on the other. Not audited here.
