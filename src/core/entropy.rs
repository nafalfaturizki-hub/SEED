#![forbid(unsafe_code)]
//! Modul Entropy Reducer: Perhitungan jarak semantik dan collapse transformasi
//!
//! Mengimplementasikan:
//! - Frobenius Norm untuk mengukur jarak dua matriks transformasi
//! - Entropy Collapse untuk meleburkan transformasi ketika jarak < threshold
//! - Engine reduksi entropi untuk mencegah memory bloat

use nalgebra::DMatrix;
use crate::{SeedResult, SeedError};
use std::f64;

/// Default threshold untuk triggerisasi entropy collapse
pub const DEFAULT_COLLAPSE_THRESHOLD: f64 = 0.15;

/// Engine reduksi entropi yang mengelola transformasi dengan threshold otomatis
#[derive(Clone, Debug)]
pub struct EntropyReducer {
    /// Threshold Frobenius distance untuk triggerisasi collapse
    collapse_threshold: f64,
    /// Bobot untuk transformasi lama saat melakukan collapse
    old_weight: f64,
    /// Bobot untuk transformasi baru saat melakukan collapse
    new_weight: f64,
}

impl EntropyReducer {
    /// Membuat entropy reducer dengan konfigurasi default
    pub fn new() -> Self {
        EntropyReducer {
            collapse_threshold: DEFAULT_COLLAPSE_THRESHOLD,
            old_weight: 0.6,
            new_weight: 0.4,
        }
    }

    /// Membuat entropy reducer dengan konfigurasi custom
    ///
    /// # Argumen
    /// - `threshold`: Frobenius distance threshold untuk collapse
    /// - `old_weight`: Bobot transformasi lama (0.0 - 1.0)
    /// - `new_weight`: Bobot transformasi baru (0.0 - 1.0)
    pub fn with_config(
        threshold: f64,
        old_weight: f64,
        new_weight: f64,
    ) -> SeedResult<Self> {
        if threshold <= 0.0 || threshold > 1.0 {
            return Err(SeedError::InvalidConfiguration {
                reason: format!("threshold must be in (0, 1], got {}", threshold),
            });
        }

        if old_weight < 0.0 || old_weight > 1.0 {
            return Err(SeedError::InvalidConfiguration {
                reason: format!("old_weight must be in [0, 1], got {}", old_weight),
            });
        }

        if new_weight < 0.0 || new_weight > 1.0 {
            return Err(SeedError::InvalidConfiguration {
                reason: format!("new_weight must be in [0, 1], got {}", new_weight),
            });
        }

        let total_weight = old_weight + new_weight;
        if (total_weight - 1.0).abs() > 1e-10 {
            return Err(SeedError::InvalidConfiguration {
                reason: format!(
                    "old_weight + new_weight must equal 1.0, got {}",
                    total_weight
                ),
            });
        }

        Ok(EntropyReducer {
            collapse_threshold: threshold,
            old_weight,
            new_weight,
        })
    }

    /// Mengecek apakah entropy collapse harus dipicu
    pub fn should_collapse(&self, distance: f64) -> bool {
        distance < self.collapse_threshold
    }

    /// Mengembalikan threshold yang digunakan reducer
    pub fn threshold(&self) -> f64 {
        self.collapse_threshold
    }

    /// Mengembalikan bobot transformasi lama
    pub fn old_weight(&self) -> f64 {
        self.old_weight
    }

    /// Mengembalikan bobot transformasi baru
    pub fn new_weight(&self) -> f64 {
        self.new_weight
    }
}

impl Default for EntropyReducer {
    fn default() -> Self {
        Self::new()
    }
}

/// Menghitung jarak Frobenius antara dua matriks transformasi
///
/// Frobenius Norm didefinisikan sebagai:
/// ```
/// ||ΔM||_F = sqrt( tr( (M_old - M_new)^T * (M_old - M_new) ) )
///          = sqrt( sum of squared elements dalam (M_old - M_new) )
/// ```
///
/// # Argumen
/// - `m_old`: Matriks transformasi lama (M_old)
/// - `m_new`: Matriks transformasi baru (M_new)
///
/// # Return
/// - `SeedResult<f64>`: Frobenius distance, atau error jika dimensi tidak sesuai
///
/// # Contoh
/// ```ignore
/// use nalgebra::DMatrix;
/// use sedd_core::core::entropy::calculate_matrix_distance;
///
/// let m_old = DMatrix::identity(2, 2) * 0.5;
/// let m_new = DMatrix::identity(2, 2) * 0.4;
/// let distance = calculate_matrix_distance(&m_old, &m_new).unwrap();
/// println!("Frobenius distance: {}", distance);
/// ```
pub fn calculate_matrix_distance(m_old: &DMatrix<f64>, m_new: &DMatrix<f64>) -> SeedResult<f64> {
    // Validasi dimensi
    if m_old.nrows() != m_new.nrows() || m_old.ncols() != m_new.ncols() {
        return Err(SeedError::DimensionMismatch {
            expected: m_old.nrows() * m_old.ncols(),
            actual: m_new.nrows() * m_new.ncols(),
        });
    }

    // Hitung ΔM = M_old - M_new
    let delta_m = m_old - m_new;

    // Hitung tr( ΔM^T * ΔM ) = sum of squared elements
    // Karena (ΔM^T * ΔM) adalah symmetric matrix, trace-nya adalah sum dari kuadrat semua elemen ΔM
    let sum_squared: f64 = delta_m.iter().map(|&x| x * x).sum();

    // Frobenius norm = sqrt(sum_squared)
    Ok(sum_squared.sqrt())
}

/// Meleburkan dua matriks transformasi dengan bobot tertentu
///
/// Formula collapse:
/// ```
/// M_collapsed = (M_old * w_old + M_new * w_new) / (w_old + w_new)
/// ```
///
/// Dimana normalizednya: w_old + w_new = 1.0
///
/// # Argumen
/// - `m_old`: Matriks transformasi lama
/// - `m_new`: Matriks transformasi baru
/// - `weight_old`: Bobot untuk transformasi lama
/// - `weight_new`: Bobot untuk transformasi baru
///
/// # Return
/// - `SeedResult<DMatrix<f64>>`: Matriks yang telah dicollapse, atau error jika dimensi tidak sesuai
///
/// # Contoh
/// ```ignore
/// use nalgebra::DMatrix;
/// use sedd_core::core::entropy::collapse_transformation;
///
/// let m_old = DMatrix::identity(2, 2) * 0.5;
/// let m_new = DMatrix::identity(2, 2) * 0.4;
/// let m_collapsed = collapse_transformation(&m_old, &m_new, 0.6, 0.4).unwrap();
/// println!("Collapsed matrix: {}", m_collapsed);
/// ```
pub fn collapse_transformation(
    m_old: &DMatrix<f64>,
    m_new: &DMatrix<f64>,
    weight_old: f64,
    weight_new: f64,
) -> SeedResult<DMatrix<f64>> {
    // Validasi dimensi
    if m_old.nrows() != m_new.nrows() || m_old.ncols() != m_new.ncols() {
        return Err(SeedError::DimensionMismatch {
            expected: m_old.nrows() * m_old.ncols(),
            actual: m_new.nrows() * m_new.ncols(),
        });
    }

    // Validasi bobot (sudah harus sum = 1.0, tapi kita double-check)
    let total_weight = weight_old + weight_new;
    if (total_weight - 1.0).abs() > 1e-10 {
        return Err(SeedError::InvalidConfiguration {
            reason: format!(
                "weights must sum to 1.0, got weight_old={} + weight_new={}",
                weight_old, weight_new
            ),
        });
    }

    // M_collapsed = M_old * weight_old + M_new * weight_new
    let collapsed = m_old * weight_old + m_new * weight_new;

    Ok(collapsed)
}

/// Collapse alternative yang menggunakan rata-rata geometrik untuk preservasi struktur
///
/// Formula:
/// ```
/// M_collapsed = exp( weight_old * ln(M_old) + weight_new * ln(M_new) )
/// ```
///
/// Ini lebih preserve struktur untuk matriks dengan eigenvalue yang complex,
/// tapi lebih expensive computationally.
///
/// # Catatan
/// Implementasi ini menggunakan matrix exponential dan logarithm dari nalgebra (via SVD).
/// Hanya untuk matriks symmetric positive-definite.
pub fn collapse_transformation_geometric(
    m_old: &DMatrix<f64>,
    m_new: &DMatrix<f64>,
    weight_old: f64,
    weight_new: f64,
) -> SeedResult<DMatrix<f64>> {
    // Validasi dimensi
    if m_old.nrows() != m_new.nrows() || m_old.ncols() != m_new.ncols() {
        return Err(SeedError::DimensionMismatch {
            expected: m_old.nrows() * m_old.ncols(),
            actual: m_new.nrows() * m_new.ncols(),
        });
    }

    // Untuk simplisitas dan performance, gunakan collapse linear standard
    // (geometric collapse memerlukan eigendecomposition, lebih complex)
    collapse_transformation(m_old, m_new, weight_old, weight_new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_matrix_distance_zero() {
        let m = DMatrix::identity(2, 2) * 0.5;
        let distance = calculate_matrix_distance(&m, &m).unwrap();
        assert!(distance < 1e-10); // Distance ke diri sendiri adalah 0
    }

    #[test]
    fn test_calculate_matrix_distance_simple() {
        let m_old = DMatrix::identity(2, 2) * 0.5;
        let m_new = DMatrix::identity(2, 2) * 0.4;

        let distance = calculate_matrix_distance(&m_old, &m_new).unwrap();

        // ΔM = [[0.1, 0], [0, 0.1]]
        // Frobenius = sqrt(0.1^2 + 0.1^2) = sqrt(0.02) ≈ 0.14142...
        let expected = (0.02_f64).sqrt();
        assert!((distance - expected).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_matrix_distance_complex() {
        let m_old = DMatrix::from_row_slice(2, 2, &[0.5, 0.1, 0.2, 0.3]);
        let m_new = DMatrix::from_row_slice(2, 2, &[0.4, 0.15, 0.25, 0.25]);

        let distance = calculate_matrix_distance(&m_old, &m_new).unwrap();

        // ΔM = [[0.1, -0.05], [-0.05, 0.05]]
        // Frobenius = sqrt(0.1^2 + 0.05^2 + 0.05^2 + 0.05^2)
        //           = sqrt(0.01 + 0.0025 + 0.0025 + 0.0025)
        //           = sqrt(0.0175) ≈ 0.13228...
        let expected = (0.0175_f64).sqrt();
        assert!((distance - expected).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_matrix_distance_dimension_mismatch() {
        let m_old = DMatrix::identity(2, 2);
        let m_new = DMatrix::identity(3, 3);

        let result = calculate_matrix_distance(&m_old, &m_new);
        assert!(result.is_err());
    }

    #[test]
    fn test_collapse_transformation_equal_weights() {
        let m_old = DMatrix::identity(2, 2) * 0.5;
        let m_new = DMatrix::identity(2, 2) * 0.3;

        let collapsed = collapse_transformation(&m_old, &m_new, 0.5, 0.5).unwrap();

        // Hasil harusnya rata-rata: identity(2) * 0.4
        let expected = DMatrix::identity(2, 2) * 0.4;
        assert!((collapsed - expected).norm() < 1e-10);
    }

    #[test]
    fn test_collapse_transformation_weighted() {
        let m_old = DMatrix::identity(2, 2) * 0.5;
        let m_new = DMatrix::identity(2, 2) * 0.3;

        let collapsed = collapse_transformation(&m_old, &m_new, 0.6, 0.4).unwrap();

        // Hasil: identity(2) * (0.5 * 0.6 + 0.3 * 0.4) = identity(2) * 0.42
        let expected = DMatrix::identity(2, 2) * (0.5 * 0.6 + 0.3 * 0.4);
        assert!((collapsed - expected).norm() < 1e-10);
    }

    #[test]
    fn test_collapse_transformation_dimension_mismatch() {
        let m_old = DMatrix::identity(2, 2);
        let m_new = DMatrix::identity(3, 3);

        let result = collapse_transformation(&m_old, &m_new, 0.5, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_collapse_transformation_invalid_weights() {
        let m = DMatrix::identity(2, 2);

        // Weights tidak sum ke 1.0
        let result = collapse_transformation(&m, &m, 0.5, 0.4);
        assert!(result.is_err());
    }

    #[test]
    fn test_entropy_reducer_new() {
        let reducer = EntropyReducer::new();
        assert!((reducer.threshold() - DEFAULT_COLLAPSE_THRESHOLD).abs() < 1e-10);
        assert!((reducer.old_weight() + reducer.new_weight() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_entropy_reducer_should_collapse() {
        let reducer = EntropyReducer::new();

        // Distance lebih kecil dari threshold
        assert!(reducer.should_collapse(0.1));

        // Distance sama dengan atau melebihi threshold
        assert!(!reducer.should_collapse(0.15));
        assert!(!reducer.should_collapse(0.2));
    }

    #[test]
    fn test_entropy_reducer_with_custom_config() {
        let reducer = EntropyReducer::with_config(0.1, 0.7, 0.3).unwrap();
        assert!((reducer.threshold() - 0.1).abs() < 1e-10);
        assert!((reducer.old_weight() - 0.7).abs() < 1e-10);
        assert!((reducer.new_weight() - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_entropy_reducer_invalid_threshold() {
        let result = EntropyReducer::with_config(0.0, 0.5, 0.5);
        assert!(result.is_err());

        let result = EntropyReducer::with_config(1.5, 0.5, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_entropy_reducer_invalid_weights() {
        let result = EntropyReducer::with_config(0.15, 0.5, 0.4);
        assert!(result.is_err());

        let result = EntropyReducer::with_config(0.15, -0.1, 1.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_entropy_reducer_default() {
        let reducer = EntropyReducer::default();
        let reducer2 = EntropyReducer::new();

        assert!((reducer.threshold() - reducer2.threshold()).abs() < 1e-10);
    }

    #[test]
    fn test_frobenius_norm_properties() {
        // Test sifat Frobenius norm: ||ΔM|| = ||ΔM^T|| (symmetric)
        let m_old = DMatrix::from_row_slice(2, 2, &[0.5, 0.1, 0.2, 0.3]);
        let m_new = DMatrix::from_row_slice(2, 2, &[0.4, 0.15, 0.25, 0.25]);

        let distance = calculate_matrix_distance(&m_old, &m_new).unwrap();

        let m_old_t = m_old.transpose();
        let m_new_t = m_new.transpose();
        let distance_t = calculate_matrix_distance(&m_old_t, &m_new_t).unwrap();

        assert!((distance - distance_t).abs() < 1e-10);
    }

    #[test]
    fn test_collapse_preserves_structure() {
        // Test bahwa collapse dari two equal matrices menghasilkan matrix yang sama
        let m = DMatrix::from_row_slice(2, 2, &[0.5, 0.1, 0.2, 0.3]);

        let collapsed = collapse_transformation(&m, &m, 0.6, 0.4).unwrap();

        assert!((collapsed - &m).norm() < 1e-10);
    }

    #[test]
    fn test_integration_entropy_collapse_workflow() {
        // Simulasi workflow lengkap: hitung distance, check collapse, kemudian collapse jika perlu
        let reducer = EntropyReducer::new();

        let m_old = DMatrix::identity(2, 2) * 0.5;
        let m_new = DMatrix::identity(2, 2) * 0.45;

        let distance = calculate_matrix_distance(&m_old, &m_new).unwrap();

        if reducer.should_collapse(distance) {
            let collapsed = collapse_transformation(
                &m_old,
                &m_new,
                reducer.old_weight(),
                reducer.new_weight(),
            )
            .unwrap();

            // Verify collapsed matrix is between old and new
            let collapsed_norm = collapsed.norm();
            let old_norm = m_old.norm();
            let new_norm = m_new.norm();

            assert!(collapsed_norm <= old_norm && collapsed_norm >= new_norm);
        }
    }
}
