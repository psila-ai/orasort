use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use orasort::prelude::*;
use rand::Rng;
use std::hint::black_box;

fn bench_strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("String Sort");
    group.sample_size(10);

    // Dataset generation
    let mut rng = rand::rng();
    let count = 10_000;

    let random_strings: Vec<String> = (0..count)
        .map(|_| {
            let len = rng.random_range(5..20);
            (0..len).map(|_| rng.random::<char>()).collect()
        })
        .collect();

    // Orasort
    group.bench_function("orasort (in-place)", |b| {
        b.iter_batched(
            || random_strings.clone(),
            |mut data| orasort_mut(black_box(&mut data)),
            BatchSize::SmallInput,
        )
    });

    // Std Sort (Stable)
    group.bench_function("slice::sort (stable)", |b| {
        b.iter_batched(
            || random_strings.clone(),
            |mut data| data.sort(),
            BatchSize::SmallInput,
        )
    });

    // Std Sort Unstable
    group.bench_function("slice::sort_unstable", |b| {
        b.iter_batched(
            || random_strings.clone(),
            |mut data| data.sort_unstable(),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_long_prefix(c: &mut Criterion) {
    let mut group = c.benchmark_group("Long Common Prefix");
    group.sample_size(10);

    // Dataset with heavy prefixes
    let mut rng = rand::rng();
    let count = 10_000;
    let prefix = "common_prefix_which_is_quite_long_indeed_";

    let input: Vec<String> = (0..count)
        .map(|_| {
            let suffix: String = (0..5).map(|_| rng.random::<char>()).collect();
            format!("{}{}", prefix, suffix)
        })
        .collect();

    group.bench_function("orasort (in-place)", |b| {
        b.iter_batched(
            || input.clone(),
            |mut data| orasort_mut(black_box(&mut data)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("slice::sort (stable)", |b| {
        b.iter_batched(
            || input.clone(),
            |mut data| data.sort(),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("slice::sort_unstable", |b| {
        b.iter_batched(
            || input.clone(),
            |mut data| data.sort_unstable(),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_strings, bench_long_prefix);
criterion_main!(benches);
