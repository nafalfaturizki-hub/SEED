#![forbid(unsafe_code)]
//! Reconstruction Engine: Reverse-query via Banach fixed-point iteration
//!
//! Menggunakan Teorema Titik Tetap Banach untuk melakukan iterasi mundur
//! dan merekonstruksi data asli dari transformasi kontraksi.

use crate::{SeedResult, SeedError};
use nalgebra::DVector;
use crate::core::StructuralFractalCore;

/// Konfigurasi untuk fixed-point iteration
#[derive(Clone, Debug)]
pub struct ReconstructionConfig {
    /// Epsilon precision untuk convergence
    pub epsilon: f64,
    /// Maximum iterations allowed
    pub max_iterations: usize,
}

impl Default for ReconstructionConfig {
    fn default() -> Self {
        ReconstructionConfig {
            epsilon: 1e-9,
            max_iterations: 1000,
        }
    }
}

/// Reconstruction Engine menggunakan Banach fixed-point theorem
pub struct ReconstructionEngine;

impl ReconstructionEngine {
    /// Rekonstruksi data asli dari compressed vector menggunakan Banach fixed-point iteration
    ///
    /// Algoritma:
    /// 1. Mulai dengan initial guess (compressed vector atau random)
    /// 2. Apply inversi transformasi: x_{n+1} = M^-1 * (x_n - B)
    /// 3. Ulangi hingga konvergen: ||x_{n+1} - x_n|| < epsilon
    /// 4. Return fixed point x*
    ///
    /// # Argumen
    /// - `core`: StructuralFractalCore yang berisi transformasi
    /// - `compressed`: Vektor terkompresi (starting point untuk iterasi)
    /// - `config`: Konfigurasi untuk iterasi
    ///
    /// # Return
    /// Vektor asli yang telah direkonstruksi
    pub fn reconstruct(
        core: &StructuralFractalCore,
        compressed: &DVector<f64>,
        config: ReconstructionConfig,
    ) -> SeedResult<DVector<f64>> {
        if core.transformation_count() == 0 {
            return Err(SeedError::InvalidConfiguration {
                reason: "No transformations in core".to_string(),
            });
        }

        // Verifikasi dimensi
        let transform = core.get_transformation(0).ok_or_else(|| {
            SeedError::InvalidConfiguration {
                reason: "Transformation 0 not found".to_string(),
            }
        })?;

        if compressed.len() != transform.bias.len() {
            return Err(SeedError::DimensionMismatch {
                expected: transform.bias.len(),
                actual: compressed.len(),
            });
        }

        // Mulai dengan initial guess = compressed vector
        let mut x_current = compressed.clone();
        let mut error = f64::INFINITY;

        for _ in 0..config.max_iterations {
            // Apply backward transformation untuk setiap transformation
            // x_new = M^{-1} * (x_current - B)
            let mut x_next = x_current.clone();

            // Iterate melalui semua transformasi dari belakang (reverse order)
            for i in (0..core.transformation_count()).rev() {
                let transform = core.get_transformation(i).ok_or_else(|| {
                    SeedError::InvalidConfiguration {
                        reason: format!("Transformation {} not found", i),
                    }
                })?;

                // Invert transformation
                let m_inv = transform
                    .matrix
                    .clone()
                    .try_inverse()
                    .ok_or_else(|| SeedError::SingularMatrix {
                        determinant: transform.matrix.determinant(),
                    })?;

                // x = M^-1 * (x - B)
                x_next = &m_inv * (&x_next - &transform.bias);
            }

            // Hitung error convergence
            error = (&x_next - &x_current).norm();

            if error < config.epsilon {
                return Ok(x_next);
            }

            x_current = x_next;
        }

        // Jika tidak konvergen setelah max_iterations
        Err(SeedError::FixedPointNoConvergence {
            max_iterations: config.max_iterations,
            epsilon: config.epsilon,
            final_error: error,
        })
    }

    /// Verifikasi reconstruction accuracy dengan forward transformation
    pub fn verify_reconstruction(
        _core: &StructuralFractalCore,
        original: &DVector<f64>,
        reconstructed: &DVector<f64>,
    ) -> SeedResult<f64> {
        if original.len() != reconstructed.len() {
            return Err(SeedError::DimensionMismatch {
                expected: original.len(),
                actual: reconstructed.len(),
            });
        }

        let diff = (original - reconstructed).norm();
        Ok(diff)
    }

    /// Iterative reconstruction dengan batch processing
    ///
    /// Memproses multiple compressed vectors sekaligus untuk meningkatkan
    /// cache locality dan throughput.
    pub fn reconstruct_batch(
        core: &StructuralFractalCore,
        compressed_batch: &[DVector<f64>],
        config: ReconstructionConfig,
    ) -> SeedResult<Vec<DVector<f64>>> {
        compressed_batch
            .iter()
            .map(|compressed| Self::reconstruct(core, compressed, config.clone()))
            .collect()
    }

    /// Advanced: Reconstruction dengan adaptive epsilon untuk trade-off kecepatan vs akurasi
    pub fn reconstruct_adaptive(
        core: &StructuralFractalCore,
        compressed: &DVector<f64>,
        target_error: f64,
        max_iterations: usize,
    ) -> SeedResult<(DVector<f64>, usize)> {
        if core.transformation_count() == 0 {
            return Err(SeedError::InvalidConfiguration {
                reason: "No transformations in core".to_string(),
            });
        }

        let mut x_current = compressed.clone();

        for iter in 0..max_iterations {
            let mut x_next = x_current.clone();

            // Reverse iteration through transformations
            for i in (0..core.transformation_count()).rev() {
                let transform = core.get_transformation(i).ok_or_else(|| {
                    SeedError::InvalidConfiguration {
                        reason: format!("Transformation {} not found", i),
                    }
                })?;

                let m_inv = transform
                    .matrix
                    .clone()
                    .try_inverse()
                    .ok_or_else(|| SeedError::SingularMatrix {
                        determinant: transform.matrix.determinant(),
                    })?;

                x_next = &m_inv * (&x_next - &transform.bias);
            }

            let error = (&x_next - &x_current).norm();

            if error < target_error {
                return Ok((x_next, iter + 1));
            }

            x_current = x_next;
        }

        Err(SeedError::FixedPointNoConvergence {
            max_iterations,
            epsilon: target_error,
            final_error: (&x_current).norm(),
        })
    }

    /// Multi-step reconstruction dengan progressive epsilon tightening
    ///
    /// Berguna untuk data yang sangat compressed dan memerlukan high precision reconstruction.
    pub fn reconstruct_progressive(
        core: &StructuralFractalCore,
        compressed: &DVector<f64>,
        target_precision: f64,
    ) -> SeedResult<DVector<f64>> {
        let mut current_epsilon = 1e-3; // Start coarse
        let mut result = compressed.clone();

        while current_epsilon > target_precision {
            let config = ReconstructionConfig {
                epsilon: current_epsilon,
                max_iterations: 5000,
            };

            result = Self::reconstruct(core, &result, config)?;
            current_epsilon /= 10.0;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::AffineTransformation;
    use nalgebra::DMatrix;

    #[test]
    fn test_reconstruction_config_default() {
        let config = ReconstructionConfig::default();
        assert!(config.epsilon > 0.0);
        assert!(config.max_iterations > 0);
    }

    #[test]
    fn test_simple_reconstruction() -> SeedResult<()> {
        // Setup: 1D system dengan transformasi x' = 0.5*x + 1.0
        let mut core = StructuralFractalCore::new(1)?;

        let matrix = DMatrix::from_row_slice(1, 1, &[0.5]);
        let bias = DVector::from_vec(vec![1.0]);
        let transform = AffineTransformation::new(matrix, bias)?;

        core.add_transformation(transform)?;

        // Original value
        let original = DVector::from_vec(vec![5.0]);

        // Apply forward transformation
        let forward = core
            .get_transformation(0)
            .unwrap()
            .apply(&original)?;

        // Reconstruct backward
        let config = ReconstructionConfig {
            epsilon: 1e-9,
            max_iterations: 100,
        };
        let reconstructed = ReconstructionEngine::reconstruct(&core, &forward, config)?;

        // Verify
        let error = (original - &reconstructed).norm();
        assert!(error < 1e-8);

        Ok(())
    }

    #[test]
    fn test_reconstruction_2d() -> SeedResult<()> {
        // 2D system dengan scaling transformation
        let mut core = StructuralFractalCore::new(2)?;

        let matrix = DMatrix::identity(2, 2) * 0.5;
        let bias = DVector::from_vec(vec![1.0, 2.0]);
        let transform = AffineTransformation::new(matrix, bias)?;

        core.add_transformation(transform)?;

        let original = DVector::from_vec(vec![4.0, 6.0]);
        let forward = core
            .get_transformation(0)
            .unwrap()
            .apply(&original)?;

        let config = ReconstructionConfig {
            epsilon: 1e-9,
            max_iterations: 100,
        };
        let reconstructed = ReconstructionEngine::reconstruct(&core, &forward, config)?;

        let error = (original - &reconstructed).norm();
        assert!(error < 1e-8);

        Ok(())
    }

    #[test]
    fn test_reconstruction_no_transformations() -> SeedResult<()> {
        let core = StructuralFractalCore::new(2)?;
        let compressed = DVector::from_vec(vec![1.0, 2.0]);

        let config = ReconstructionConfig::default();
        let result = ReconstructionEngine::reconstruct(&core, &compressed, config);

        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_reconstruction_dimension_mismatch() -> SeedResult<()> {
        let mut core = StructuralFractalCore::new(2)?;

        let matrix = DMatrix::identity(2, 2) * 0.5;
        let bias = DVector::from_vec(vec![1.0, 2.0]);
        let transform = AffineTransformation::new(matrix, bias)?;

        core.add_transformation(transform)?;

        let wrong_dim = DVector::from_vec(vec![1.0]); // Should be 2D

        let config = ReconstructionConfig::default();
        let result = ReconstructionEngine::reconstruct(&core, &wrong_dim, config);

        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_verify_reconstruction() -> SeedResult<()> {
        let original = DVector::from_vec(vec![1.0, 2.0]);
        let reconstructed = DVector::from_vec(vec![1.01, 2.01]);

        let diff = ReconstructionEngine::verify_reconstruction(&StructuralFractalCore::new(2)?, &original, &reconstructed)?;

        assert!(diff < 0.05);

        Ok(())
    }

    #[test]
    fn test_reconstruct_adaptive() -> SeedResult<()> {
        let mut core = StructuralFractalCore::new(1)?;

        let matrix = DMatrix::from_row_slice(1, 1, &[0.5]);
        let bias = DVector::from_vec(vec![1.0]);
        let transform = AffineTransformation::new(matrix, bias)?;

        core.add_transformation(transform)?;

        let original = DVector::from_vec(vec![5.0]);
        let forward = core
            .get_transformation(0)
            .unwrap()
            .apply(&original)?;

        let (reconstructed, iterations) =
            ReconstructionEngine::reconstruct_adaptive(&core, &forward, 1e-8, 100)?;

        let error = (original - &reconstructed).norm();
        assert!(error < 1e-8);
        assert!(iterations > 0 && iterations < 100);

        Ok(())
    }

    #[test]
    fn test_reconstruct_batch() -> SeedResult<()> {
        let mut core = StructuralFractalCore::new(1)?;

        let matrix = DMatrix::from_row_slice(1, 1, &[0.5]);
        let bias = DVector::from_vec(vec![1.0]);
        let transform = AffineTransformation::new(matrix, bias)?;

        core.add_transformation(transform)?;

        let compressed_batch = vec![
            DVector::from_vec(vec![3.0]),
            DVector::from_vec(vec![4.0]),
            DVector::from_vec(vec![5.0]),
        ];

        let config = ReconstructionConfig::default();
        let results = ReconstructionEngine::reconstruct_batch(&core, &compressed_batch, config)?;

        assert_eq!(results.len(), 3);

        Ok(())
    }
}

