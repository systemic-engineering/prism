//! Safe Rust wrappers around the Fortran LAPACK subroutines in `core/native/`.
//!
//! All public functions accept row-major `&[f64]` slices, convert to
//! column-major before the Fortran call, and convert results back.  Callers
//! never touch `unsafe` or C types.
//!
//! This module is compiled only when `--features lapack` is active; the
//! Fortran objects are linked by `build.rs` under the same feature gate.

use std::os::raw::c_int;

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
    fn prism_review(
        n: c_int,
        projection: *const f64,
        focus: *const f64,
        result: *mut f64,
    );

    /// (I - P) * source + T * (P * source) → result.
    fn prism_modify(
        n: c_int,
        projection: *const f64,
        source: *const f64,
        transform: *const f64,
        result: *mut f64,
    );

    /// P2 * P1 → composed.
    fn prism_compose(
        n: c_int,
        p1: *const f64,
        p2: *const f64,
        composed: *mut f64,
    );

    /// dsyev('N') — eigenvalues only.
    fn spectral_eigenvalues(
        n: c_int,
        matrix: *const f64,
        eigenvalues: *mut f64,
        info: *mut c_int,
    );

    /// dsyev('V') — eigenvalues + eigenvectors.
    fn spectral_eigensystem(
        n: c_int,
        matrix: *const f64,
        eigenvalues: *mut f64,
        eigenvectors: *mut f64,
        info: *mut c_int,
    );

    /// dgesvd('N','N') — singular values only.
    fn spectral_singular_values(
        m: c_int,
        n: c_int,
        matrix: *const f64,
        singular_values: *mut f64,
        info: *mut c_int,
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
    );
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
/// Returns `Some(focus)` if the projected vector is non-zero (‖focus‖ > 1e-12),
/// `None` if the source is orthogonal to the projection subspace.
pub fn preview(n: usize, projection: &[f64], source: &[f64]) -> Option<Vec<f64>> {
    let proj_cm = row_to_col_major(projection, n, n);
    let mut focus = vec![0.0_f64; n];
    let mut matched: c_int = 0;

    unsafe {
        prism_preview(
            n as c_int,
            proj_cm.as_ptr(),
            source.as_ptr(),
            focus.as_mut_ptr(),
            &mut matched,
        );
    }

    if matched != 0 { Some(focus) } else { None }
}

/// Embed `focus` back into the full space via Pᵀ.
pub fn review(n: usize, projection: &[f64], focus: &[f64]) -> Vec<f64> {
    let proj_cm = row_to_col_major(projection, n, n);
    let mut result = vec![0.0_f64; n];

    unsafe {
        prism_review(
            n as c_int,
            proj_cm.as_ptr(),
            focus.as_ptr(),
            result.as_mut_ptr(),
        );
    }

    result
}

/// Apply `transform` to the projected part of `source`, leave the complement untouched.
///
/// Computes `(I - P) * source + T * (P * source)`.
pub fn modify(n: usize, projection: &[f64], source: &[f64], transform: &[f64]) -> Vec<f64> {
    let proj_cm = row_to_col_major(projection, n, n);
    let xfm_cm  = row_to_col_major(transform,  n, n);
    let mut result = vec![0.0_f64; n];

    unsafe {
        prism_modify(
            n as c_int,
            proj_cm.as_ptr(),
            source.as_ptr(),
            xfm_cm.as_ptr(),
            result.as_mut_ptr(),
        );
    }

    result
}

/// Compose two projection matrices: `P2 * P1`.
///
/// Returns a row-major flat `n×n` matrix.
pub fn compose(n: usize, p1: &[f64], p2: &[f64]) -> Vec<f64> {
    let p1_cm = row_to_col_major(p1, n, n);
    let p2_cm = row_to_col_major(p2, n, n);
    let mut composed_cm = vec![0.0_f64; n * n];

    unsafe {
        prism_compose(
            n as c_int,
            p1_cm.as_ptr(),
            p2_cm.as_ptr(),
            composed_cm.as_mut_ptr(),
        );
    }

    col_to_row_major(&composed_cm, n, n)
}

// ---------------------------------------------------------------------------
// Public safe wrappers — spectral operations
// ---------------------------------------------------------------------------

/// Compute eigenvalues of a real symmetric `n×n` matrix.
///
/// Returns eigenvalues in ascending order.  Input is row-major; for symmetric
/// matrices the layout is invariant under transpose, but we convert for
/// consistency with the rest of the API.
///
/// Returns an empty `Vec` when `n == 0`.
pub fn eigenvalues(n: usize, matrix: &[f64]) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let mat_cm = row_to_col_major(matrix, n, n);
    let mut evals = vec![0.0_f64; n];
    let mut info: c_int = 0;

    unsafe {
        spectral_eigenvalues(
            n as c_int,
            mat_cm.as_ptr(),
            evals.as_mut_ptr(),
            &mut info,
        );
    }

    assert_eq!(info, 0, "spectral_eigenvalues: LAPACK dsyev returned info={info}");
    evals
}

/// Compute eigenvalues and eigenvectors of a real symmetric `n×n` matrix.
///
/// Returns `(eigenvalues, eigenvectors)`.  Eigenvalues are in ascending order.
/// Eigenvectors are returned as a row-major flat `n×n` matrix where each
/// **row** is an eigenvector corresponding to the same-index eigenvalue.
///
/// Note: LAPACK stores eigenvectors as **columns**; the wrapper converts back
/// to row-major so `evecs[i * n .. i * n + n]` is eigenvector `i`.
pub fn eigensystem(n: usize, matrix: &[f64]) -> (Vec<f64>, Vec<f64>) {
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    let mat_cm = row_to_col_major(matrix, n, n);
    let mut evals = vec![0.0_f64; n];
    let mut evecs_cm = vec![0.0_f64; n * n];
    let mut info: c_int = 0;

    unsafe {
        spectral_eigensystem(
            n as c_int,
            mat_cm.as_ptr(),
            evals.as_mut_ptr(),
            evecs_cm.as_mut_ptr(),
            &mut info,
        );
    }

    assert_eq!(info, 0, "spectral_eigensystem: LAPACK dsyev returned info={info}");

    // LAPACK stores eigenvectors as columns of evecs_cm (col-major n×n).
    // col_to_row_major converts so that row i holds eigenvector i.
    let evecs = col_to_row_major(&evecs_cm, n, n);
    (evals, evecs)
}

/// Compute singular values of an `m×n` matrix.
///
/// Returns `min(m, n)` singular values in descending order.
pub fn singular_values(m: usize, n: usize, matrix: &[f64]) -> Vec<f64> {
    let k = m.min(n);
    if k == 0 {
        return Vec::new();
    }
    let mat_cm = row_to_col_major(matrix, m, n);
    let mut svs = vec![0.0_f64; k];
    let mut info: c_int = 0;

    unsafe {
        spectral_singular_values(
            m as c_int,
            n as c_int,
            mat_cm.as_ptr(),
            svs.as_mut_ptr(),
            &mut info,
        );
    }

    assert_eq!(info, 0, "spectral_singular_values: LAPACK dgesvd returned info={info}");
    svs
}

/// Compute the full SVD of an `m×n` matrix.
///
/// Returns `(singular_values, u, vt)` where:
/// - `singular_values` has length `min(m, n)`, in descending order.
/// - `u` is an `m×m` unitary matrix (row-major flat).
/// - `vt` is an `n×n` unitary matrix — V transposed (row-major flat).
pub fn svd(m: usize, n: usize, matrix: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let k = m.min(n);
    if k == 0 {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let mat_cm = row_to_col_major(matrix, m, n);
    let mut svs     = vec![0.0_f64; k];
    let mut u_cm    = vec![0.0_f64; m * m];
    let mut vt_cm   = vec![0.0_f64; n * n];
    let mut info: c_int = 0;

    unsafe {
        spectral_svd(
            m as c_int,
            n as c_int,
            mat_cm.as_ptr(),
            svs.as_mut_ptr(),
            u_cm.as_mut_ptr(),
            vt_cm.as_mut_ptr(),
            &mut info,
        );
    }

    assert_eq!(info, 0, "spectral_svd: LAPACK dgesvd returned info={info}");

    let u  = col_to_row_major(&u_cm,  m, m);
    let vt = col_to_row_major(&vt_cm, n, n);
    (svs, u, vt)
}

// ---------------------------------------------------------------------------
// Tests (only compiled + run with --features lapack; require gfortran + LAPACK)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_identity_is_passthrough() {
        let n = 3;
        let identity = vec![1.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,1.0];
        let source = vec![1.0, 2.0, 3.0];
        let result = preview(n, &identity, &source).unwrap();
        for i in 0..n {
            assert!((result[i] - source[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn preview_orthogonal_returns_none() {
        let px = vec![1.0, 0.0, 0.0, 0.0]; // projects onto x
        let y_only = vec![0.0, 1.0];
        assert!(preview(2, &px, &y_only).is_none());
    }

    #[test]
    fn compose_orthogonal_is_zero() {
        let px = vec![1.0, 0.0, 0.0, 0.0];
        let py = vec![0.0, 0.0, 0.0, 1.0];
        let composed = compose(2, &px, &py);
        for v in &composed {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn eigenvalues_diagonal() {
        let matrix = vec![3.0, 0.0, 0.0, 5.0];
        let evals = eigenvalues(2, &matrix);
        assert!((evals[0] - 3.0).abs() < 1e-10);
        assert!((evals[1] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn eigenvalues_empty() {
        assert!(eigenvalues(0, &[]).is_empty());
    }

    #[test]
    fn eigenvalues_1x1() {
        assert_eq!(eigenvalues(1, &[7.0]), vec![7.0]);
    }

    #[test]
    fn singular_values_identity() {
        let identity = vec![1.0, 0.0, 0.0, 1.0];
        let svs = singular_values(2, 2, &identity);
        for sv in &svs {
            assert!((sv - 1.0).abs() < 1e-10);
        }
    }
}
