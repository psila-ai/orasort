use orasort::core::KeyAccessor;
use orasort::prelude::*;

// Simulate an external struct (like from apache-arrow)
struct MockArrowArray {
    data: Vec<u8>,
    offsets: Vec<usize>,
}

impl MockArrowArray {
    fn new(strings: &[&str]) -> Self {
        let mut data = Vec::new();
        let mut offsets = vec![0];
        for s in strings {
            data.extend_from_slice(s.as_bytes());
            offsets.push(data.len());
        }
        Self { data, offsets }
    }
}

// Implement KeyAccessor for the external struct.
// This proves the trait is implementable by "outside crates".
impl KeyAccessor for MockArrowArray {
    fn get_key(&self, index: usize) -> &[u8] {
        let start = self.offsets[index];
        let end = self.offsets[index + 1];
        &self.data[start..end]
    }

    fn len(&self) -> usize {
        self.offsets.len() - 1
    }
}

#[test]
fn test_external_struct_compatibility() {
    let mock = MockArrowArray::new(&["foo", "bar", "baz"]);
    let indices = orasort(&mock);

    // sorted: bar (1), baz (2), foo (0)
    assert_eq!(indices, vec![1, 2, 0]);
}
