use orasort::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn test_zeroed_entries() {
    let mut rng = StdRng::seed_from_u64(42);

    for _iter in 0..10 {
        let len = rng.random_range(2000..5000);
        // eprintln!("Iter {} len {}", iter, len);
        let mut input: Vec<Vec<u8>> = Vec::new();

        for _ in 0..len {
            let row_len = rng.random_range(0..4); // Keep short to trigger [0] vs []
            let mut row = vec![0u8; row_len];
            rng.fill(&mut row[..]);
            input.push(row);
        }

        let indices = orasort(&input);

        let mut expected = input.clone();
        expected.sort();

        let actual: Vec<Vec<u8>> = indices.iter().map(|&i| input[i].clone()).collect();

        if actual != expected {
            // Find first mismatch
            for (i, (a, b)) in actual.iter().zip(expected.iter()).enumerate() {
                if a != b {
                    panic!("Mismatch at index {}: Got {:?}, Expected {:?}", i, a, b);
                }
            }
            panic!(
                "Lengths differ? Actual: {}, Expected: {}",
                actual.len(),
                expected.len()
            );
        }
    }
}
