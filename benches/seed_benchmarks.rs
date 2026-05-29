use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use nalgebra::{DMatrix, DVector};
use sedd_core::core::fractal::{StructuralFractalCore, AffineTransformation};
use sedd_core::core::entropy::calculate_matrix_distance;
use sedd_core::engine::ingestion::DataIngestion;

fn benchmark_affine_transformation(c: &mut Criterion) {
    c.bench_function("affine_new_2x2", |b| {
        b.iter(|| {
            let matrix = DMatrix::identity(2, 2) * 0.5;
            let bias = DVector::zeros(2);
            AffineTransformation::new(black_box(matrix), black_box(bias))
        })
    });

    c.bench_function("affine_new_10x10", |b| {
        b.iter(|| {
            let matrix = DMatrix::identity(10, 10) * 0.5;
            let bias = DVector::zeros(10);
            AffineTransformation::new(black_box(matrix), black_box(bias))
        })
    });

    c.bench_function("affine_apply_2x2", |b| {
        let matrix = DMatrix::identity(2, 2) * 0.5;
        let bias = DVector::zeros(2);
        let transform = AffineTransformation::new(matrix, bias).unwrap();
        let x = DVector::from_vec(vec![1.0, 2.0]);

        b.iter(|| transform.apply(black_box(&x)))
    });
}

fn benchmark_entropy_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("entropy_distance");

    for size in [2, 5, 10, 20].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let m_old = DMatrix::identity(size, size) * 0.5;
            let m_new = DMatrix::identity(size, size) * 0.49;

            b.iter(|| calculate_matrix_distance(black_box(&m_old), black_box(&m_new)))
        });
    }

    group.finish();
}

fn benchmark_data_ingestion(c: &mut Criterion) {
    c.bench_function("parse_json_small", |b| {
        let json = r#"[0.1, 0.2, 0.3, 0.4, 0.5]"#;
        b.iter(|| DataIngestion::parse_json(black_box(json)))
    });

    c.bench_function("parse_json_large", |b| {
        let json = (0..1000)
            .map(|i| format!("{}", (i as f64) / 1000.0))
            .collect::<Vec<_>>()
            .join(",");
        let json_str = format!("[{}]", json);
        b.iter(|| DataIngestion::parse_json(black_box(&json_str)))
    });

    c.bench_function("parse_csv_small", |b| {
        let csv = "a,b,c\n1.0,2.0,3.0\n4.0,5.0,6.0";
        b.iter(|| DataIngestion::parse_csv(black_box(csv), true))
    });
}

fn benchmark_fractal_core(c: &mut Criterion) {
    c.bench_function("core_creation_2d", |b| {
        b.iter(|| StructuralFractalCore::new(black_box(2)))
    });

    c.bench_function("core_add_transformation", |b| {
        let mut core = StructuralFractalCore::new(2).unwrap();
        let matrix = DMatrix::identity(2, 2) * 0.5;
        let bias = DVector::zeros(2);
        let transform = AffineTransformation::new(matrix, bias).unwrap();

        b.iter(|| core.add_transformation(black_box(transform.clone())))
    });
}

criterion_group!(
    benches,
    benchmark_affine_transformation,
    benchmark_entropy_distance,
    benchmark_data_ingestion,
    benchmark_fractal_core
);
criterion_main!(benches);
