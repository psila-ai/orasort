use orasort::prelude::*;
use rand::Rng;

#[test]
fn test_basic_sort_strings() {
    let input = vec![
        "banana".to_string(),
        "apple".to_string(),
        "cherry".to_string(),
        "date".to_string(),
    ];

    let indices = orasort(&input);

    let sorted: Vec<&String> = indices.iter().map(|&i| &input[i]).collect();
    assert_eq!(sorted, vec!["apple", "banana", "cherry", "date"]);
}

#[test]
fn test_long_common_prefix() {
    // Generate strings with long prefix
    let prefix = "a".repeat(100);
    let input = vec![
        format!("{}c", prefix),
        format!("{}a", prefix),
        format!("{}b", prefix),
    ];

    let indices = orasort(&input);
    let sorted: Vec<&String> = indices.iter().map(|&i| &input[i]).collect();

    // Check order
    assert!(sorted[0].ends_with("a"));
    assert!(sorted[1].ends_with("b"));
    assert!(sorted[2].ends_with("c"));
}

#[test]
fn test_cache_boundary_sort() {
    // Test differences at byte 7, 8, 9 to verify u64 cache logic.
    // offsets 0..7 are in first cache. 8 is start of next.

    let base = vec![0u8; 16];

    let mut v1 = base.clone();
    v1[7] = 2; // In first 8 bytes
    let mut v2 = base.clone();
    v2[7] = 1;

    let mut v3 = base.clone();
    v3[8] = 2; // At boundary
    let mut v4 = base.clone();
    v4[8] = 1;

    let mut v5 = base.clone();
    v5[9] = 2; // Past boundary
    let mut v6 = base.clone();
    v6[9] = 1;

    let input = vec![
        v1.clone(),
        v2.clone(),
        v3.clone(),
        v4.clone(),
        v5.clone(),
        v6.clone(),
    ];
    // Expected order:
    // v2 (byte 7=1), v1 (byte 7=2)  -- These are unequal at pos 7.
    // v3, v4, v5, v6?
    // Wait, all start with 0s.
    // v2 vs v1: v2 < v1.
    // v4 vs v3: v4 < v3 (diff at 8).
    // v6 vs v5: v6 < v5 (diff at 9).
    //
    // All v3, v4, v5, v6 have 0 at pos 7. So they are < v2 (if v2[7]=1 > 0).
    // ACTUALLY: v2[7]=1, v3[7]=0. So v3 < v2.
    //
    // Let's rely on standard sort to verify correctness.

    let indices = orasort(&input);

    // Verify against std sort
    let mut expected = input.clone();
    expected.sort();

    let actual: Vec<Vec<u8>> = indices.iter().map(|&i| input[i].clone()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_fuzz_random() {
    let mut rng = rand::rng();
    let mut input: Vec<Vec<u8>> = Vec::new();

    for _ in 0..10_000 {
        let len = rng.random_range(0..50);
        let mut row = vec![0u8; len];
        rng.fill(&mut row[..]);
        input.push(row);
    }

    let indices = orasort(&input);

    let mut expected = input.clone();
    expected.sort();

    let actual: Vec<Vec<u8>> = indices.iter().map(|&i| input[i].clone()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_fuzz_random_mut() {
    let mut rng = rand::rng();

    for _ in 0..10_000 {
        let len = rng.random_range(0..50);
        let mut row = vec![0u8; len];
        rng.fill(&mut row[..]);

        // Create random input
        let count = rng.random_range(0..20);
        let mut input: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let inner_len = rng.random_range(0..50);
                let mut inner = vec![0u8; inner_len];
                rng.fill(&mut inner[..]);
                inner
            })
            .collect();

        let mut expected = input.clone();
        expected.sort();

        orasort_mut(&mut input);
        assert_eq!(input, expected);
    }
}

#[test]
fn test_fuzz_random_mut_large() {
    let mut rng = rand::rng();

    // 100 iterations of larger sorts
    for _ in 0..100 {
        let count = rng.random_range(100..1000);
        let mut input: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let inner_len = rng.random_range(0..100);
                let mut inner = vec![0u8; inner_len];
                rng.fill(&mut inner[..]);
                inner
            })
            .collect();

        let mut expected = input.clone();
        expected.sort();

        orasort_mut(&mut input);
        assert_eq!(input, expected);
    }
}

#[test]
fn test_fuzz_edge_cases_mut() {
    // 1. All empty
    let mut input = vec![vec![]; 50];
    let expected = input.clone();
    orasort_mut(&mut input);
    assert_eq!(input, expected);

    // 2. All same
    let mut input = vec![vec![1, 2, 3]; 50];
    let expected = input.clone();
    orasort_mut(&mut input);
    assert_eq!(input, expected);

    // 3. Reversed
    let mut input: Vec<Vec<u8>> = (0..50).map(|i| vec![i as u8]).rev().collect();
    let mut expected = input.clone();
    expected.sort();
    orasort_mut(&mut input);
    assert_eq!(input, expected);

    // 4. Sorted
    let mut input: Vec<Vec<u8>> = (0..50).map(|i| vec![i as u8]).collect();
    let expected = input.clone();
    orasort_mut(&mut input);
    assert_eq!(input, expected);
}

#[test]
fn test_sort_string_bytes() {
    let input = "banana";
    let indices = orasort(input); // Impl implemented for str

    // Check indices into the string
    // "banana" -> b, a, n, a, n, a
    // indices should point to a, a, a, b, n, n
    // 'a' at 1, 3, 5. 'b' at 0. 'n' at 2, 4.

    let sorted_bytes: Vec<u8> = indices.iter().map(|&i| input.as_bytes()[i]).collect();
    assert_eq!(sorted_bytes, b"aaabnn".to_vec());
}

#[test]
fn test_vec_deque() {
    use std::collections::VecDeque;
    let input: VecDeque<String> = VecDeque::from(vec![
        "banana".to_string(),
        "apple".to_string(),
        "cherry".to_string(),
    ]);

    let indices = orasort(&input);

    let sorted: Vec<&String> = indices.iter().map(|&i| &input[i]).collect();
    assert_eq!(sorted, vec!["apple", "banana", "cherry"]);
}

#[test]
fn test_empty() {
    let input: Vec<String> = vec![];
    let indices = orasort(&input);
    assert!(indices.is_empty());
}

#[test]
fn test_mutable_sort() {
    let mut data = vec![
        "banana".to_string(),
        "apple".to_string(),
        "cherry".to_string(),
    ];
    orasort_mut(&mut data);
    assert_eq!(data, vec!["apple", "banana", "cherry"]);
}
