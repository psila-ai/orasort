use orasort::prelude::*;
use rand::Rng;
use std::time::Instant;

#[test]
fn test_sort_1m() {
    let count = 1_000_000;
    println!("Generating {} random elements...", count);

    let mut rng = rand::rng();
    let mut input: Vec<Vec<u8>> = Vec::with_capacity(count);

    for _ in 0..count {
        let len = rng.random_range(4..16);
        let mut row = vec![0u8; len];
        rng.fill(&mut row[..]);
        input.push(row);
    }

    println!("Sorting {} elements...", count);
    let start = Instant::now();
    let indices = orasort(&input);
    let duration = start.elapsed();
    println!("Sorted 1M elements in {:?}", duration);

    assert_eq!(indices.len(), count);

    // limited verification to save time
    for i in 0..count - 1 {
        let a = &input[indices[i]];
        let b = &input[indices[i + 1]];
        assert!(a <= b, "Sort failed at index {}", i);
    }
}

#[test]
#[ignore]
fn test_sort_1b() {
    // WARNING: This test requires significant RAM (32GB+).
    // 1B elements * (24 bytes Vec overhead + ~8 bytes data) = ~32GB data
    // Orasort overhead: 1B * 16 bytes SortPtr = 16GB
    // Output overhead: 1B * 8 bytes usize = 8GB
    let count = 1_000_000_000;
    println!(
        "Generating {} random elements... (Expect high RAM usage)",
        count
    );

    let mut rng = rand::rng();
    // Use a flat buffer to reduce Vec overhead?
    // Vec<Vec<u8>> is too heavy for 1B items (24 bytes per struct).
    // Let's use a custom struct with flat storage to make it feasible on 64GB machines.

    struct FlatStorage {
        data: Vec<u8>,
        offsets: Vec<usize>,
    }

    impl orasort::core::KeyAccessor for FlatStorage {
        fn get_key(&self, index: usize) -> &[u8] {
            let start = self.offsets[index];
            let end = if index + 1 < self.offsets.len() {
                self.offsets[index + 1]
            } else {
                self.data.len()
            };
            &self.data[start..end]
        }
        fn len(&self) -> usize {
            self.offsets.len()
        }
    }

    // Generate 1B items of 8 bytes each = 8GB data + 8GB offsets = 16GB input.
    // + 24GB orasort overhead = ~40GB total.
    let mut storage = FlatStorage {
        data: vec![0u8; count * 8],
        offsets: Vec::with_capacity(count),
    };

    println!("Filling data...");
    rng.fill(&mut storage.data[..]);
    for i in 0..count {
        storage.offsets.push(i * 8);
    }

    println!("Sorting 1B elements...");
    let start = Instant::now();
    let indices = orasort(&storage);
    let duration = start.elapsed();
    println!("Sorted 1B elements in {:?}", duration);

    assert_eq!(indices.len(), count);

    // Verify sample
    for i in (0..count - 1).step_by(10_000) {
        let start_a = storage.offsets[indices[i]];
        let a = &storage.data[start_a..start_a + 8];

        let start_b = storage.offsets[indices[i + 1]];
        let b = &storage.data[start_b..start_b + 8];

        assert!(a <= b, "Sort failed at index {}", i);
    }
}
