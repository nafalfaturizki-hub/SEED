#![forbid(unsafe_code)]
//! Modul Fractal Core: Transformasi affine dan kontraksi fraktal
//!
//! Menangani struktur data transformasi W(x) = M·x + B dan validasi properti kontraksi.

use nalgebra::{DMatrix, DVector};
use crate::SeedResult;

/// Transformasi affine W(x) = M·x + B dengan properti kontraksi dijamin
#[derive(Clone, Debug)]
pub struct AffineTransformation {
    /// Matriks transformasi M (harus menjadi kontraksi: ||M|| < 1.0)
    pub matrix: DMatrix<f64>,
    /// Vektor bias B
    pub bias: DVector<f64>,
    /// Menyimpan norm operasi ||M|| untuk cache akses cepat
    pub matrix_norm: f64,
}

impl AffineTransformation {
    /// Membuat transformasi affine baru dengan validasi kontraksi
    ///
    /// # Argumen
    /// - `matrix`: Matriks transformasi M (akan dinormalisasi jika perlu)
    /// - `bias`: Vektor bias B
    ///
    /// # Return
    /// `SeedResult<AffineTransformation>` - Error jika matrix tidak dapat dinormalisasi
    pub fn new(mut matrix: DMatrix<f64>, bias: DVector<f64>) -> SeedResult<Self> {
        use crate::SeedError;

        // Validasi dimensi
        let dim = matrix.nrows();
        if matrix.ncols() != dim {
            return Err(SeedError::DimensionMismatch {
                expected: dim,
                actual: matrix.ncols(),
            });
        }
        if bias.len() != dim {
            return Err(SeedError::DimensionMismatch {
                expected: dim,
                actual: bias.len(),
            });
        }

        // Hitung operator norm (spectral norm ≈ singular value terbesar)
        let mut matrix_norm = Self::compute_spectral_norm(&matrix);

        // Jika matrix_norm >= 1.0, lakukan normalisasi otomatis dengan sigmoid kompresi
        if matrix_norm >= 1.0 {
            matrix = Self::normalize_via_sigmoid(&matrix, matrix_norm);
            matrix_norm = Self::compute_spectral_norm(&matrix);

            // Validasi ulang setelah normalisasi
            if matrix_norm >= 1.0 {
                return Err(SeedError::MathConvergenceFailure {
                    singular_value: matrix_norm,
                    threshold: 1.0,
                });
            }
        }

        Ok(AffineTransformation {
            matrix,
            bias,
            matrix_norm,
        })
    }

    /// Menghitung spectral norm (singular value terbesar) matriks secara aproksimasi
    fn compute_spectral_norm(matrix: &DMatrix<f64>) -> f64 {
        // Implementasi power iteration untuk mengestimasi singular value terbesar
        let n = matrix.ncols();
        if n == 0 {
            return 0.0;
        }

        let mut v = DVector::from_element(n, 1.0 / (n as f64).sqrt());
        for _ in 0..10 {
            let av = matrix * &v;
            let norm_av = av.norm();
            if norm_av < 1e-15 {
                break;
            }
            v = &av / norm_av;
        }

        let av = matrix * &v;
        av.norm()
    }

    /// Normalisasi matriks via sigmoid kompresi untuk menjamin kontraksi
    fn normalize_via_sigmoid(matrix: &DMatrix<f64>, norm: f64) -> DMatrix<f64> {
        // Gunakan sigmoid untuk memetakan norm ke [0, 1)
        let sigmoid_factor = 2.0 / (1.0 + (-norm + 1.0).exp()) - 1.0; // ≈ 0.76 untuk norm=1.0
        let compression_factor = sigmoid_factor / norm;
        matrix * compression_factor
    }

    /// Terapkan transformasi W(x) = M·x + B ke vektor input
    pub fn apply(&self, x: &DVector<f64>) -> SeedResult<DVector<f64>> {
        use crate::SeedError;

        if x.len() != self.matrix.ncols() {
            return Err(SeedError::DimensionMismatch {
                expected: self.matrix.ncols(),
                actual: x.len(),
            });
        }

        Ok(&self.matrix * x + &self.bias)
    }

    /// Menghitung determinan matriks transformasi
    pub fn determinant(&self) -> f64 {
        self.matrix.determinant()
    }

    /// Mencoba menginversi transformasi: x = M^-1 * (y - B)
    pub fn try_invert(&self) -> SeedResult<AffineTransformation> {
        use crate::SeedError;

        let det = self.determinant();
        if det.abs() < 1e-10 {
            return Err(SeedError::SingularMatrix {
                determinant: det,
            });
        }

        let m_inv = self.matrix.clone().try_inverse().ok_or_else(|| {
            SeedError::SingularMatrix {
                determinant: det,
            }
        })?;

        let b_inv = -&m_inv * &self.bias;

        // Matriks invers dari kontraksi juga harus valid
        AffineTransformation::new(m_inv, b_inv)
    }
}

/// Core engine fraktal struktural SEED
#[derive(Clone, Debug)]
pub struct StructuralFractalCore {
    /// Transformasi-transformasi affine yang aktif (versi saat ini)
    transformations: Vec<AffineTransformation>,
    /// Dimensi vektor intrinsik (fixed setelah inisialisasi)
    dimension: usize,
}

impl StructuralFractalCore {
    /// Membuat core fraktal baru dengan dimensi spesifik
    pub fn new(dimension: usize) -> SeedResult<Self> {
        if dimension == 0 {
            return Err(crate::SeedError::InvalidConfiguration {
                reason: "dimension must be > 0".to_string(),
            });
        }

        Ok(StructuralFractalCore {
            transformations: Vec::new(),
            dimension,
        })
    }

    /// Menambah transformasi affine baru ke dalam kumpulan
    pub fn add_transformation(&mut self, transform: AffineTransformation) -> SeedResult<()> {
        if transform.matrix.nrows() != self.dimension {
            return Err(crate::SeedError::DimensionMismatch {
                expected: self.dimension,
                actual: transform.matrix.nrows(),
            });
        }
        self.transformations.push(transform);
        Ok(())
    }

    /// Mengambil referensi ke transformasi pada indeks tertentu
    pub fn get_transformation(&self, idx: usize) -> Option<&AffineTransformation> {
        self.transformations.get(idx)
    }

    /// Mengembalikan jumlah transformasi saat ini
    pub fn transformation_count(&self) -> usize {
        self.transformations.len()
    }

    /// Mengembalikan referensi mutable ke vektor transformasi untuk operasi internal
    pub fn transformations_mut(&mut self) -> &mut Vec<AffineTransformation> {
        &mut self.transformations
    }

    /// Mengembalikan referensi ke vektor transformasi
    pub fn transformations(&self) -> &[AffineTransformation] {
        &self.transformations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affine_transformation_creation() {
        let matrix = DMatrix::identity(2, 2) * 0.5; // Kontraksi valid
        let bias = DVector::zeros(2);

        let transform = AffineTransformation::new(matrix, bias).unwrap();
        assert!(transform.matrix_norm < 1.0);
    }

    #[test]
    fn test_affine_transformation_auto_normalize() {
        let matrix = DMatrix::identity(2, 2) * 1.5; // Bukan kontraksi, akan dinormalisasi
        let bias = DVector::zeros(2);

        let transform = AffineTransformation::new(matrix, bias).unwrap();
        assert!(transform.matrix_norm < 1.0); // Harus dinormalisasi otomatis
    }

    #[test]
    fn test_affine_apply() {
        let matrix = DMatrix::identity(2, 2) * 0.5;
        let bias = DVector::from_vec(vec![1.0, 2.0]);
        let transform = AffineTransformation::new(matrix, bias).unwrap();

        let x = DVector::from_vec(vec![1.0, 1.0]);
        let result = transform.apply(&x).unwrap();

        let expected = DVector::from_vec(vec![1.5, 2.5]);
        assert!((result - expected).norm() < 1e-10);
    }

    #[test]
    fn test_affine_dimension_mismatch() {
        let matrix = DMatrix::identity(2, 2) * 0.5;
        let bias = DVector::zeros(3); // Mismatch!

        assert!(AffineTransformation::new(matrix, bias).is_err());
    }

    #[test]
    fn test_structural_fractal_core() {
        let mut core = StructuralFractalCore::new(2).unwrap();
        assert_eq!(core.transformation_count(), 0);

        let matrix = DMatrix::identity(2, 2) * 0.5;
        let bias = DVector::zeros(2);
        let transform = AffineTransformation::new(matrix, bias).unwrap();

        core.add_transformation(transform).unwrap();
        assert_eq!(core.transformation_count(), 1);
    }
}
