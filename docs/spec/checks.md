# The check map: every gate check, its inputs, and whether it is hermetic

*prism, 2026-11-17. prism-020 ask 3 (mirror-Reed's handoff; prism-Reed's Response §2, items 3 and 4). Observation only: this describes the gate as it is at `8de5883`. The to-fix table at the end is the list; items 1–4, 6 and 7 are fixed on `reed/deterministic-gate`, and item 5 on `reed/lcov-paths` (see its row).*

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
| `test` | `craneLib.cargoTest` (`cargo test` of the workspace; prismqueer's one `suite` binary, terni's tests, unit tests) | `testFileset` + `nativeEnv` | `docs/spec/{gate,identity,ruler,imperfect}-vectors.json` (via `CARGO_MANIFEST_DIR`); `prismqueer/tests/` listing and `main.rs`, `prismqueer/Cargo.toml` (`suite_manifest`); the `prismqueer/` listing (`build_hygiene`); `PRISM_SPEC_UPDATE` (`spec_vectors`; unset in nix; set, it can only fail); every `prismqueer/src/**/*.rs` (`no_clock`). No clock, timer or address is read (fixed: to-fix 1–4, 7) | **yes** | Every file read is declared, and since to-fix 1–4 nothing else (clock, load, address layout) reaches a verdict. `no_clock` keeps it so. |
| `coverage` | `craneLib.cargoLlvmCov --remap-path-prefix --ignore-filename-regex ^target/ --fail-under-lines 94 --lcov`, over `coverageDeps`; `installCheckPhase` runs `nix/lcov-paths.sh` on the output | as `test`, plus `nix/lcov-paths.sh` | as `test`, instrumented | **yes** (verdict and bytes) | As `test`. The lcov is bit-reproducible (to-fix 5): `SF:` paths are repo-relative, and `nix build --rebuild` of the derivation reports no difference. `nix/lcov-paths.sh` fails the build on an absolute `SF:` path, one naming `/nix/var/nix/builds`, one under `target/`, or none at all; `checks.lcov-paths` tests the guard on fixtures. |
| `doctest` | `craneLib.cargoDocTest` | as `test` | doc comments in the sources; no doctest reads a file, the clock or the environment (grep) | **yes** | |
| `ratchet` | nix evaluation: `builtins.readFile ./.github/workflows/ci.yml`, matched against `coverageFloor` | `flake.nix`, `.github/workflows/ci.yml` (read at evaluation, from the flake's own source) | the same | **yes** | An evaluation-time constant; the derivation is `touch $out`. |
| `invariance` | nix evaluation: `fs.intersection packageFileset probes` must be empty | `flake.nix` (the filesets and the probe list) | the same | **yes** | Evaluation only. |

## Justfile gate recipes

| Recipe | Tool | Declared inputs | Actually reads | Hermetic? | Why |
|---|---|---|---|---|---|
| `check` | `nix develop -c bash -c 'cargo fmt --check && cargo clippy --workspace --all-targets --features lapack,fate -- --deny warnings'` | **none**: the working tree, as cargo finds it | as `fmt` + `clippy`, but from the working tree: untracked `*.rs` files too, and the incremental `target/` (or `PRISM_TARGET_DIR`) | **no (unkeyed)** | Deterministic in practice, but its key is the working tree plus the devshell, plus whatever the caller's environment adds (`RUSTFLAGS`, `CARGO_*`). It is the fast path of `checks.fmt` + `checks.clippy`, not a cacheable verdict. |
| `test` (plain) | `nix develop -c cargo test --workspace --features lapack,fate` | **none**: the working tree | as `checks.test`, from the working tree; **and the caller's environment**. With `PRISM_SPEC_UPDATE` set, `spec_vectors` *writes* `docs/spec/imperfect-vectors.json` and then fails ("review the diff and re-run"), so it can't pass on its own output (fixed: to-fix 6) | **no (unkeyed)** | Deterministic as `checks.test` is, but keyed on the working tree and the caller's environment, not a declared input set. |
| `test coverage` | `nix eval --impure builtins.currentSystem`; `nix build --dry-run` (hit/miss probe); `nix build checks.<sys>.coverage checks.<sys>.doctest` | the flake's checks, so `testFileset` (taken from git: tracked files, with their uncommitted edits) | as `coverage` + `doctest` | **no** | Inherits `coverage`'s causes. The cache key itself is sound: a hit is served by content address. |
| `pre-push` | `test coverage` | as `test coverage` | as `test coverage` | **no** | The same recipe. |
| `invariance` | `nix/invariance.sh`: copies `git ls-files --recurse-submodules` to a `mktemp` dir, evaluates the packages' and `checks.test`'s drvPaths before and after six probe mutations, then `nix flake check --no-build --all-systems` with IFD off | the tracked tree; `flake.lock` | the same; a temp dir; flake inputs (from the store, or **the network** on a cold store) | **yes** (verdict) | Evaluation only, pinned by `flake.lock`. A cold store fetches the locked inputs, which are content-addressed, so the network changes the time, not the answer. |
| `flake-check` | `nix flake check -L` | every flake check for the current system | as above | **no** | Contains `test` and `coverage`. |

The bench recipes (`bench-gate`, `bench-suite`, `bench`) time things by design and aren't gates. The wall-clock claims that used to sit in the suite are in `prismqueer/benches/timing.rs` (`just bench timing`).

## Outside the local gate

- **CI** (`.github/workflows/ci.yml`) runs `systemic-engineering/ci/.github/workflows/rust-nix-check.yml@main` and `systemic-engineering/ci/actions/nix-setup@main`. Both are **unpinned** (`@main`), so CI's tool can change without a prism commit. That is a coupling gap, not a hermeticity one, and it's listed in `prism.spec`.

## To fix

Fixed on `reed/deterministic-gate` unless the row says otherwise.

| # | Where | What varied | Fix |
|---|---|---|---|
| 1 | `prismqueer/src/flux.rs`, `Sample::new` (and `Default`) | seeded from `SystemTime::now()` + a process counter; `tests/pbt_surface.rs`'s 8 call sites | **Fixed.** `Sample::new(seed: &[u8])`: an empty Sample whose extension stream is SHA-256(tag ‖ seed). Clock, counter and `Default` removed, no alias. Each `pbt_surface` call passes its test name (and loop index). `forall` and `search` already used `from_bytes`. `determinism` checks one seed draws alike, two seeds draw apart, and pins one seed's stream, so sameness holds across processes. Re-adding a clock seed fails three tests (mutation, 2026-11-17). |
| 2 | `prismqueer/src/fate/mod.rs`, `Fate::excited` | seeded from `SystemTime::now()` XOR the function's address (ASLR) | **Fixed.** `Fate::excited(seed: u64)`; the library reads neither clock nor address. A caller that wants a fresh trajectory per run supplies the entropy. `two_excited_instances_differ` (which could fail inside one clock tick) is now `two_excited_instances_from_two_seeds_differ`. `determinism` pins the models seeds 0–7 pick (discrete, so robust to last-bit float differences across platforms). |
| 3 | `prismqueer/src/fate/runtime.rs`, `runtime_performance` | asserted `per_call < 1 ms` from `Instant` | **Fixed, deterministic.** `runtime_halts_within_its_step_budget`: each transition, over quiet, loud and saturated features, takes at most 100k BF steps, counted by the interpreter (`FateRuntime::steps`; measured 163–167 quiet, about 1,050 loud). The timing is `fate_runtime/interpreted` in `benches/timing.rs`. |
| 4 | `prismqueer/src/fate/runtime.rs`, `compiled_performance` | asserted compiled ≥ 2× interpreted from `Instant` | **Fixed, split.** The deterministic part: `compiled_program_folds_to_at_most_half_the_instructions`, from build.rs's `FATE_BF_INSTRUCTIONS` (300) and `FATE_BF_OPERATIONS` (87). That is structure, not speed; the ≥ 2× claim itself is read off `fate_runtime/{interpreted,compiled}` in `benches/timing.rs`. |
| 5 | `checks.coverage` output | the lcov line set moved with 1 and 2 | **Fixed.** The data was fixed with 1 and 2: every DA/FN/FNDA/BRDA record and the total identical across `nix build --rebuild` (16387/17152 lines, 95.54%, at the time). The bytes still moved, because each `SF:` path carried the darwin build directory (`/nix/var/nix/builds/nix-<pid>-<n>/source/…`). Now cargo-llvm-cov's `--remap-path-prefix` maps the workspace root to `""` in the workspace crates only (its rustc wrapper; `coverageDeps` still serves), so the paths are repo-relative. llvm-cov's `-path-equivalence` doesn't do it: it locates sources, and the exported `SF:` names stay absolute. Relative paths slip past cargo-llvm-cov's absolute target-dir filter, which let the build script's `out/bf_compiled.rs` in (+100 lines), so `--ignore-filename-regex ^target/` keeps it out. The lcov equals the old one byte for byte once its prefix is stripped (81 files, 16573/17338, 95.59% at `fec0214`), and `nix build --rebuild` reports nothing. **Guard:** `nix/lcov-paths.sh` in the derivation's `installCheckPhase`. |
| 6 | `just test` (plain) | `PRISM_SPEC_UPDATE` turned `spec_vectors` into a writer that then passed | **Fixed.** Update mode writes and then fails: "vectors rewritten …; review the diff and re-run without PRISM_SPEC_UPDATE". `update_mode_never_reports_success` checks the verdict function. |
| 7 | `prismqueer/src/named.rs`, `benchmark_derive_prism_hash` | read `Instant`, asserted nothing | **Moved** to `benches/timing.rs` (`derive_prism_oid`), so `prismqueer/src` has no clock reads at all. |

**Guard:** `tests/no_clock.rs` fails if `SystemTime::now` or `Instant::now` appears anywhere in `prismqueer/src` (empty allow-list). It is a suite test, not a `just` recipe, so it runs inside `checks.test` and `checks.coverage`, keyed on `testFileset`.

**Not found:** no test reads a file outside `testFileset`; no network; no `HOME`; no threads spawned by tests; no random-number crate. `HashMap`/`HashSet` (per-process `RandomState`) appear only in test helpers keyed lookups and set-size assertions, which don't depend on iteration order. `fate/brainfuck/test_fate.rs` reads `fate.bf` from the working directory, but it is a standalone `rustc` program that no crate compiles. `fate/train.rs` reads a path it is given, under `fate-training`, which the gate doesn't enable.

**Unknown:** LAPACK/OpenBLAS floating-point results are deterministic for a fixed library build and `OPENBLAS_NUM_THREADS=1`, which the checks fix. Across systems (darwin vs linux, different BLAS kernels) the bits can differ; a test with a tight float tolerance could pass on one and fail on the other. Not audited here.
