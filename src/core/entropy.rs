use nalgebra::{DMatrix, DVector};
use crate::core::fractal::AffineTransformation;

/// Engine untuk menghitung penyusutan informasi dan entropi semantik
pub struct EntropyReducer;

impl EntropyReducer {
    /// Menghitung jarak Frobenius antara dua matriks untuk melihat kemiripan pola data
    pub fn calculate_matrix_distance(m1: &DMatrix<f64>, m2: &DMatrix<f64>) -> f64 {
        let diff = m1 - m2;
        diff.frobenius_norm()
    }

    /// Menggabungkan dua transformasi menjadi satu rumus tunggal (Entropy Collapse)
    /// Ini adalah kunci mengapa ukuran database bisa menyusut mendekati konstan.
    pub fn collapse(t1: &AffineTransformation, t2: &AffineTransformation) -> AffineTransformation {
        // Rata-rata bobot matriks koefisien (0.5 melambangkan keseimbangan entropi)
        let collapsed_matrix = (&t1.matrix + &t2.matrix) * 0.5;
        
        // Rata-rata vektor pergeseran semantik
        let collapsed_translation = (&t1.translation + &t2.translation) * 0.5;

        AffineTransformation {
            matrix: collapsed_matrix,
            translation: collapsed_translation,
        }
    }
}
