#![allow(unsafe_code)]
//! Matrix Storage: Zero-copy mmap I/O dengan serialisasi binary dan checksum

use crate::{SeedResult, SeedError};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use memmap2::Mmap;
use serde::{Serialize, Deserialize};
use crc::Crc;

/// Magic bytes untuk file signature
const SEED_MAGIC: &[u8] = b"SEED";

/// Versi format file
#[allow(dead_code)]
const FILE_VERSION: u32 = 1;

/// CRC-32 untuk checksum validation
static CRC_32: once_cell::sync::Lazy<Crc<u32>> = 
    once_cell::sync::Lazy::new(|| Crc::<u32>::new(&crc::CRC_32_ISCSI));

/// Serializable representation dari transformasi affine untuk persistent storage
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializedTransformation {
    /// Matriks M sebagai flat vector
    pub matrix_data: Vec<f64>,
    pub matrix_rows: usize,
    pub matrix_cols: usize,
    /// Vektor bias B
    pub bias: Vec<f64>,
    /// Norm dari matriks
    pub matrix_norm: f64,
}

/// Header untuk file SEED transformasi
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct SeedFileHeader {
    magic: [u8; 4],
    version: u32,
    num_transformations: u32,
    timestamp: u64,
}

/// Matrix Store Engine untuk persistent storage dan retrieval transformasi
pub struct MatrixStore {
    path: PathBuf,
    file: Option<File>,
    mmap: Option<Mmap>,
    cached_transformations: Vec<SerializedTransformation>,
    dirty: bool,
}

impl MatrixStore {
    /// Membuat atau membuka matrix store di path spesifik
    pub fn open<P: AsRef<Path>>(path: P) -> SeedResult<Self> {
        let path = path.as_ref().to_path_buf();

        let file = if path.exists() {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Failed to open store: {}", e),
                })?
        } else {
            File::create(&path)
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Failed to create store: {}", e),
                })?
        };

        let mut store = MatrixStore {
            path,
            file: Some(file),
            mmap: None,
            cached_transformations: Vec::new(),
            dirty: false,
        };

        // Load existing transformations jika file tidak kosong
        if store.file.as_ref().unwrap().metadata()
            .map(|m| m.len() > 0)
            .unwrap_or(false)
        {
            store.load_all()?;
        }

        Ok(store)
    }

    /// Store satu transformasi ke dalam memory cache
    pub fn store_transformation(&mut self, transform: SerializedTransformation) -> SeedResult<()> {
        self.cached_transformations.push(transform);
        self.dirty = true;
        Ok(())
    }

    /// Retrieve transformasi by index
    pub fn get_transformation(&self, idx: usize) -> Option<&SerializedTransformation> {
        self.cached_transformations.get(idx)
    }

    /// Jumlah transformasi tersimpan
    pub fn count(&self) -> usize {
        self.cached_transformations.len()
    }

    /// Load semua transformasi dari disk ke memory
    pub fn load_all(&mut self) -> SeedResult<()> {
        if let Some(ref mut file) = self.file {
            file.seek(SeekFrom::Start(0))
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Seek failed: {}", e),
                })?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Read failed: {}", e),
                })?;

            if buffer.len() < 16 {
                return Ok(()); // File kosong
            }

            // Verify magic
            if &buffer[0..4] != SEED_MAGIC {
                return Err(SeedError::DataCorrupted {
                    expected: u64::from_le_bytes([SEED_MAGIC[0], SEED_MAGIC[1], SEED_MAGIC[2], SEED_MAGIC[3], 0, 0, 0, 0]),
                    actual: u64::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3], 0, 0, 0, 0]),
                });
            }

            // Verify checksum (last 4 bytes)
            let checksum_stored = u32::from_le_bytes([
                buffer[buffer.len()-4], buffer[buffer.len()-3], buffer[buffer.len()-2], buffer[buffer.len()-1],
            ]);

            let checksum_computed = CRC_32.checksum(&buffer[..buffer.len()-4]);

            if checksum_stored != checksum_computed {
                return Err(SeedError::DataCorrupted {
                    expected: checksum_stored as u64,
                    actual: checksum_computed as u64,
                });
            }

            // Deserialize payload (tanpa magic dan checksum)
            let payload = &buffer[4..buffer.len()-8];
            self.cached_transformations = bincode::deserialize(payload)
                .map_err(|e| SeedError::DeserializationError {
                    reason: format!("Bincode error: {}", e),
                })?;

            self.dirty = false;
        }

        Ok(())
    }

    /// Flush semua transformasi dari memory ke disk dengan checksum
    pub fn flush(&mut self) -> SeedResult<()> {
        if !self.dirty {
            return Ok(());
        }

        if let Some(ref mut file) = self.file {
            // Serialize transformasi
            let payload = bincode::serialize(&self.cached_transformations)
                .map_err(|e| SeedError::DeserializationError {
                    reason: format!("Serialization error: {}", e),
                })?;

            // Buat buffer dengan magic, payload, dan checksum
            let mut buffer = Vec::new();
            buffer.extend_from_slice(SEED_MAGIC);
            buffer.extend_from_slice(&payload);

            // Hitung checksum dari magic + payload
            let checksum = CRC_32.checksum(&buffer);
            buffer.extend_from_slice(&checksum.to_le_bytes());

            // Write ke file
            file.seek(SeekFrom::Start(0))
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Seek failed: {}", e),
                })?;

            file.write_all(&buffer)
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Write failed: {}", e),
                })?;

            file.set_len(buffer.len() as u64)
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Truncate failed: {}", e),
                })?;

            file.sync_all()
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Sync failed: {}", e),
                })?;

            self.dirty = false;
        }

        Ok(())
    }

    /// Mmap buffer untuk zero-copy read (untuk file yang sudah di-flush)
    pub fn mmap_readonly(&mut self) -> SeedResult<()> {
        if let Some(ref file) = self.file {
            // SAFETY: Mmap::map is safe here since:
            // 1. File is opened with proper permissions
            // 2. We maintain exclusive mutable access to the mmap field
            // 3. File content is not modified while mmap is active
            unsafe {
                self.mmap = Some(Mmap::map(file)
                    .map_err(|e| SeedError::StorageIOError {
                        reason: format!("Mmap failed: {}", e),
                    })?);
            }
        }
        Ok(())
    }

    /// Akses mmap buffer (read-only)
    pub fn mmap_buffer(&self) -> Option<&Mmap> {
        self.mmap.as_ref()
    }

    /// Clear semua transformasi dari memory (keep on disk)
    pub fn clear_cache(&mut self) {
        self.cached_transformations.clear();
        self.mmap = None;
    }

    /// Get path dari store
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Builder untuk MatrixStore dengan fluent API
pub struct MatrixStoreBuilder {
    path: PathBuf,
    create_if_missing: bool,
}

impl MatrixStoreBuilder {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        MatrixStoreBuilder {
            path: path.as_ref().to_path_buf(),
            create_if_missing: true,
        }
    }

    pub fn create_if_missing(mut self, create: bool) -> Self {
        self.create_if_missing = create;
        self
    }

    pub fn build(self) -> SeedResult<MatrixStore> {
        if !self.create_if_missing && !self.path.exists() {
            return Err(SeedError::StorageIOError {
                reason: format!("Store not found at {}", self.path.display()),
            });
        }
        MatrixStore::open(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_matrix_store_create() -> SeedResult<()> {
        let temp = NamedTempFile::new()
            .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;
        let store = MatrixStore::open(temp.path())?;
        assert_eq!(store.count(), 0);
        Ok(())
    }

    #[test]
    fn test_matrix_store_store_and_retrieve() -> SeedResult<()> {
        let temp = NamedTempFile::new()
            .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;
        let mut store = MatrixStore::open(temp.path())?;

        let transform = SerializedTransformation {
            matrix_data: vec![0.5, 0.0, 0.0, 0.5],
            matrix_rows: 2,
            matrix_cols: 2,
            bias: vec![1.0, 2.0],
            matrix_norm: 0.5,
        };

        store.store_transformation(transform.clone())?;
        assert_eq!(store.count(), 1);

        let retrieved = store.get_transformation(0).unwrap();
        assert_eq!(retrieved.matrix_norm, 0.5);

        Ok(())
    }

    #[test]
    fn test_matrix_store_persistence() -> SeedResult<()> {
        let temp = NamedTempFile::new()
            .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;
        let temp_path = temp.path().to_path_buf();

        {
            let mut store = MatrixStore::open(&temp_path)?;

            let transform = SerializedTransformation {
                matrix_data: vec![0.5, 0.1, 0.2, 0.3],
                matrix_rows: 2,
                matrix_cols: 2,
                bias: vec![1.0, 2.0],
                matrix_norm: 0.5,
            };

            store.store_transformation(transform)?;
            store.flush()?;
        }

        // Reopen dan verify
        let mut store = MatrixStore::open(&temp_path)?;
        store.load_all()?;
        assert_eq!(store.count(), 1);

        let retrieved = store.get_transformation(0).unwrap();
        assert_eq!(retrieved.matrix_norm, 0.5);
        assert_eq!(retrieved.bias, vec![1.0, 2.0]);

        Ok(())
    }

    #[test]
    fn test_matrix_store_checksum_validation() -> SeedResult<()> {
        let temp = NamedTempFile::new()
            .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;
        let temp_path = temp.path().to_path_buf();

        {
            let mut store = MatrixStore::open(&temp_path)?;
            let transform = SerializedTransformation {
                matrix_data: vec![0.5],
                matrix_rows: 1,
                matrix_cols: 1,
                bias: vec![0.0],
                matrix_norm: 0.5,
            };
            store.store_transformation(transform)?;
            store.flush()?;
        }

        // Corrupt last 4 bytes (in checksum)
        {
            let mut file = OpenOptions::new()
                .write(true)
                .open(&temp_path)
                .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;

            let metadata = file.metadata()
                .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;
            let _size = metadata.len() as usize;

            file.seek(SeekFrom::End(-4))
                .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;

            file.write_all(&[0xFF, 0xFF, 0xFF, 0xFF])
                .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;
        }

        // Try to load - should fail checksum
        let mut store = MatrixStore::open(&temp_path)?;
        let result = store.load_all();
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_matrix_store_builder() -> SeedResult<()> {
        let temp = NamedTempFile::new()
            .map_err(|e| SeedError::StorageIOError { reason: e.to_string() })?;

        let store = MatrixStoreBuilder::new(temp.path())
            .create_if_missing(true)
            .build()?;

        assert_eq!(store.count(), 0);
        Ok(())
    }
}

