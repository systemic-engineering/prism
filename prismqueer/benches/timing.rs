//! Wall-clock claims, moved out of the correctness gate.
//!
//! A gate verdict must be a deterministic function of its key, and a timing
//! depends on load, instrumentation and the machine (docs/spec/checks.md,
//! to-fix 3, 4 and 7). The gate keeps the deterministic form of each claim;
//! the time itself is measured here, by `just bench timing`.
//!
//! - `fate_runtime/interpreted`: was `runtime_performance`, "under 1 ms per
//!   inference". The gate keeps `runtime_halts_within_its_step_budget`
//!   (at most 100k BF steps).
//! - `fate_runtime/compiled` against `interpreted`: was `compiled_performance`,
//!   "compiled at least 2x faster". The gate keeps
//!   `compiled_program_folds_to_at_most_half_the_instructions`. Read the
//!   claim off the two criterion means.
//! - `derive_prism_oid`: was `named::benchmark_derive_prism_hash`, which
//!   timed 1k `oid()` calls and asserted nothing.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use prismqueer::fate::runtime::{CompiledFateRuntime, FateRuntime};
use prismqueer::fate::{Model, FEATURE_DIM};
use prismqueer::Addressable;

#[derive(prismqueer::DerivePrism)]
#[oid("@bench/simple")]
struct Simple;

fn fate_runtime(c: &mut Criterion) {
    let features = [0.0f64; FEATURE_DIM];
    let interpreted = FateRuntime::new();
    let compiled = CompiledFateRuntime::new();
    let mut group = c.benchmark_group("fate_runtime");
    group.bench_function("interpreted", |b| {
        b.iter(|| interpreted.select(black_box(Model::Abyss), black_box(&features)))
    });
    group.bench_function("compiled", |b| {
        b.iter(|| compiled.select(black_box(Model::Abyss), black_box(&features)))
    });
    group.finish();
}

fn derive_prism_oid(c: &mut Criterion) {
    c.bench_function("derive_prism_oid", |b| b.iter(|| black_box(&Simple).oid()));
}

criterion_group!(benches, fate_runtime, derive_prism_oid);
criterion_main!(benches);
