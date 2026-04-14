//! Benchmarks comparing light, mixed, and dark beam pipelines.
//!
//! Dark beams should be faster than light beams — no computation, just
//! propagation of the fixpoint through the pipeline.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use prism_core::beam::{Beam, Optic};
use prism_core::ScalarLoss;
use terni::Imperfect;

/// Light pipeline: 10-step smap chain, all Success.
fn light_pipeline_10() -> bool {
    let b: Optic<(), u32, String> = Optic::ok((), 1);
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    b.is_ok()
}

/// Mixed pipeline: 10-step smap chain, alternating Success and Partial.
fn mixed_pipeline_10() -> bool {
    let b: Optic<(), u32, String> = Optic::ok((), 1);
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::partial(v + 1, ScalarLoss::new(0.1)));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::partial(v + 1, ScalarLoss::new(0.1)));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::partial(v + 1, ScalarLoss::new(0.1)));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::partial(v + 1, ScalarLoss::new(0.1)));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::partial(v + 1, ScalarLoss::new(0.1)));
    b.is_ok()
}

/// Dark pipeline via smap: Failure at step 0, 10 smaps that never execute.
fn dark_pipeline_smap_10() -> bool {
    let b: Optic<(), u32, String> = Optic::err((), "dark".into());
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    let b = b.smap(|&v| Imperfect::success(v + 1));
    b.is_err()
}

/// Dark pipeline via next: Failure at step 0, 10 nexts that propagate darkness.
fn dark_pipeline_next_10() -> bool {
    let b: Optic<(), u32, String> = Optic::err((), "dark".into());
    let b = b.next(2u32);
    let b = b.next(3u32);
    let b = b.next(4u32);
    let b = b.next(5u32);
    let b = b.next(6u32);
    let b = b.next(7u32);
    let b = b.next(8u32);
    let b = b.next(9u32);
    let b = b.next(10u32);
    let b = b.next(11u32);
    b.is_err()
}

fn bench_pipelines(c: &mut Criterion) {
    let mut group = c.benchmark_group("beam_pipeline");

    group.bench_function("light_10", |b| b.iter(|| black_box(light_pipeline_10())));
    group.bench_function("mixed_10", |b| b.iter(|| black_box(mixed_pipeline_10())));
    group.bench_function("dark_smap_10", |b| {
        b.iter(|| black_box(dark_pipeline_smap_10()))
    });
    group.bench_function("dark_next_10", |b| {
        b.iter(|| black_box(dark_pipeline_next_10()))
    });

    group.finish();
}

criterion_group!(benches, bench_pipelines);
criterion_main!(benches);
