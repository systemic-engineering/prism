# LAPACK and Fortran failures as values

prism-020, 2026-11-18. Alex: *"if LAPACK has errors then prism wants to
capture them in the Rust. That's the whole job of the Rust."*

Every way a call into prism's Fortran (`prismqueer/native/`) or the LAPACK
it calls can fail comes back from `prismqueer::ffi` as an `ffi::Fault`, in a
`Result<T, Fault>`, which becomes a `terni::Imperfect<T, Fault, L>` by
`From`. Nothing prism calls STOPs, exits, or reads out of bounds. What can't be
caught in-process is named at the end, with how it is contained.

## The fault

| `Fault`        | What happened                                                 | Caught by |
|----------------|---------------------------------------------------------------|-----------|
| `Argument`     | an argument the routine refuses; `position` is LAPACK's `-INFO` | `Caught::PreCheck` (Rust, before the call), `Caught::Info` (the routine's `INFO < 0`), `Caught::Xerbla(name)` (prism's `xerbla_`) |
| `Shape`        | a slice whose length isn't what the dimensions say            | Rust, before the call |
| `Unconverged`  | the routine ran and didn't converge: `INFO > 0`               | `INFO` |
| `Allocation`   | an `ALLOCATE` in prism's Fortran failed: its `STAT=`; `-1` = the workspace LAPACK asked for exceeds a default integer | `STAT=` |

`INFO < 0` is never success, and never merged with `INFO > 0`: an argument
error and a non-convergence are different answers.

Callers carry the fault, not an `i32`: `room::RoomError::Unmeasured(Fault)`,
`settle::Wall::Eigensystem(Fault)`, `commutator::Unmeasured(Fault)`,
`spectral_dimension::graph_spectral_dimension -> Result<_, Fault>`.

## How LAPACK is linked

`--features lapack`: `build.rs` compiles `native/spectral.f90` and
`native/prism.f90` with `$FC` (gfortran, from the flake's `nativeEnv`) into a
static `libspectral_native.a`, and links LAPACK and BLAS **dynamically**
(`-llapack -lblas`, from `$LAPACK_DIR` and `$BLAS_DIR`).

nixpkgs' `lapack` and `blas` are **OpenBLAS 0.3.33** behind the reference
names: `liblapack.dylib` / `liblapack.so.3` is libopenblas (darwin install
name `libopenblas.0.dylib`), linked against libgfortran. Verified 2026-11-18
(`otool -L`, `objdump -T`).

| platform       | LAPACK                       | `xerbla_` in it                    | internal calls to it          |
|----------------|------------------------------|------------------------------------|-------------------------------|
| aarch64-darwin | `liblapack.dylib` (OpenBLAS) | `T _xerbla_` (strong, exported)   | direct branch: no bind fixup  |
| x86_64-linux   | `liblapack.so.3` (OpenBLAS)  | `w xerbla_` (weak, exported)      | `R_X86_64_JUMP_SLOT xerbla_` (PLT) |

Neither STOPs: OpenBLAS's `xerbla_` is C, `printf(" ** On entry to %6s
parameter number %2d had an illegal value\n")` and return (disassembled on
Linux; observed on darwin). Reference LAPACK's `XERBLA` (`WRITE` then `STOP`)
is not what prism links. If a provider ever swaps in reference LAPACK, its
`STOP` is the one path the pre-check exists to keep prism's calls away from.

## prism's `xerbla_`, per platform

`ffi.rs` defines `#[no_mangle] extern "C" fn xerbla_`: it records the routine
name and argument position for the calling thread (a mutex over thread
ids: `thread_local!` would expand std's source into the crate, and the
coverage check refuses its absolute path) and returns, so the routine
returns `INFO < 0`. Every wrapper clears the record before its call and
checks it after; a record is `Caught::Xerbla(name)`, even when the outer
`INFO` is 0 (an inner routine refused an argument). The wrappers take its
address, so every binary that calls LAPACK through `ffi` links it.

- **Linux: takes effect (predicted, not run here).** The `.so` reaches
  `xerbla_` through the PLT and its own definition is weak, so the
  executable's wins once it is in `.dynsym`. `build.rs` exports it
  explicitly for prismqueer's binaries (`-Wl,--export-dynamic-symbol=xerbla_`).
  The test `ffi::tests::a_bad_argument_reaching_lapack_comes_back_as_info_never_a_stop`
  asserts the record `("DSYEV ", 3)` on Linux. No Linux builder was available
  on 2026-11-18; CI (ubuntu-latest) is the first run of that assertion.
- **darwin: cannot take effect.** Two-level namespace, and more: the dylib's
  `dsyev` branches to its own `_xerbla_` with no bind fixup at all
  (`dyld_info -fixups` lists none for it), so neither a flat-namespace link
  nor `DYLD_FORCE_FLAT_NAMESPACE=1` interposes it. Measured with a C program
  defining `xerbla_` and calling `dsyev_` and `dgesv_` with `N = -1`: INFO
  came back `-3` and `-1`, the override was never called, OpenBLAS printed.
  The same test asserts the override did **not** run on darwin, so if it ever
  starts to, the test fails and this section is out of date.
- **To make it real on darwin:** link OpenBLAS statically (an `.a`, where
  the executable's definition is found first). Not done: a static OpenBLAS
  build is not in the binary cache, and the pre-check already keeps prism's
  calls from reaching `xerbla_`.

## The fallback: refuse first, in Rust

Because the override can't be relied on everywhere, the honest guarantee is
that no call prism makes reaches `xerbla_` at all. Every wrapper checks what
LAPACK (or prism's Fortran) would refuse or would read past, before calling:

| Rust wrapper                                 | Fortran (`native/`)          | LAPACK  | pre-check |
|----------------------------------------------|------------------------------|---------|-----------|
| `eigenvalues`, `eigenvalues_columns`         | `spectral_eigenvalues`       | `dsyev('N','U')` | `N` (arg 3) fits `c_int`; matrix `n×n` |
| `eigensystem`, `eigensystem_columns`         | `spectral_eigensystem`       | `dsyev('V','U')` | same |
| `singular_values`, `singular_values_columns` | `spectral_singular_values`   | `dgesvd('N','N')` | `M` (3), `N` (4) fit `c_int`; matrix `m×n` |
| `svd`                                        | `spectral_svd`               | `dgesvd('A','A')` | same |
| `phase_lock`                                 | `spectral_phase_lock`        | none    | `N ≥ 1` (arg 1), fits `c_int`; `omegas` `n`; `steps` (arg 5) fits `c_int` |
| `preview`, `review`, `modify`, `compose`     | `prism_preview` …            | none    | `n` (arg 1) fits `c_int`; every matrix `n×n`, every vector `n` |

The other arguments are prism's constants (`JOBZ`, `UPLO`, `JOBU`, `JOBVT`)
or follow from the checked ones (`LDA = N`, `LDU = M`, `LDVT = N`, `LWORK`
from LAPACK's own query). `n = 0` still returns empty without a call (dsyev
would refuse `LDA = 0`), after the checks.

Before this, a short slice panicked in the row-major copy or, through the
`_columns` paths in a release build, let Fortran read past it; `steps` past a
C `int` wrapped to a negative count and silently ran zero steps; the four
prism operations had no failure at all.

The Fortran side, for what still comes back (`native/spectral.f90`):

- every wrapper returns `info` (LAPACK's) and `stat` (its own);
- a refused workspace query (`LWORK = -1` with `INFO ≠ 0`) returns at once:
  `WORK(1)` was never written, and allocating its garbage was the old path;
- `WORK(1)` past a default integer (or NaN) is `stat = -1`, not an `int()`
  overflow;
- `spectral_phase_lock` refuses `N < 1` as `info = -1`, LAPACK's way (was
  `info = 1`, which reads as non-convergence).

## Fortran ownership and `STAT=`

prism owns two Fortran sources, compiled by `build.rs` under `lapack`:
`native/spectral.f90` and `native/prism.f90`. Every `ALLOCATE` in them
carries `STAT=` and returns the status:

- `spectral_eigenvalues`: `a(n,n)`, `work(lwork)`;
- `spectral_eigensystem`: `work(lwork)`;
- `spectral_singular_values`, `spectral_svd`: `a(m,n)`, `work(lwork)`;
- `spectral_phase_lock`: `theta(n)`, `dtheta(n)` (were automatic arrays).

`spectral_allocate_probe(count, stat)` exists for the tests: the wrappers'
sizes come from LAPACK, so it is the one way to make an `ALLOCATE` fail on
purpose (`i64::MAX / 2` doubles overflows the byte count; `stat ≠ 0`, the
process goes on).

## NIF boundary

prism has none. `beam/` is a pure Gleam package (`prism_beam`, types only, no
`@external`), there is no `rustler` in `Cargo.lock`, and no crate builds a
`cdylib`. So there is no `catch_unwind` to add or to test here: a panic in
prismqueer reaches whatever embeds it. A crate that wraps prismqueer in a NIF
owns that boundary; rustler's `#[rustler::nif]` wraps each NIF body in
`std::panic::catch_unwind` and raises the panic as an Erlang exception
(rustler's codegen), which covers panics, not the residue below. (The one
rustler crate on this machine, Reed's `body/native/body_tts`, rustler 0.37,
does not depend on prismqueer.)

On the Rust side of prism's own boundary: `xerbla_` is the only Rust function
Fortran calls; it doesn't unwrap (a poisoned lock is still read), because
unwinding out of an `extern "C"` function aborts.

## Residue: what can't be caught in-process

| what | where it comes from | caught? |
|------|---------------------|---------|
| **compiler temporaries** | `matmul`/`transpose` results and array expressions in `prism.f90` (`prism_modify`'s `identity(n,n)`, `projected`, `complement` are automatic arrays) and in the `spectral` copies; gfortran allocates them without `STAT=` | no: a failure is a libgfortran runtime error, which exits |
| **OOM kill** | the kernel (Linux OOM killer, darwin jetsam) under memory pressure; `malloc` succeeding lazily and the page fault failing later | no: the process is killed |
| **stack overflow** | deep recursion in Rust, large automatic arrays if gfortran ever places them on the stack (`-fstack-arrays`, implied by `-Ofast`; `build.rs` uses `-O2`) | no: SIGSEGV on the guard page, or Rust's "stack overflow" abort |
| **abort** | `panic = "abort"` builds, a panic inside a panic, unwinding out of `extern "C"`, OpenBLAS's own internal `abort()`s (e.g. thread-buffer allocation failure: "BLAS : Program is Terminated") | no |
| **UB in the provider** | a LAPACK or OpenBLAS bug | no |
| **`expect` on a fault** | callers that turn a `Fault` back into a panic: `fate::derive::dark_eigensystem` (`"DSYEV convergence failure"`), `fate`'s `preview` call (shape fixed by type, so unreachable) | in-process, as a panic, not a value |

**Containment.** None of this can be turned into a value inside the process
that suffers it; it can only be kept from taking down something else.

- Under a BEAM embedding: run LAPACK-calling NIFs on **dirty CPU schedulers**
  so a long `dsyev` doesn't block a normal scheduler, and bound the input
  (`n`) before the call so the workspace stays within a known budget. That
  keeps the VM responsive; it does not survive an abort or an OOM kill,
  which take the VM with them.
- For the residue only: a **port** (a separate OS process speaking to the VM
  over a pipe) around the LAPACK calls. An abort, stack overflow or OOM kill
  then kills the port, the VM gets an `EXIT` and can turn it into an error
  value and restart the port. The price is a copy of every matrix across the
  pipe. Documented, not built: prism has no BEAM embedding yet.
- Outside the BEAM: the same split, a child process with its own resource
  limits (`ulimit -v`/`-s`, cgroups on Linux) for calls whose size isn't
  bounded in advance.

## Tests

`prismqueer/src/ffi.rs` (unit) and `prismqueer/tests/lapack_faults.rs`:

- **bad argument reaching LAPACK**: `spectral_eigenvalues`,
  `spectral_eigensystem`, `spectral_singular_values`, `spectral_svd` with a
  dimension of `-1`, through prism's Fortran: `INFO = -3`, `stat = 0`, the
  process goes on; `xerbla_` asserted per platform; `check` reads it as
  `Argument { position: 3 }`.
- **bad argument caught first**: dimensions past `c_int`, wrong-length
  slices for every wrapper, `phase_lock` with no oscillators or with `steps`
  past `c_int`.
- **allocation failure**: the probe; `check` reads the status as
  `Allocation`.
- **INFO read three ways**: `0` ok, `< 0` argument, `> 0` unconverged, `stat`
  first, an inner routine's `xerbla_` record a fault even with `INFO = 0`.
- **panic**: no NIF boundary to test; the paths that panicked before (a short
  slice in the row-major copy) now return `Shape`.

Mutations, each made to fail a test and reverted: the shape pre-check
removed; `INFO < 0` read as success. Removing the `xerbla_` override is
caught on Linux only (darwin never calls it).
