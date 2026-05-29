# Contributing to SEED

## Development Setup

### Prerequisites
- Rust 1.96.0+ (May 2026 edition)
- Cargo with workspaces support
- ~500 MB disk space for dependencies

### Initial Setup

```bash
# Clone repository
git clone https://github.com/nafalfaturizki-hub/SEED.git
cd SEED

# Build project
cargo build

# Run tests
cargo test --lib

# Run benchmarks
cargo bench
```

## Code Organization

### Module Structure

```
src/
├── lib.rs                 # Root: SeedError, module exports
├── main.rs                # CLI: subcommands, server
├── core/                  # Mathematical foundation
│   ├── fractal.rs         # Affine transformations
│   └── entropy.rs         # Entropy reduction
├── engine/                # Data processing
│   ├── ingestion.rs       # Multi-format parsing
│   └── reconstruction.rs  # Fixed-point iteration
└── storage/               # Persistence
    └── matrix_store.rs    # mmap + CRC storage
```

### Adding New Features

1. **Create new module file**: `src/engine/new_feature.rs`
2. **Export in parent**: Add `pub mod new_feature;` to `src/engine/mod.rs`
3. **Export in lib.rs**: Add `pub use engine::new_feature::*;`
4. **Add tests**: Module-level `#[cfg(test)]` section
5. **Document public API**: Rustdoc comments on public items

Example:
```rust
// src/engine/new_feature.rs
use crate::SeedError;
use nalgebra::DVector;

/// Short description of feature
/// 
/// # Arguments
/// * `input` - Input vector
/// 
/// # Returns
/// * `Ok(output)` - Success case
/// * `Err(SeedError)` - Failure case
pub fn process_feature(input: &DVector<f64>) -> crate::SeedResult<DVector<f64>> {
    // Implementation
    Ok(input.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process_feature() {
        // Test implementation
    }
}
```

## Coding Standards

### Error Handling

**Always use `SeedResult<T>`** for fallible operations:

```rust
// ✓ CORRECT
pub fn do_something(x: f64) -> SeedResult<f64> {
    if x < 0.0 {
        return Err(SeedError::InvalidConfiguration);
    }
    Ok(x.sqrt())
}

// ✗ INCORRECT - Don't use unwrap/panic
pub fn bad_function(x: f64) -> f64 {
    x.sqrt()  // panics on negative x
}
```

### Memory Safety

**Unsafe code requires SAFETY comments**:

```rust
// ✓ CORRECT
unsafe {
    // SAFETY: File is opened read-only, ensuring mmap safety
    let mmap = Mmap::map(&file)?;
}

// ✗ INCORRECT - No safety justification
unsafe {
    let mmap = Mmap::map(&file)?;  // Unsafe why?
}
```

### Type Annotations

**Explicit return types on public functions**:

```rust
// ✓ CORRECT
pub fn calculate(input: f64) -> f64 {
    input * 2.0
}

// ✗ INCORRECT - Implicit return type
pub fn calculate(input: f64) {
    input * 2.0
}
```

### Documentation

**Comprehensive doc comments**:

```rust
/// Computes Frobenius norm distance between matrices
/// 
/// The Frobenius norm is defined as:
/// ||ΔM||_F = √(Σ_{i,j} (ΔM_{ij})²)
/// 
/// # Arguments
/// * `m_old` - First matrix (n × m)
/// * `m_new` - Second matrix (n × m)
/// 
/// # Returns
/// * `Ok(distance)` - Non-negative f64 distance
/// * `Err(DimensionMismatch)` - If matrices have different dimensions
/// 
/// # Examples
/// ```
/// let m1 = DMatrix::identity(2, 2);
/// let m2 = DMatrix::identity(2, 2) * 0.9;
/// let dist = calculate_matrix_distance(&m1, &m2)?;
/// assert!(dist < 0.2);
/// ```
pub fn calculate_matrix_distance(
    m_old: &DMatrix<f64>,
    m_new: &DMatrix<f64>,
) -> SeedResult<f64> {
    // Implementation
}
```

## Testing

### Test Organization

Each module should have comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Happy path
    #[test]
    fn test_basic_functionality() {
        assert!(basic_case_works());
    }
    
    // Edge cases
    #[test]
    fn test_empty_input() {
        assert_eq!(handle_empty(), Ok(default_value()));
    }
    
    // Error cases
    #[test]
    fn test_error_handling() {
        assert!(matches!(
            invalid_input(),
            Err(SeedError::InvalidConfiguration)
        ));
    }
}
```

### Running Tests

```bash
# All tests
cargo test --lib

# Specific test
cargo test --lib test_name

# Integration tests
cargo test --lib integration_tests -- --nocapture

# With output
cargo test --lib -- --nocapture
```

### Benchmark Guidelines

New performance-critical functions should have benchmarks:

```bash
# Run benchmarks
cargo bench --bench seed_benchmarks

# Specific benchmark
cargo bench affine_transformation
```

## Performance Considerations

### Optimization Checklist

- [ ] Function has benchmark in `benches/seed_benchmarks.rs`
- [ ] No unnecessary heap allocations in hot loops
- [ ] Matrix operations use nalgebra in-place variants where possible
- [ ] Linear algebra complexity is O(n²) or better for n-dimensional vectors
- [ ] CRC/checksum computed once, cached if repeated

### Common Optimizations

**Avoid cloning in tight loops**:
```rust
// ✗ SLOW - Clones in every iteration
for i in 0..1000 {
    let copy = matrix.clone();  // Expensive!
    process(&copy);
}

// ✓ FAST - Use reference
for i in 0..1000 {
    process(&matrix);  // No clone
}
```

**Use in-place operations**:
```rust
// ✗ SLOW - Creates new matrix
let result = &matrix1 + &matrix2 + &matrix3;

// ✓ FAST - Modifies in place
let mut result = matrix1;
result += &matrix2;
result += &matrix3;
```

## Dependency Management

### Adding Dependencies

1. **Check existing alternatives**: Could nalgebra function suffice?
2. **Pin major versions**: `serde = "1"` (not `"1.0.0"`)
3. **Use conservative ranges**: Avoid `*` or overly wide ranges
4. **Document rationale**: Add comment explaining why needed

### Approved Dependencies

**Core**:
- `nalgebra` - Linear algebra (essential)
- `tokio` - Async runtime (essential for TCP)
- `serde` + `bincode` - Serialization (essential for storage)

**Optional**:
- `tracing` - Structured logging (recommended)
- `csv` - Data format support (approved)
- `memmap2` - Performance (approved with unsafe)

**NOT Approved**:
- `ndarray` - nalgebra already used
- `rayon` - Not needed for MVP (future optimization)

## Git Workflow

### Branch Naming

- `feature/feature-name` - New features
- `fix/bug-description` - Bug fixes
- `perf/optimization-name` - Performance work
- `docs/documentation-name` - Documentation only

### Commit Messages

```
feat: Add adaptive reconstruction algorithm

- Implement progressive epsilon refinement
- Add reconstruct_adaptive() method
- Update integration tests
- Performance: 50% faster convergence on ill-conditioned systems

Fixes #42
```

**Format**:
- First line: `type: short description` (50 chars max)
- Blank line
- Detailed explanation (optional)
- Footer: Issue references

**Types**: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `chore`

### Pull Request Checklist

- [ ] Tests pass: `cargo test --lib`
- [ ] Format correct: `cargo fmt`
- [ ] Lints pass: `cargo clippy`
- [ ] New tests added for new features
- [ ] Documentation updated
- [ ] Benchmarks added if performance-critical
- [ ] No unsafe code without SAFETY comments
- [ ] No clippy warnings

## Documentation Guidelines

### README Updates

Update [README.md](README.md) when:
- Adding new CLI command
- Adding significant feature
- Changing architecture
- Adding new dependency

### ARCHITECTURE Updates

Update [ARCHITECTURE.md](ARCHITECTURE.md) when:
- Adding/removing module
- Changing API signature
- Adding new algorithm
- Documenting limitations

### Code Comments

- **WHY not WHAT**: Comment explains reasoning
- **SAFETY comments**: Required for unsafe blocks
- **TODO comments**: Link to GitHub issue if applicable

```rust
// ✓ GOOD - Explains reasoning
// Sigmoid normalization prevents ||M|| >= 1 by construction,
// ensuring contraction property is mathematically guaranteed
fn normalize_via_sigmoid(norm: f64) -> f64 {

// ✗ BAD - States obvious
// Multiply matrix by norm
let m = matrix * norm;
```

## Release Process

### Version Bumping

Uses semantic versioning:
- `0.1.0` → `0.2.0` for feature releases
- `0.1.0` → `0.1.1` for bug fixes
- `1.0.0` for production release

Update in:
1. `Cargo.toml`: `version = "0.2.0"`
2. `src/main.rs`: `SEED Core Engine v0.2.0` in info command
3. Create GitHub Release with changelog

### Pre-Release Checklist

- [ ] All tests passing
- [ ] Benchmarks show no regressions
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version bumped consistently
- [ ] No known issues blocking release

## Performance Profiling

### Benchmarking with Criterion

```bash
# Generate HTML report
cargo bench --bench seed_benchmarks -- --verbose

# Compare against baseline
cargo bench --bench seed_benchmarks -- --save-baseline my_baseline
cargo bench --bench seed_benchmarks -- --baseline my_baseline
```

### Flame Graph Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile specific operation
cargo flamegraph --bin sedd-core -- dissolve --input data.json
```

## Troubleshooting

### Common Issues

**Issue**: `error: could not compile sedd-core (bin "sedd-core")`

**Solution**: 
1. Check import paths: `crate::core::fractal::` not `sedd_core::`
2. Verify module exports in parent `mod.rs`
3. Run `cargo check` for more details

**Issue**: Tests pass locally but fail in CI

**Solution**:
1. Check for platform-specific code (Windows vs Linux)
2. Verify no hardcoded absolute paths
3. Ensure tests clean up temporary files

**Issue**: Benchmark results are noisy

**Solution**:
1. Increase sample size: `--benchmark-samples 1000`
2. Run on quiet system (close other programs)
3. Use `--warm-up-time 5` to stabilize

## Getting Help

- **Architecture questions**: See [ARCHITECTURE.md](ARCHITECTURE.md)
- **API questions**: Read rustdoc: `cargo doc --open`
- **Performance**: Check [benches/seed_benchmarks.rs](benches/seed_benchmarks.rs)
- **Examples**: See [examples/](examples/) directory

---

**Happy contributing to SEED!** 🌱
