mod core;

use nalgebra::DVector;
use crate::core::fractal::StructuralFractalCore;

fn main() {
    println!("--- Mula Jalur Struktur SEDD Engine ---");

    // Kita asumsikan data kita memiliki 3 dimensi (misal: ID Personel, Kode Lokasi, Nilai Transaksi)
    let banyakkah_dimensi = 3;
    let mut db_core = StructuralFractalCore::new(banyakkah_dimensi);

    // Simulasi Data Mentah 1
    let data_personel_1 = DVector::from_vec(vec![101.0, 45.0, 15000.0]);
    // Simulasi Data Mentah 2
    let data_personel_2 = DVector::from_vec(vec![102.0, 12.0, 75000.0]);

    println!("Melarutkan data 1 ke dalam rumus...");
    db_core.dissolve_data(data_personel_1);

    println!("Melarutkan data 2 ke dalam rumus...");
    db_core.dissolve_data(data_personel_2);

    println!("Jumlah transformasi fraktal saat ini: {}", db_core.transformations.len());
    println!("Sistem berhasil mengonversi data menjadi matriks koefisien murni!");
}
