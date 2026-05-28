#![forbid(unsafe_code)]
//! Data Ingestion Engine: Parser normalisasi vektor dari berbagai format input

use crate::{SeedResult, SeedError};
use nalgebra::DVector;
use serde_json::Value;
use std::io::Read;

/// Skema input yang didukung
#[derive(Clone, Debug, PartialEq)]
pub enum InputSchema {
    /// Flat array of numbers
    FlatArray,
    /// Nested array (2D matrix)
    NestedArray,
    /// JSON object dengan field tertentu
    JsonObject { fields: Vec<String> },
    /// CSV dengan headers
    Csv { has_header: bool },
    /// Raw binary float64 array
    BinaryF64,
}

/// Result dari parsing yang berisi metadata tentang data yang di-parse
#[derive(Clone, Debug)]
pub struct ParsedData {
    /// Vektor normalisasi (flattened)
    pub vector: DVector<f64>,
    /// Shape asli (untuk reconstruction)
    pub shape: (usize, usize),
    /// Min/Max untuk denormalisasi kemudian
    pub min: f64,
    pub max: f64,
}

/// Data Ingestion Engine untuk parsing dan normalisasi
pub struct DataIngestion;

impl DataIngestion {
    /// Parse JSON string menjadi vektor dengan auto-detection schema
    pub fn parse_json(input: &str) -> SeedResult<ParsedData> {
        let value: Value = serde_json::from_str(input)
            .map_err(|e| SeedError::DeserializationError {
                reason: format!("JSON parse error: {}", e),
            })?;

        Self::parse_json_value(&value)
    }

    /// Parse JSON value (bisa dipanggil recursive)
    fn parse_json_value(value: &Value) -> SeedResult<ParsedData> {
        match value {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Err(SeedError::InvalidConfiguration {
                        reason: "Empty array".to_string(),
                    });
                }

                // Check if nested array (array of arrays)
                if let Value::Array(_) = &arr[0] {
                    Self::parse_nested_array(arr)
                } else {
                    Self::parse_flat_array(arr)
                }
            }
            Value::Object(obj) => {
                // Extract numeric values dari object
                let mut numbers = Vec::new();
                for (_, v) in obj {
                    if let Value::Number(n) = v {
                        if let Some(f) = n.as_f64() {
                            numbers.push(f);
                        }
                    }
                }

                if numbers.is_empty() {
                    return Err(SeedError::InvalidConfiguration {
                        reason: "No numeric values in object".to_string(),
                    });
                }

                Self::normalize_vector(&numbers)
            }
            _ => Err(SeedError::UnknownVectorSchema {
                schema: format!("{:?}", value),
            }),
        }
    }

    /// Parse flat array dari JSON
    fn parse_flat_array(arr: &[Value]) -> SeedResult<ParsedData> {
        let mut numbers = Vec::new();
        for v in arr {
            if let Value::Number(n) = v {
                if let Some(f) = n.as_f64() {
                    numbers.push(f);
                }
            } else {
                return Err(SeedError::UnknownVectorSchema {
                    schema: format!("Mixed types in array: {:?}", v),
                });
            }
        }

        if numbers.is_empty() {
            return Err(SeedError::InvalidConfiguration {
                reason: "No numbers in array".to_string(),
            });
        }

        Self::normalize_vector(&numbers)
    }

    /// Parse nested array dari JSON (matrix)
    fn parse_nested_array(arr: &[Value]) -> SeedResult<ParsedData> {
        let mut matrix: Vec<Vec<f64>> = Vec::new();
        let mut cols = 0;

        for (i, row_val) in arr.iter().enumerate() {
            if let Value::Array(row) = row_val {
                if i == 0 {
                    cols = row.len();
                }

                if row.len() != cols {
                    return Err(SeedError::DimensionMismatch {
                        expected: cols,
                        actual: row.len(),
                    });
                }

                let mut row_nums = Vec::new();
                for v in row {
                    if let Value::Number(n) = v {
                        if let Some(f) = n.as_f64() {
                            row_nums.push(f);
                        }
                    }
                }
                matrix.push(row_nums);
            } else {
                return Err(SeedError::UnknownVectorSchema {
                    schema: "Nested array contains non-array".to_string(),
                });
            }
        }

        // Flatten matrix
        let mut flat = Vec::new();
        for row in matrix {
            flat.extend(row);
        }

        let mut data = Self::normalize_vector(&flat)?;
        data.shape = (arr.len(), cols);
        Ok(data)
    }

    /// Parse CSV string menjadi vektor
    pub fn parse_csv(input: &str, has_header: bool) -> SeedResult<ParsedData> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(has_header)
            .from_reader(input.as_bytes());

        let mut numbers = Vec::new();

        for result in reader.records() {
            let record = result
                .map_err(|e| SeedError::DeserializationError {
                    reason: format!("CSV parse error: {}", e),
                })?;

            for field in record.iter() {
                let num: f64 = field
                    .parse()
                    .map_err(|_| SeedError::DeserializationError {
                        reason: format!("Cannot parse '{}' as f64", field),
                    })?;
                numbers.push(num);
            }
        }

        if numbers.is_empty() {
            return Err(SeedError::InvalidConfiguration {
                reason: "No data in CSV".to_string(),
            });
        }

        Self::normalize_vector(&numbers)
    }

    /// Parse binary format (little-endian f64 array)
    pub fn parse_binary<R: Read>(reader: &mut R) -> SeedResult<ParsedData> {
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .map_err(|e| SeedError::StorageIOError {
                reason: format!("Read error: {}", e),
            })?;

        if buffer.len() % 8 != 0 {
            return Err(SeedError::InvalidConfiguration {
                reason: format!(
                    "Binary data length must be multiple of 8, got {}",
                    buffer.len()
                ),
            });
        }

        let mut numbers = Vec::new();
        for i in (0..buffer.len()).step_by(8) {
            let bytes = [
                buffer[i],
                buffer[i + 1],
                buffer[i + 2],
                buffer[i + 3],
                buffer[i + 4],
                buffer[i + 5],
                buffer[i + 6],
                buffer[i + 7],
            ];
            let f64_val = f64::from_le_bytes(bytes);
            numbers.push(f64_val);
        }

        if numbers.is_empty() {
            return Err(SeedError::InvalidConfiguration {
                reason: "No data in binary".to_string(),
            });
        }

        Self::normalize_vector(&numbers)
    }

    /// Normalisasi vektor ke range [0, 1] dengan min-max scaling
    fn normalize_vector(numbers: &[f64]) -> SeedResult<ParsedData> {
        if numbers.is_empty() {
            return Err(SeedError::InvalidConfiguration {
                reason: "Empty vector".to_string(),
            });
        }

        let min = numbers
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max = numbers
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        let range = max - min;
        let normalized: Vec<f64> = if range < 1e-10 {
            // All values are same
            numbers.iter().map(|_| 0.5).collect()
        } else {
            numbers
                .iter()
                .map(|&x| (x - min) / range)
                .collect()
        };

        Ok(ParsedData {
            vector: DVector::from_vec(normalized),
            shape: (numbers.len(), 1),
            min,
            max,
        })
    }

    /// Denormalisasi vektor kembali ke range asli
    pub fn denormalize(data: &ParsedData) -> Vec<f64> {
        let range = data.max - data.min;
        data.vector
            .iter()
            .map(|&x| x * range + data.min)
            .collect()
    }

    /// Parse dari file dengan auto-detection format berdasarkan extension
    pub fn parse_file(path: &str) -> SeedResult<ParsedData> {
        if path.ends_with(".json") {
            let content = std::fs::read_to_string(path)
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Cannot read file: {}", e),
                })?;
            Self::parse_json(&content)
        } else if path.ends_with(".csv") {
            let content = std::fs::read_to_string(path)
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Cannot read file: {}", e),
                })?;
            Self::parse_csv(&content, true)
        } else if path.ends_with(".bin") {
            let mut file = std::fs::File::open(path)
                .map_err(|e| SeedError::StorageIOError {
                    reason: format!("Cannot open file: {}", e),
                })?;
            Self::parse_binary(&mut file)
        } else {
            Err(SeedError::UnknownVectorSchema {
                schema: format!("Unknown extension for: {}", path),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_flat_array() -> SeedResult<()> {
        let json = "[1.0, 2.0, 3.0, 4.0]";
        let data = DataIngestion::parse_json(json)?;

        assert_eq!(data.vector.len(), 4);
        assert!((data.min - 1.0).abs() < 1e-10);
        assert!((data.max - 4.0).abs() < 1e-10);
        assert!((data.vector[0] - 0.0).abs() < 1e-10); // normalized

        Ok(())
    }

    #[test]
    fn test_parse_json_nested_array() -> SeedResult<()> {
        let json = "[[1.0, 2.0], [3.0, 4.0]]";
        let data = DataIngestion::parse_json(json)?;

        assert_eq!(data.vector.len(), 4);
        assert_eq!(data.shape, (2, 2));

        Ok(())
    }

    #[test]
    fn test_parse_json_object() -> SeedResult<()> {
        let json = r#"{"a": 1.0, "b": 2.0, "c": 3.0}"#;
        let data = DataIngestion::parse_json(json)?;

        assert_eq!(data.vector.len(), 3);

        Ok(())
    }

    #[test]
    fn test_parse_csv() -> SeedResult<()> {
        let csv = "a,b,c\n1.0,2.0,3.0\n4.0,5.0,6.0";
        let data = DataIngestion::parse_csv(csv, true)?;

        assert_eq!(data.vector.len(), 6);

        Ok(())
    }

    #[test]
    fn test_normalize_vector() -> SeedResult<()> {
        let numbers = vec![1.0, 2.0, 3.0, 4.0];
        let data = DataIngestion::normalize_vector(&numbers)?;

        assert!((data.vector[0] - 0.0).abs() < 1e-10);
        assert!((data.vector[3] - 1.0).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn test_denormalize() -> SeedResult<()> {
        let numbers = vec![1.0, 2.0, 3.0, 4.0];
        let data = DataIngestion::normalize_vector(&numbers)?;

        let denormalized = DataIngestion::denormalize(&data);

        for (i, &orig) in numbers.iter().enumerate() {
            assert!((denormalized[i] - orig).abs() < 1e-10);
        }

        Ok(())
    }

    #[test]
    fn test_parse_json_empty_array() {
        let json = "[]";
        assert!(DataIngestion::parse_json(json).is_err());
    }

    #[test]
    fn test_parse_binary() -> SeedResult<()> {
        let numbers: Vec<f64> = vec![1.0, 2.0, 3.0];
        let mut binary = Vec::new();
        for n in &numbers {
            binary.extend_from_slice(&n.to_le_bytes());
        }

        let mut cursor = std::io::Cursor::new(binary);
        let data = DataIngestion::parse_binary(&mut cursor)?;

        assert_eq!(data.vector.len(), 3);

        Ok(())
    }
}

