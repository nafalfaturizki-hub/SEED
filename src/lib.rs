#![warn(unsafe_code)]
//! SEED: Structural Entropy Dissolution Database Engine
//!
//! Non-Von Neumann database engine operating in compressed fractal mathematical space.
//! Dissolution, entropy collapse, and lossless reconstruction via Banach fixed-point theorem.

pub mod core;
pub mod storage;
pub mod engine;

#[cfg(test)]
mod integration_test;

use thiserror::Error;

/// Komprehensif error enum untuk seluruh subsistem SEED
#[derive(Error, Debug)]
pub enum SeedError {
    /// Transformasi affine gagal konvergen karena matriks kontraksi condition terlanggar
    #[error("Math convergence failure: matrix singular value >= 1.0, contraction property violated")]
    MathConvergenceFailure {
        singular_value: f64,
        threshold: f64,
    },

    /// Determinan matriks transformasi nol atau singular (tidak invertibel)
    #[error("Determinant is zero or near-singular (det={determinant}), cannot invert transformation")]
    SingularMatrix {
        determinant: f64,
    },

    /// Entropi telah mencapai saturasi, tidak dapat menambah transformasi baru
    #[error("Entropy saturation reached: transformation array at capacity limit, collapse required")]
    EntropySaturation {
        current_capacity: usize,
        max_capacity: usize,
    },

    /// Iterasi fixed-point tidak konvergen dalam jumlah iterasi maksimal
    #[error("Fixed-point iteration failed to converge within {max_iterations} iterations, epsilon={epsilon}")]
    FixedPointNoConvergence {
        max_iterations: usize,
        epsilon: f64,
        final_error: f64,
    },

    /// Kesalahan I/O saat membaca/menulis data ke storage (disk, mmap)
    #[error("Storage I/O error: {reason}")]
    StorageIOError {
        reason: String,
    },

    /// Data terdeteksi korup atau checksum tidak sesuai
    #[error("Data corruption detected: expected checksum {expected}, got {actual}")]
    DataCorrupted {
        expected: u64,
        actual: u64,
    },

    /// Deserialisasi biner gagal (bincode error)
    #[error("Binary deserialization failed: {reason}")]
    DeserializationError {
        reason: String,
    },

    /// Vektor input memiliki dimensi yang tidak sesuai dengan transformasi saat ini
    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        expected: usize,
        actual: usize,
    },

    /// Operasi matrix algebra gagal (rank insufficient, dll)
    #[error("Matrix algebra failure: {reason}")]
    MatrixAlgebraError {
        reason: String,
    },

    /// Threshold atau parameter konfigurasi tidak valid
    #[error("Invalid configuration: {reason}")]
    InvalidConfiguration {
        reason: String,
    },

    /// Skema normalisasi vektor input tidak dikenali
    #[error("Unknown vector schema: {schema}")]
    UnknownVectorSchema {
        schema: String,
    },

    /// Iterasi atau proses generic mengalami overflow/underflow numerik
    #[error("Numeric overflow or underflow: {reason}")]
    NumericInstability {
        reason: String,
    },
}

/// Type alias untuk Result dengan SeedError sebagai error default
pub type SeedResult<T> = Result<T, SeedError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_error_display() {
        let err = SeedError::MathConvergenceFailure {
            singular_value: 1.05,
            threshold: 1.0,
        };
        assert!(err.to_string().contains("convergence failure"));

        let err = SeedError::EntropySaturation {
            current_capacity: 100,
            max_capacity: 100,
        };
        assert!(err.to_string().contains("saturation"));
    }

    #[test]
    fn test_seed_result_type() {
        let result: SeedResult<i32> = Err(SeedError::InvalidConfiguration {
            reason: "test".to_string(),
        });
        assert!(result.is_err());
    }
}
