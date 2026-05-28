#![forbid(unsafe_code)]
//! SEED CLI Server: IPC/TCP listener dan daemon handler

use sedd_core::core::StructuralFractalCore;
use sedd_core::SeedResult;

fn main() -> SeedResult<()> {
    println!("SEED Core Engine v0.1.0");
    println!("Structural Entropy Dissolution Database");
    println!();

    // Inisialisasi core fractal dengan dimensi 2
    let core = StructuralFractalCore::new(2)?;
    println!("✓ Fractal core initialized with dimension: 2");
    println!("✓ Active transformations: {}", core.transformation_count());
    println!();

    println!("SEED is ready for ingestion and dissolution operations.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_initialization() {
        let core = StructuralFractalCore::new(2).unwrap();
        assert_eq!(core.transformation_count(), 0);
    }
}
