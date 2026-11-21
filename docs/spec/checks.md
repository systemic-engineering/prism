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

**Answered (was Unknown): LAPACK float results differ between darwin and Linux, by enough to change a search.** They are deterministic for one library build with `OPENBLAS_NUM_THREADS=1`, which the checks fix. Across systems they aren't. On x86_64-linux CI (run 44378341338) `tests/steer.rs::steering_by_the_spectrum_beats_blind_sampling` failed: "steered Some(0.5102182770720322) did not beat blind 0.4747389814366527". On aarch64-darwin the same test reached the 7-vertex path (0.4450). The blind minimum was the same on both, so only the steered path moved. The cause: the steering score was LAPACK's float for the quietest mode, and graphs whose modes are exactly equal (relabelings, mostly) were ordered by their last bits. A wobble of 1 to 8 ulps per graph moved the darwin search on 52 of 64 salts, and 15 of them landed on 0.510218, the Linux score, on darwin. Then the exact score with first-seen ties gave 0.510218 on darwin as well. Linux had the noise-free answer. Darwin's pass was luck: the noise let the search drift across a plateau of equal scores.

## Floats that decide a discrete outcome

Where a LAPACK float (or prism's Fortran) decides a comparison, sort, argmin/argmax, threshold or count that a verdict, a search, an address or a test depends on. Margins were measured on darwin over the whole suite (`--features lapack,fate`, 2026-11-21) by logging each decision's distance to its edge. Fixtures that sit on a threshold on purpose (the band's own unit tests) are left out. Error unit e = 16·ε·n·max(scale, 1), the declared band.

| Site | What the float decides | Closest margin in the suite | Status |
|---|---|---|---|
| `tests/steer.rs` steering score (`quietest`), through `flux::pillar::steer` | which buffer is best, so which graph is mutated next: the whole path, the best score, the pin | exact ties, decided by the last bit | **Exact.** `room::quietest` gives L₀'s smallest positive eigenvalue as an `Algebraic` (minimal polynomial over ℤ, root index). D's quietest mode is its √, so the order is the same. `Algebraic::compare` orders two of them without reading a float. `steer` takes an `Ord` score, so a float can't steer, and keeps the least (score, address). The float is a reading beside the score and is never compared. The P7 pin is root 0 of y³ − 5y² + 6y − 1. |
| `spectral::instrument::certify` (`Instrument::kernel_dimension`): kernel modes in the zero band, the rest clear of the gap 1/√(n(n−1)) | `Success` vs `Partial`. The steer test's `holds` and `tests/instrument.rs` read it. The count itself is union-find, exact. | kernel modes at most 0.016 e; the next mode at least 3·10¹² e past the gap | **Can't flip by a platform's last bits.** It would need LAPACK's error to grow about 60× on a kernel mode. It is a measured certificate and stays one. |
| `spectral::settle::kernel` | `Wall::Unseparated` vs settled | kernel at most 0.018 e; the gap at least 2.9·10¹² e | Same as `certify`. |
| `Instrument::hodge`: modes sorted by \|λ\|, the first b₀ + b₁ projected | which eigenvectors form the harmonic part | the same separation as `certify` | **Can't flip.** The kernel/non-kernel split is the gap above, and inside the kernel any basis spans the same space. The output is a float cochain, not a discrete value. |
| `Instrument::restrict`: eigenvalue against the window edges, refused within e | `Straddles` vs restricted, and which side each mode falls on | at least 1.35·10¹⁰ e from an edge | **Can't flip in the suite.** In general it's a measured reading with a guard band. It flips only for an eigenvalue within a platform difference of the band's own edge. |
| `spectral::commutator::read`, band zero < 1e-12, nonzero ≥ 1e-6 (× scale) | `Dissonance` `Success` vs `Partial`, and which singular values count | a factor of 10 from either edge; exact zeros are exactly 0 | **Can't flip by ulps.** A measured reading with a don't-know region. |
| `spectral::join::verify_fiedler_rise`: λ₂(composed) > max(λ₂(a), λ₂(b)), raw floats | whether a `Composition` (and so its address) exists | at least 1.0 absolute | **Can't flip in the suite. Not exact in general:** the doc comment already marks it "Uncertified". **Needs a decision**, see below. |
| `spectral::rotation` grain rounding (`Grain::round`, band `zeros::rotation`), over Kuramoto phases from `ffi::phase_lock` (Fortran sin/cos, so libm too) | the rounded phase in an address, or ambiguous | at least e^5.47 ≈ 240× from the band edge | **Can't flip in the suite.** A measured reading with a guard band. The boundary case is refused, and `a_phase_on_the_grains_boundary_is_read_as_ambiguous` pins that. |
| `poly::Roots::locate`, `Factorization::identify`: a measured value ± radius against exact roots | `Root`, `Ambiguous` or `Absent` | not measured; the tests use radii far from neighbouring roots | A measured reading. It refuses rather than guess when two roots are in the window. It flips only when a root sits within a platform difference of the window's edge. |
| `fate::derive::dark_eigensystem` (lapack): LAPACK order reversed, eigenvectors become selector weights | the derived weights, so fate's later argmax routing | eigenvalue gap 0 in `derived_weights_produce_meaningful_routing` (9 tied eigenvalues) | **A reading. It can differ across platforms inside a degenerate eigenspace.** The test only asserts the top, non-degenerate mode (gap 9), so it can't flip. Weights for the tied modes are platform-dependent and must never enter a verdict. Only `fate::derive`'s own unit tests call it. |
| `kleinos::Pushout::fiedler`, `join::fiedler_lambda_2_of_graph` | nothing: sealed `Measurement`s, read beside | - | Readings. The tests compare them with tolerances (`< 1e-9`) or to the same computation on the same machine (bit-equal by construction). |
| `rotation::infer_via_rotation`: `fiedler_at_basepoint < previous_fiedler` | the Foerster-gauge `Fail` | 0: the same float on both sides | **Can't flip.** x < x is false for every non-NaN float. |
| Test tolerances (`tests/{instrument,hodge,window,one_graph,room,spectral_dimension}`, `retain(\|x\| > 1e-9)`, `filter(\|l\| <= band).count()`) | test verdicts | covered by the margins above | Measured readings against declared tolerances, far from their edges. Not loosened. |

**What is exact now:** the steering score and its comparison (`room::quietest`, `Algebraic::compare`), steer's order and tie-break (`Ord` score, least (score, address)), and the P7 closed form. **Guards:** `the_steered_search_does_not_read_the_last_bit` wobbles the float reading 1 to 8 ulps per graph over 16 salts and requires the same path, best, exact score and stop. `among_tied_scores_the_least_address_is_kept` checks the tie-break. `the_quietest_mode_of_p7_is_the_least_root_of_the_odd_cubic` checks the factor choice. **Mutations (2026-11-21), each caught:** comparing by the float reading caught by the wobble test; ties broken by the reading caught by the wobble test; dropping the address tie-break (first seen) caught by the tie test and by beats-blind (0.510218); taking the first irreducible factor in canonical order caught by both P7 pins and the components test.

**Needs Alex's decision:** `verify_fiedler_rise` can be made exact with the same tools: λ₂ is 0 when the graph is disconnected, otherwise `room::quietest`. Two things are open. First, the verdict when the factoring budget runs out (`Partial`? an `Undecided` coordinate?). Second, the budget for large compositions (K_{2n+1} is cheap; arbitrary sheaves may not be). Until then it stays a raw float `>` with at least 1.0 of margin in the suite.
