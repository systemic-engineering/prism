//! Kernel specification — what the runtime executes.
//!
//! Fate's forward pass produces logits. KernelSpec interprets those logits
//! as a structured kernel specification: which dimensions to preserve,
//! what decomposition to apply, what precision floor.
//!
//! Always available (no feature gate). The `lapack` feature controls
//! whether dispatch goes to Fortran or Rust — the spec is just a type.

use crate::Precision;

/// Which decomposition the kernel should apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decomposition {
    /// Eigenvalue decomposition (dsyev). For symmetric/spectral data.
    Eigenvalue,
    /// Singular value decomposition (dgesvd). For general matrices.
    Svd,
    /// Matrix-vector multiply only. Cheapest path.
    MatVec,
    /// Full projection: preview + modify + review cycle.
    FullProjection,
}

/// The specification that Fate's routing produces.
/// Determines what the runtime executes — Fortran or Rust.
#[derive(Clone, Debug)]
pub struct KernelSpec {
    /// Which dimensions to preserve during transport.
    /// Derived from logits: dimensions where activation > threshold.
    pub dimensions: Vec<usize>,
    /// Which decomposition to apply.
    pub decomposition: Decomposition,
    /// Precision floor — eigenvalues below this are zeroed.
    pub precision: Precision,
}

impl KernelSpec {
    /// Create a kernel spec with explicit dimensions.
    pub fn new(dimensions: Vec<usize>, decomposition: Decomposition, precision: Precision) -> Self {
        KernelSpec { dimensions, decomposition, precision }
    }

    /// Construct from logits: dimensions where logit > threshold are preserved.
    /// The logits ARE the dimension selector.
    pub fn from_logits(
        logits: &[f64],
        threshold: f64,
        decomposition: Decomposition,
        precision: Precision,
    ) -> Self {
        let dimensions: Vec<usize> = logits.iter()
            .enumerate()
            .filter(|(_, &l)| l > threshold)
            .map(|(i, _)| i)
            .collect();
        KernelSpec { dimensions, decomposition, precision }
    }

    /// Number of preserved dimensions.
    pub fn rank(&self) -> usize {
        self.dimensions.len()
    }

    /// Build a diagonal projection matrix (n×n, row-major) that preserves
    /// only the specified dimensions. Everything else is zeroed.
    pub fn projection_matrix(&self, n: usize) -> Vec<f64> {
        let mut matrix = vec![0.0f64; n * n];
        for &d in &self.dimensions {
            if d < n {
                matrix[d * n + d] = 1.0;
            }
        }
        matrix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_spec_from_dimensions() {
        let spec = KernelSpec::new(
            vec![0, 2, 4, 6],
            Decomposition::Eigenvalue,
            Precision::new(0.01),
        );
        assert_eq!(spec.dimensions.len(), 4);
        assert_eq!(spec.decomposition, Decomposition::Eigenvalue);
        assert_eq!(spec.rank(), 4);
    }

    #[test]
    fn kernel_spec_from_logits_filters_by_threshold() {
        let logits = [1.0, -0.5, 2.0, -1.0, 0.3, -0.2, 1.5, -0.8,
                      0.1, -0.3, 0.8, -0.1, 0.5, -0.4, 1.2, -0.6];
        let spec = KernelSpec::from_logits(&logits, 0.0, Decomposition::Eigenvalue, Precision::new(0.01));
        // Positive logits at indices: 0, 2, 4, 6, 8, 10, 12, 14
        assert_eq!(spec.dimensions, vec![0, 2, 4, 6, 8, 10, 12, 14]);
        assert_eq!(spec.rank(), 8);
    }

    #[test]
    fn kernel_spec_from_logits_high_threshold() {
        let logits = [1.0, -0.5, 2.0, -1.0, 0.3, -0.2, 1.5, -0.8,
                      0.1, -0.3, 0.8, -0.1, 0.5, -0.4, 1.2, -0.6];
        let spec = KernelSpec::from_logits(&logits, 1.0, Decomposition::Svd, Precision::new(0.1));
        // Logits > 1.0: indices 2 (2.0), 6 (1.5), 14 (1.2)
        assert_eq!(spec.dimensions, vec![2, 6, 14]);
        assert_eq!(spec.decomposition, Decomposition::Svd);
    }

    #[test]
    fn projection_matrix_diagonal() {
        let spec = KernelSpec::new(
            vec![0, 2],
            Decomposition::MatVec,
            Precision::new(0.01),
        );
        let matrix = spec.projection_matrix(4);
        // 4×4 matrix, 1s at (0,0) and (2,2)
        assert_eq!(matrix[0 * 4 + 0], 1.0);  // (0,0)
        assert_eq!(matrix[1 * 4 + 1], 0.0);  // (1,1) not preserved
        assert_eq!(matrix[2 * 4 + 2], 1.0);  // (2,2)
        assert_eq!(matrix[3 * 4 + 3], 0.0);  // (3,3) not preserved
    }

    #[test]
    fn projection_matrix_out_of_bounds_ignored() {
        let spec = KernelSpec::new(
            vec![0, 5],  // 5 is out of bounds for n=4
            Decomposition::MatVec,
            Precision::new(0.01),
        );
        let matrix = spec.projection_matrix(4);
        assert_eq!(matrix[0 * 4 + 0], 1.0);
        // Index 5 silently ignored (d < n check)
        let sum: f64 = matrix.iter().sum();
        assert_eq!(sum, 1.0);  // Only one 1.0 in the matrix
    }

    #[test]
    fn empty_logits_empty_spec() {
        let spec = KernelSpec::from_logits(&[], 0.0, Decomposition::Eigenvalue, Precision::new(0.01));
        assert_eq!(spec.rank(), 0);
        assert!(spec.dimensions.is_empty());
    }
}
