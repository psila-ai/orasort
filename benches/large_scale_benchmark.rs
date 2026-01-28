use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use orasort::prelude::*;
use rand::Rng;
use std::hint::black_box;
use std::time::Duration;

fn bench_1m_strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("1M Strings");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(90)); // Increase time for large sort setup overhead

    // Dataset generation
    let mut rng = rand::rng();
    let count = 1_000_000;

    // Generate ~16MB of string data (avg length 16)
    // 1M * 16 bytes = 16MB data + 24MB structs = 40MB total
    let random_strings: Vec<String> = (0..count)
        .map(|_| {
            let len = rng.random_range(8..24);
            (0..len).map(|_| rng.random::<char>()).collect()
        })
        .collect();

    // Calculate approximate size for throughput
    let total_bytes: usize = random_strings.iter().map(|s| s.len()).sum();
    group.throughput(Throughput::Bytes(total_bytes as u64));

    // Orasort
    group.bench_function("orasort (in-place)", |b| {
        b.iter_batched(
            || random_strings.clone(),
            |mut data| orasort_mut(black_box(&mut data)),
            BatchSize::LargeInput,
        )
    });

    // Std Sort (Stable)
    group.bench_function("slice::sort (stable)", |b| {
        b.iter_batched(
            || random_strings.clone(),
            |mut data| data.sort(),
            BatchSize::LargeInput,
        )
    });

    // Std Sort Unstable
    group.bench_function("slice::sort_unstable", |b| {
        b.iter_batched(
            || random_strings.clone(),
            |mut data| data.sort_unstable(),
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_1m_strings);
criterion_main!(benches);
