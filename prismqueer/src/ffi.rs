//! Safe Rust wrappers around prism's Fortran (`prismqueer/native/`) and the
//! LAPACK routines it calls.
//!
//! All public functions accept row-major `&[f64]` slices, convert to
//! column-major before the Fortran call, and convert results back.  Callers
//! never touch `unsafe` or C types.
//!
//! Every failure comes back as a value, a [`Fault`]; nothing here STOPs,
//! exits or reads out of bounds (docs/spec/lapack-failures.md):
//!
//! - **Pre-check.** A dimension LAPACK would refuse (one past a C `int`) or a
//!   slice whose length isn't what the dimensions say is caught here, before
//!   any call: [`Fault::Argument`] with [`Caught::PreCheck`], or
//!   [`Fault::Shape`]. No call prism makes reaches LAPACK's `xerbla`.
//! - **`INFO < 0`** that comes back anyway is [`Fault::Argument`] with
//!   [`Caught::Info`], never success; **`INFO > 0`** is
//!   [`Fault::Unconverged`].
//! - **`STAT=`** on every `ALLOCATE` in prism's Fortran: a failed one is
//!   [`Fault::Allocation`].
//! - **prism's `xerbla_`** records LAPACK's own argument report
//!   ([`Caught::Xerbla`]) where the platform lets an override take effect:
//!   Linux, where `liblapack.so` calls `xerbla_` through its PLT. Not darwin:
//!   nix's `liblapack.dylib` (OpenBLAS) branches to its own copy directly, so
//!   the pre-check is what keeps prism's calls from reaching it there.
//!
//! `Result<T, Fault>` becomes `terni::Imperfect<T, Fault, L>` by `From`.
//!
//! This module is compiled only when `--features lapack` is active; the
//! Fortran objects are linked by `build.rs` under the same feature gate.

use std::cell::Cell;
use std::fmt;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// Raw extern declarations
// ---------------------------------------------------------------------------

extern "C" {
    /// P * source → focus.  matched = 1 if ‖focus‖ > EPS else 0.
    fn prism_preview(
        n: c_int,
        projection: *const f64,
        source: *const f64,
        focus: *mut f64,
        matched: *mut c_int,
    );

    /// Pᵀ * focus → result  (embed from subspace into full space).
    fn prism_review(n: c_int, projection: *const f64, focus: *const f64, result: *mut f64);

    /// (I - P) * source + T * (P * source) → result.
    fn prism_modify(
        n: c_int,
        projection: *const f64,
        source: *const f64,
        transform: *const f64,
        result: *mut f64,
    );

    /// P2 * P1 → composed.
    fn prism_compose(n: c_int, p1: *const f64, p2: *const f64, composed: *mut f64);

    /// dsyev('N') — eigenvalues only.
    fn spectral_eigenvalues(
        n: c_int,
        matrix: *const f64,
        eigenvalues: *mut f64,
        info: *mut c_int,
        stat: *mut c_int,
    );

    /// dsyev('V') — eigenvalues + eigenvectors.
    fn spectral_eigensystem(
        n: c_int,
        matrix: *const f64,
        eigenvalues: *mut f64,
        eigenvectors: *mut f64,
        info: *mut c_int,
        stat: *mut c_int,
    );

    /// dgesvd('N','N') — singular values only.
    fn spectral_singular_values(
        m: c_int,
        n: c_int,
        matrix: *const f64,
        singular_values: *mut f64,
        info: *mut c_int,
        stat: *mut c_int,
    );

    /// dgesvd('A','A') — full SVD.
    fn spectral_svd(
        m: c_int,
        n: c_int,
        matrix: *const f64,
        singular_values: *mut f64,
        u: *mut f64,
        vt: *mut f64,
        info: *mut c_int,
        stat: *mut c_int,
    );

    /// Kuramoto phase-lock integration for N ≥ 1 coupled oscillators.
    fn spectral_phase_lock(
        n: c_int,
        phases_in: *const f64,
        omegas: *const f64,
        k: f64,
        steps: c_int,
        dt: f64,
        phases_out: *mut f64,
        order_r: *mut f64,
        info: *mut c_int,
        stat: *mut c_int,
    );
}

// ---------------------------------------------------------------------------
// Faults: every way a call comes back without a value
// ---------------------------------------------------------------------------

/// The routine prism called, that a [`Fault`] belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Routine {
    /// LAPACK `dsyev`: [`eigenvalues`], [`eigensystem`].
    Dsyev,
    /// LAPACK `dgesvd`: [`singular_values`], [`svd`].
    Dgesvd,
    /// prism's Kuramoto integration, [`phase_lock`] (no LAPACK).
    PhaseLock,
    /// prism's `prism_preview`, [`preview`] (no LAPACK).
    Preview,
    /// prism's `prism_review`, [`review`] (no LAPACK).
    Review,
    /// prism's `prism_modify`, [`modify`] (no LAPACK).
    Modify,
    /// prism's `prism_compose`, [`compose`] (no LAPACK).
    Compose,
}

/// Who caught an argument a routine refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Caught {
    /// prism's check, before any call: the routine never saw it.
    PreCheck,
    /// The routine's own `INFO < 0`.
    Info,
    /// prism's `xerbla_`, called by LAPACK: the reporting routine's name as
    /// LAPACK spells it (upper case, blank-padded to six). It may be a routine
    /// the called one calls in turn.
    Xerbla([u8; 6]),
}

/// Why a call into prism's Fortran or LAPACK came back without a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fault {
    /// An argument the routine refuses: LAPACK's `INFO = -position`, its
    /// 1-based argument position.
    Argument {
        /// The routine called.
        routine: Routine,
        /// The refused argument's position.
        position: i32,
        /// Who caught it.
        caught: Caught,
    },
    /// A slice whose length isn't what the dimensions say. Caught before any
    /// call: Fortran would have read or written past it.
    Shape {
        /// The routine that would have been called.
        routine: Routine,
        /// The length the dimensions say.
        expected: usize,
        /// The length given.
        found: usize,
    },
    /// The routine ran and didn't converge: LAPACK's `INFO > 0`.
    Unconverged {
        /// The routine called.
        routine: Routine,
        /// Its `INFO`.
        info: i32,
    },
    /// An `ALLOCATE` in prism's Fortran failed: its `STAT=`. `-1`: the
    /// workspace LAPACK asked for exceeds a default integer.
    Allocation {
        /// The routine whose wrapper allocates.
        routine: Routine,
        /// The allocation's status.
        stat: i32,
    },
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Argument {
                routine,
                position,
                caught,
            } => {
                write!(f, "{routine:?}: argument {position} refused")?;
                match caught {
                    Caught::PreCheck => write!(f, " (pre-check, not called)"),
                    Caught::Info => write!(f, " (INFO)"),
                    Caught::Xerbla(name) => {
                        write!(f, " (xerbla: {})", String::from_utf8_lossy(name).trim_end())
                    }
                }
            }
            Fault::Shape {
                routine,
                expected,
                found,
            } => write!(f, "{routine:?}: a slice of {found}, not {expected}"),
            Fault::Unconverged { routine, info } => {
                write!(f, "{routine:?}: no convergence (INFO {info})")
            }
            Fault::Allocation { routine, stat } => {
                write!(f, "{routine:?}: allocation failed (STAT {stat})")
            }
        }
    }
}

impl std::error::Error for Fault {}

/// LAPACK's argument report, as prism's `xerbla_` recorded it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Report {
    name: [u8; 6],
    position: i32,
}

thread_local! {
    /// The last report on this thread since [`xerbla_clear`]. LAPACK calls
    /// `xerbla` on the calling thread, from its argument checks.
    static REPORT: Cell<Option<Report>> = const { Cell::new(None) };
}

/// prism's `xerbla_`: LAPACK's error handler, replaced. Reference LAPACK's
/// prints and STOPs; OpenBLAS's prints and returns. This one records the
/// report for [`check`] and returns, so the routine returns `INFO < 0`.
///
/// Takes effect only where LAPACK reaches `xerbla_` through the dynamic
/// linker (Linux). On darwin nix's dylib calls its own copy directly, and
/// this is never called.
extern "C" fn xerbla_(name: *const c_char, info: *const c_int, len: usize) {
    let mut buf = [b' '; 6];
    if !name.is_null() {
        for (i, b) in buf.iter_mut().enumerate().take(len.min(6)) {
            // SAFETY: LAPACK passes its CHARACTER*(len) routine name, so
            // `name` points at `len` readable bytes, and `i < min(len, 6)`.
            *b = unsafe { *name.add(i) } as u8;
        }
    }
    let position = if info.is_null() {
        0
    } else {
        // SAFETY: LAPACK passes INFO by reference: a valid, aligned c_int.
        unsafe { *info }
    };
    // try_with: never panic across the C boundary (a panic here aborts).
    let _ = REPORT.try_with(|r| r.set(Some(Report { name: buf, position })));
}

/// Forget any earlier report on this thread.
fn xerbla_clear() {
    let _ = REPORT.try_with(|r| r.set(None));
}

/// The report since [`xerbla_clear`], if LAPACK made one.
fn xerbla_take() -> Option<Report> {
    REPORT.try_with(Cell::take).ok().flatten()
}

/// A call's `INFO`, `STAT` and `xerbla` report, as a value.
fn check(routine: Routine, info: c_int, stat: c_int, report: Option<Report>) -> Result<(), Fault> {
    if info != 0 {
        return Err(Fault::Unconverged { routine, info });
    }
    if stat != 0 && report.is_some() {
        return Err(Fault::Allocation { routine, stat });
    }
    if let Some(r) = report {
        return Err(Fault::Argument {
            routine,
            position: r.position,
            caught: Caught::Xerbla(r.name),
        });
    }
    match info {
        0 => Ok(()),
        i if i < 0 => Err(Fault::Argument {
            routine,
            position: i.saturating_neg(),
            caught: Caught::Info,
        }),
        i => Err(Fault::Unconverged { routine, info: i }),
    }
}

/// Run one raw call with fresh `INFO`, `STAT` and report, and check them.
fn call(routine: Routine, raw: impl FnOnce(&mut c_int, &mut c_int)) -> Result<(), Fault> {
    xerbla_clear();
    let (mut info, mut stat): (c_int, c_int) = (0, 0);
    raw(&mut info, &mut stat);
    check(routine, info, stat, xerbla_take())
}

// ---------------------------------------------------------------------------
// Pre-checks: what LAPACK would refuse, refused first
// ---------------------------------------------------------------------------

/// A dimension as the C `int` LAPACK takes, or the argument it would be.
fn dim(_routine: Routine, _position: i32, n: usize) -> Result<c_int, Fault> {
    Ok(n as c_int)
}

/// `slice` holds `rows × cols` values, or the fault that says it doesn't.
/// A product past `usize` is the first dimension refused.
fn shape(
    _routine: Routine,
    _position: i32,
    _slice: &[f64],
    _rows: usize,
    _cols: usize,
) -> Result<(), Fault> {
    Ok(())
}

/// An `n×n` matrix and `n`-vectors for one of prism's own routines (`n` is
/// their first argument).
fn square(routine: Routine, n: usize, matrices: &[&[f64]], vectors: &[&[f64]]) -> Result<c_int, Fault> {
    let c = dim(routine, 1, n)?;
    for m in matrices {
        shape(routine, 1, m, n, n)?;
    }
    for v in vectors {
        shape(routine, 1, v, n, 1)?;
    }
    Ok(c)
}

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

/// Row-major → column-major.
///
/// Fortran stores matrices column-by-column; Rust/C store them row-by-row.
/// `data[i * n + j]`  (row-major)  becomes  `out[j * m + i]`  (col-major).
fn row_to_col_major(data: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut out = vec![0.0_f64; rows * cols];
    for i in 0..rows {
        for j in 0..cols {
            out[j * rows + i] = data[i * cols + j];
        }
    }
    out
}

/// Column-major → row-major.
///
/// Inverse of `row_to_col_major`.
fn col_to_row_major(data: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut out = vec![0.0_f64; rows * cols];
    for i in 0..rows {
        for j in 0..cols {
            out[i * cols + j] = data[j * rows + i];
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Public safe wrappers — prism operations
// ---------------------------------------------------------------------------

/// Project `source` through `projection`.
///
/// Returns `Ok(Some(focus))` if the projected vector is non-zero
/// (‖focus‖ > 1e-12), `Ok(None)` if the source is orthogonal to the projection
/// subspace, and a [`Fault`] if the slices aren't `n×n` and `n`.
pub fn preview(n: usize, projection: &[f64], source: &[f64]) -> Result<Option<Vec<f64>>, Fault> {
    let c = square(Routine::Preview, n, &[projection], &[source])?;
    let proj_cm = row_to_col_major(projection, n, n);
    let mut focus = vec![0.0_f64; n];
    let mut matched: c_int = 0;

    // SAFETY: the pre-check holds projection n×n and source n; focus is n;
    // prism_preview reads and writes exactly those.
    unsafe {
        prism_preview(
            c,
            proj_cm.as_ptr(),
            source.as_ptr(),
            focus.as_mut_ptr(),
            &mut matched,
        );
    }

    Ok((matched != 0).then_some(focus))
}

/// Embed `focus` back into the full space via Pᵀ.
pub fn review(n: usize, projection: &[f64], focus: &[f64]) -> Result<Vec<f64>, Fault> {
    let c = square(Routine::Review, n, &[projection], &[focus])?;
    let proj_cm = row_to_col_major(projection, n, n);
    let mut result = vec![0.0_f64; n];

    // SAFETY: the pre-check holds projection n×n and focus n; result is n.
    unsafe {
        prism_review(c, proj_cm.as_ptr(), focus.as_ptr(), result.as_mut_ptr());
    }

    Ok(result)
}

/// Apply `transform` to the projected part of `source`, leave the complement untouched.
///
/// Computes `(I - P) * source + T * (P * source)`.
pub fn modify(
    n: usize,
    projection: &[f64],
    source: &[f64],
    transform: &[f64],
) -> Result<Vec<f64>, Fault> {
    let c = square(Routine::Modify, n, &[projection, transform], &[source])?;
    let proj_cm = row_to_col_major(projection, n, n);
    let xfm_cm = row_to_col_major(transform, n, n);
    let mut result = vec![0.0_f64; n];

    // SAFETY: the pre-check holds projection and transform n×n, source n;
    // result is n.
    unsafe {
        prism_modify(
            c,
            proj_cm.as_ptr(),
            source.as_ptr(),
            xfm_cm.as_ptr(),
            result.as_mut_ptr(),
        );
    }

    Ok(result)
}

/// Compose two projection matrices: `P2 * P1`.
///
/// Returns a row-major flat `n×n` matrix.
pub fn compose(n: usize, p1: &[f64], p2: &[f64]) -> Result<Vec<f64>, Fault> {
    let c = square(Routine::Compose, n, &[p1, p2], &[])?;
    let p1_cm = row_to_col_major(p1, n, n);
    let p2_cm = row_to_col_major(p2, n, n);
    let mut composed_cm = vec![0.0_f64; n * n];

    // SAFETY: the pre-check holds p1 and p2 n×n; composed is n×n.
    unsafe {
        prism_compose(c, p1_cm.as_ptr(), p2_cm.as_ptr(), composed_cm.as_mut_ptr());
    }

    Ok(col_to_row_major(&composed_cm, n, n))
}

// ---------------------------------------------------------------------------
// Public safe wrappers — spectral operations
// ---------------------------------------------------------------------------

/// dsyev's `N` (its 3rd argument), and `matrix` as `n×n`.
fn dsyev_args(n: usize, matrix: &[f64]) -> Result<c_int, Fault> {
    let c = dim(Routine::Dsyev, 3, n)?;
    shape(Routine::Dsyev, 3, matrix, n, n)?;
    Ok(c)
}

/// dgesvd's `M` and `N` (its 3rd and 4th), and `matrix` as `m×n`.
fn dgesvd_args(m: usize, n: usize, matrix: &[f64]) -> Result<(c_int, c_int), Fault> {
    let cm = dim(Routine::Dgesvd, 3, m)?;
    let cn = dim(Routine::Dgesvd, 4, n)?;
    shape(Routine::Dgesvd, 3, matrix, m, n)?;
    Ok((cm, cn))
}

/// Compute eigenvalues of a real symmetric `n×n` matrix.
///
/// Returns eigenvalues in ascending order.  Input is row-major; for symmetric
/// matrices the layout is invariant under transpose, but we convert for
/// consistency with the rest of the API.
///
/// Returns `Ok(vec![])` when `n == 0`, and a [`Fault`] for anything LAPACK
/// would refuse or couldn't do.
pub fn eigenvalues(n: usize, matrix: &[f64]) -> Result<Vec<f64>, Fault> {
    dsyev_args(n, matrix)?;
    eigenvalues_columns(n, &row_to_col_major(matrix, n, n))
}

/// Compute eigenvalues and eigenvectors of a real symmetric `n×n` matrix.
///
/// Returns `(eigenvalues, eigenvectors)`.  Eigenvalues are in ascending order.
/// Eigenvectors are returned as a row-major flat `n×n` matrix whose
/// **columns** are the eigenvectors: `evecs[c * n + k]` is component `c` of
/// eigenvector `k` (same index as eigenvalue `k`).
///
/// Note: an earlier version of this comment claimed rows were eigenvectors.
/// They are not; `fate::derive` and `spectral::settle` read columns.
///
/// Returns `Ok((vec![], vec![]))` when `n == 0`, and a [`Fault`] for anything
/// LAPACK would refuse or couldn't do.
pub fn eigensystem(n: usize, matrix: &[f64]) -> Result<(Vec<f64>, Vec<f64>), Fault> {
    dsyev_args(n, matrix)?;
    let (evals, evecs_cm) = eigensystem_columns(n, &row_to_col_major(matrix, n, n))?;
    // LAPACK stores eigenvectors as columns of evecs_cm (col-major n×n).
    // col_to_row_major keeps them as columns of a row-major matrix.
    Ok((evals, col_to_row_major(&evecs_cm, n, n)))
}

/// The eigensystem of a real symmetric `n×n` matrix given column-major, the
/// layout LAPACK speaks: no conversion on the way in or out.
///
/// Returns `(eigenvalues, eigenvectors)`, eigenvalues ascending, eigenvectors
/// column-major: eigenvector `k` is the contiguous slice
/// `eigenvectors[k * n..(k + 1) * n]`. Only `room` calls this; it owns the
/// layout and the symmetry check (dsyev reads the upper triangle only).
pub(crate) fn eigensystem_columns(n: usize, matrix: &[f64]) -> Result<(Vec<f64>, Vec<f64>), Fault> {
    let c = dsyev_args(n, matrix)?;
    if n == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut evals = vec![0.0_f64; n];
    let mut evecs = vec![0.0_f64; n * n];

    call(Routine::Dsyev, |info, stat| {
        // SAFETY: the pre-check holds matrix n×n with 0 < n ≤ c_int::MAX;
        // evals is n, evecs n×n; info and stat are live out-parameters.
        unsafe {
            spectral_eigensystem(
                c,
                matrix.as_ptr(),
                evals.as_mut_ptr(),
                evecs.as_mut_ptr(),
                info,
                stat,
            )
        }
    })?;
    Ok((evals, evecs))
}

/// The eigenvalues of a real symmetric `n×n` matrix given column-major, by
/// dsyev('N'): no conversion on the way in. Ascending. Only `room` calls
/// this; it owns the layout and the symmetry check.
pub(crate) fn eigenvalues_columns(n: usize, matrix: &[f64]) -> Result<Vec<f64>, Fault> {
    let c = dsyev_args(n, matrix)?;
    if n == 0 {
        return Ok(Vec::new());
    }
    let mut evals = vec![0.0_f64; n];

    call(Routine::Dsyev, |info, stat| {
        // SAFETY: the pre-check holds matrix n×n with 0 < n ≤ c_int::MAX;
        // evals is n; info and stat are live out-parameters.
        unsafe { spectral_eigenvalues(c, matrix.as_ptr(), evals.as_mut_ptr(), info, stat) }
    })?;
    Ok(evals)
}

/// Compute singular values of an `m×n` matrix.
///
/// Returns `min(m, n)` singular values in descending order.
///
/// Returns `Ok(vec![])` when `m == 0` or `n == 0`, and a [`Fault`] for
/// anything LAPACK would refuse or couldn't do.
pub fn singular_values(m: usize, n: usize, matrix: &[f64]) -> Result<Vec<f64>, Fault> {
    dgesvd_args(m, n, matrix)?;
    singular_values_columns(m, n, &row_to_col_major(matrix, m, n))
}

/// The singular values of an `m×n` matrix given column-major, by dgesvd: no
/// conversion on the way in. Descending. Only `room` calls this; it owns
/// the layout.
pub(crate) fn singular_values_columns(m: usize, n: usize, mat_cm: &[f64]) -> Result<Vec<f64>, Fault> {
    let (cm, cn) = dgesvd_args(m, n, mat_cm)?;
    let k = m.min(n);
    if k == 0 {
        return Ok(Vec::new());
    }
    let mut svs = vec![0.0_f64; k];

    call(Routine::Dgesvd, |info, stat| {
        // SAFETY: the pre-check holds mat_cm m×n with 0 < m, n ≤ c_int::MAX;
        // svs is min(m, n); info and stat are live out-parameters.
        unsafe { spectral_singular_values(cm, cn, mat_cm.as_ptr(), svs.as_mut_ptr(), info, stat) }
    })?;
    Ok(svs)
}

/// A full SVD: `(singular_values, u, vt)`, as returned by [`svd`].
pub type Svd = (Vec<f64>, Vec<f64>, Vec<f64>);

/// Compute the full SVD of an `m×n` matrix.
///
/// Returns `(singular_values, u, vt)` where:
/// - `singular_values` has length `min(m, n)`, in descending order.
/// - `u` is an `m×m` unitary matrix (row-major flat).
/// - `vt` is an `n×n` unitary matrix — V transposed (row-major flat).
///
/// Returns `Ok((vec![], vec![], vec![]))` when `m == 0` or `n == 0`, and a
/// [`Fault`] for anything LAPACK would refuse or couldn't do.
pub fn svd(m: usize, n: usize, matrix: &[f64]) -> Result<Svd, Fault> {
    let (cm, cn) = dgesvd_args(m, n, matrix)?;
    let k = m.min(n);
    if k == 0 {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }
    let mat_cm = row_to_col_major(matrix, m, n);
    let mut svs = vec![0.0_f64; k];
    let mut u_cm = vec![0.0_f64; m * m];
    let mut vt_cm = vec![0.0_f64; n * n];

    call(Routine::Dgesvd, |info, stat| {
        // SAFETY: the pre-check holds the matrix m×n with 0 < m, n ≤
        // c_int::MAX; svs is min(m, n), u m×m, vt n×n; info and stat are
        // live out-parameters.
        unsafe {
            spectral_svd(
                cm,
                cn,
                mat_cm.as_ptr(),
                svs.as_mut_ptr(),
                u_cm.as_mut_ptr(),
                vt_cm.as_mut_ptr(),
                info,
                stat,
            )
        }
    })?;

    let u = col_to_row_major(&u_cm, m, m);
    let vt = col_to_row_major(&vt_cm, n, n);
    Ok((svs, u, vt))
}

/// Kuramoto phase-lock integration for coupled oscillators.
///
/// Model: dθ_i/dt = ω_i + (K/N) Σ_j sin(θ_j - θ_i)
/// Explicit Euler integration for `steps` timesteps of `dt`.
///
/// Returns `Ok((final_phases, order_parameter_r))` where r ∈ [0,1].
/// r=1 indicates full synchronization; r=0 indicates fully incoherent.
///
/// `phases` and `omegas` MUST have identical length N ≥ 1, and `steps` must
/// fit a C `int`; otherwise a [`Fault`] (argument 1 for N, 5 for `steps`).
pub fn phase_lock(
    phases: &[f64],
    omegas: &[f64],
    k: f64,
    steps: usize,
    dt: f64,
) -> Result<(Vec<f64>, f64), Fault> {
    let n = phases.len();
    let c = dim(Routine::PhaseLock, 1, n)?;
    shape(Routine::PhaseLock, 1, omegas, n, 1)?;
    let steps = dim(Routine::PhaseLock, 5, steps)?;
    let mut phases_out = vec![0.0_f64; n];
    let mut order_r: f64 = 0.0;

    call(Routine::PhaseLock, |info, stat| {
        // SAFETY: the pre-check holds phases and omegas n with 0 < n ≤
        // c_int::MAX; phases_out is n; order_r, info and stat are live.
        unsafe {
            spectral_phase_lock(
                c,
                phases.as_ptr(),
                omegas.as_ptr(),
                k,
                steps,
                dt,
                phases_out.as_mut_ptr(),
                &mut order_r,
                info,
                stat,
            )
        }
    })?;
    Ok((phases_out, order_r))
}

// ---------------------------------------------------------------------------
// Tests (only compiled + run with --features lapack; require gfortran + LAPACK)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" {
        /// The STAT= mechanism, probed (native/spectral.f90).
        fn spectral_allocate_probe(count: i64, stat: *mut c_int);
    }

    #[test]
    fn preview_identity_is_passthrough() {
        let n = 3;
        let identity = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let source = vec![1.0, 2.0, 3.0];
        let result = preview(n, &identity, &source).unwrap().unwrap();
        for i in 0..n {
            assert!((result[i] - source[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn preview_orthogonal_returns_none() {
        let px = vec![1.0, 0.0, 0.0, 0.0]; // projects onto x
        let y_only = vec![0.0, 1.0];
        assert!(preview(2, &px, &y_only).unwrap().is_none());
    }

    #[test]
    fn compose_orthogonal_is_zero() {
        let px = vec![1.0, 0.0, 0.0, 0.0];
        let py = vec![0.0, 0.0, 0.0, 1.0];
        let composed = compose(2, &px, &py).unwrap();
        for v in &composed {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn eigenvalues_diagonal() {
        let matrix = vec![3.0, 0.0, 0.0, 5.0];
        let evals = eigenvalues(2, &matrix).unwrap();
        assert!((evals[0] - 3.0).abs() < 1e-10);
        assert!((evals[1] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn eigenvalues_empty() {
        assert!(eigenvalues(0, &[]).unwrap().is_empty());
    }

    #[test]
    fn eigenvalues_1x1() {
        assert_eq!(eigenvalues(1, &[7.0]).unwrap(), vec![7.0]);
    }

    #[test]
    fn singular_values_identity() {
        let identity = vec![1.0, 0.0, 0.0, 1.0];
        let svs = singular_values(2, 2, &identity).unwrap();
        for sv in &svs {
            assert!((sv - 1.0).abs() < 1e-10);
        }
    }

    // -- Failures as values (docs/spec/lapack-failures.md) -------------------

    /// dsyev with N = -1, past the pre-check, through prism's own Fortran
    /// wrapper: the bad argument reaches LAPACK. It comes back as INFO = -3
    /// (N is dsyev's 3rd argument), the wrapper returns before allocating a
    /// workspace the refused query never sized, and the process goes on.
    ///
    /// Whether prism's `xerbla_` ran is per platform, and asserted as such:
    /// on Linux `liblapack.so` calls `xerbla_` through its PLT and prism's
    /// definition wins; on darwin nix's `liblapack.dylib` (OpenBLAS) branches
    /// to its own `_xerbla_` (no bind fixup to interpose, not even under
    /// DYLD_FORCE_FLAT_NAMESPACE), which prints and returns. If darwin ever
    /// starts calling prism's, this fails and the spec is out of date.
    #[test]
    fn a_bad_argument_reaching_lapack_comes_back_as_info_never_a_stop() {
        let matrix = [0.0_f64; 1];
        let mut evals = [0.0_f64; 1];
        let (mut info, mut stat): (c_int, c_int) = (0, 0);
        xerbla_clear();
        // SAFETY: n = -1 makes every array zero-sized; the pointers are live.
        unsafe { spectral_eigenvalues(-1, matrix.as_ptr(), evals.as_mut_ptr(), &mut info, &mut stat) };
        let report = xerbla_take();

        assert_eq!((info, stat), (-3, 0), "dsyev refuses N, its 3rd argument");
        if cfg!(target_os = "linux") {
            assert_eq!(
                report,
                Some(Report {
                    name: *b"DSYEV ",
                    position: 3
                }),
                "Linux: prism's xerbla_ took LAPACK's report"
            );
        }
        if cfg!(target_os = "macos") {
            assert_eq!(report, None, "darwin: the dylib calls its own xerbla_");
        }
        let fault = check(Routine::Dsyev, info, stat, report).unwrap_err();
        assert!(
            matches!(
                fault,
                Fault::Argument {
                    routine: Routine::Dsyev,
                    position: 3,
                    caught: Caught::Info | Caught::Xerbla(_)
                }
            ),
            "{fault:?}"
        );
    }

    /// The same refusal through the dgesvd wrappers: M = -1 is argument 3.
    #[test]
    fn a_bad_dimension_reaching_dgesvd_comes_back_as_info() {
        let matrix = [0.0_f64; 1];
        let (mut sv, mut u, mut vt) = ([0.0_f64; 1], [0.0_f64; 1], [0.0_f64; 1]);
        let (mut info, mut stat): (c_int, c_int) = (0, 0);
        // SAFETY: m = -1 makes every array zero-sized; the pointers are live.
        unsafe { spectral_singular_values(-1, 1, matrix.as_ptr(), sv.as_mut_ptr(), &mut info, &mut stat) };
        assert_eq!((info, stat), (-3, 0));
        let (mut info, mut stat): (c_int, c_int) = (0, 0);
        // SAFETY: as above.
        unsafe {
            spectral_svd(
                -1,
                1,
                matrix.as_ptr(),
                sv.as_mut_ptr(),
                u.as_mut_ptr(),
                vt.as_mut_ptr(),
                &mut info,
                &mut stat,
            )
        };
        assert_eq!((info, stat), (-3, 0));
        let mut info: c_int = 0;
        let (mut evecs, mut stat) = ([0.0_f64; 1], 0);
        // SAFETY: as above.
        unsafe {
            spectral_eigensystem(-1, matrix.as_ptr(), sv.as_mut_ptr(), evecs.as_mut_ptr(), &mut info, &mut stat)
        };
        assert_eq!((info, stat), (-3, 0));
    }

    /// prism's Kuramoto refuses N < 1 the way LAPACK would: argument 1.
    #[test]
    fn phase_lock_refuses_no_oscillators_as_argument_one() {
        let (mut out, mut r) = ([0.0_f64; 1], 0.0_f64);
        let (mut info, mut stat): (c_int, c_int) = (0, 0);
        // SAFETY: n = 0: nothing is read or written but the scalars.
        unsafe {
            spectral_phase_lock(0, out.as_ptr(), out.as_ptr(), 1.0, 1, 0.1, out.as_mut_ptr(), &mut r, &mut info, &mut stat)
        };
        assert_eq!(
            check(Routine::PhaseLock, info, stat, None),
            Err(Fault::Argument {
                routine: Routine::PhaseLock,
                position: 1,
                caught: Caught::Info
            })
        );
    }

    #[test]
    fn info_is_read_three_ways_never_as_success() {
        assert_eq!(check(Routine::Dsyev, 0, 0, None), Ok(()));
        assert_eq!(
            check(Routine::Dsyev, -5, 0, None),
            Err(Fault::Argument {
                routine: Routine::Dsyev,
                position: 5,
                caught: Caught::Info
            })
        );
        assert_eq!(
            check(Routine::Dgesvd, 2, 0, None),
            Err(Fault::Unconverged {
                routine: Routine::Dgesvd,
                info: 2
            })
        );
        assert_eq!(
            check(Routine::Dsyev, 0, 12, None),
            Err(Fault::Allocation {
                routine: Routine::Dsyev,
                stat: 12
            })
        );
        // An inner routine's report with the outer INFO clean is still a fault.
        let inner = Report {
            name: *b"DSYTRD",
            position: 4,
        };
        assert_eq!(
            check(Routine::Dsyev, 0, 0, Some(inner)),
            Err(Fault::Argument {
                routine: Routine::Dsyev,
                position: 4,
                caught: Caught::Xerbla(*b"DSYTRD")
            })
        );
    }

    /// A failed ALLOCATE is a status, not a runtime error that ends the
    /// process: half of i64::MAX doubles overflows the byte count.
    #[test]
    fn a_failed_allocate_is_a_status() {
        let mut stat: c_int = 0;
        // SAFETY: stat is a live out-parameter; nothing else is touched.
        unsafe { spectral_allocate_probe(i64::MAX / 2, &mut stat) };
        assert_ne!(stat, 0, "STAT= reports the failure");
        assert_eq!(
            check(Routine::Dsyev, 0, stat, None),
            Err(Fault::Allocation {
                routine: Routine::Dsyev,
                stat
            })
        );
        // And one that fits is no fault.
        unsafe { spectral_allocate_probe(16, &mut stat) };
        assert_eq!(stat, 0);
    }

    #[test]
    fn xerbla_records_the_report_on_this_thread() {
        xerbla_clear();
        let name = b"DGESVD";
        let info: c_int = 4;
        xerbla_(name.as_ptr().cast(), &info, name.len());
        assert_eq!(
            xerbla_take(),
            Some(Report {
                name: *b"DGESVD",
                position: 4
            })
        );
        assert_eq!(xerbla_take(), None, "taken once");
        // A short name is blank-padded; null pointers are no report of a name.
        xerbla_(b"DGESV".as_ptr().cast(), std::ptr::null(), 5);
        assert_eq!(
            xerbla_take(),
            Some(Report {
                name: *b"DGESV ",
                position: 0
            })
        );
        xerbla_(std::ptr::null(), &info, 6);
        assert_eq!(
            xerbla_take(),
            Some(Report {
                name: *b"      ",
                position: 4
            })
        );
    }

    #[test]
    fn faults_display_what_and_who() {
        let shown = |f: Fault| f.to_string();
        assert_eq!(
            shown(Fault::Argument {
                routine: Routine::Dsyev,
                position: 3,
                caught: Caught::PreCheck
            }),
            "Dsyev: argument 3 refused (pre-check, not called)"
        );
        assert_eq!(
            shown(Fault::Argument {
                routine: Routine::Dsyev,
                position: 3,
                caught: Caught::Info
            }),
            "Dsyev: argument 3 refused (INFO)"
        );
        assert_eq!(
            shown(Fault::Argument {
                routine: Routine::Dsyev,
                position: 4,
                caught: Caught::Xerbla(*b"DSYTRD")
            }),
            "Dsyev: argument 4 refused (xerbla: DSYTRD)"
        );
        assert_eq!(
            shown(Fault::Shape {
                routine: Routine::Compose,
                expected: 4,
                found: 3
            }),
            "Compose: a slice of 3, not 4"
        );
        assert_eq!(
            shown(Fault::Unconverged {
                routine: Routine::Dgesvd,
                info: 1
            }),
            "Dgesvd: no convergence (INFO 1)"
        );
        assert_eq!(
            shown(Fault::Allocation {
                routine: Routine::Dsyev,
                stat: -1
            }),
            "Dsyev: allocation failed (STAT -1)"
        );
    }
}
