//! Core sorting algorithms (CPS-Quicksort and Adaptive Radix Sort).
//!
//! This module implements the [Orasort](https://patents.google.com/patent/US7680791B2) algorithm, which is a hybrid of:
//! - **CPS-Quicksort**: Quicksort extended with Common Prefix Skipping (CPS).
//! - **Adaptive Radix Sort**: Used for large partitions to improve locality and avoid excessive comparisons.
//! - **Insertion Sort**: Fallback for small partitions.
//!
//! The main entry points are [`orasort`] and [`orasort_mut`].

use crate::core::{KeyAccessor, SortPtr};
use cuneiform::cuneiform;
use std::cmp::Ordering;

const INSERTION_SORT_THRESHOLD: usize = 32;
const RADIX_SORT_THRESHOLD: usize = 1024;

/// Helper to load 8 bytes from a slice into a `u64` (big-endian), 0-padded.
///
/// If `offset` is beyond the end of the slice, returns 0.
/// If `offset` leaves fewer than 8 bytes, the remaining bytes are loaded into the
/// most significant positions, and the rest are 0-padded.
#[inline(always)]
fn load_u64_be(bytes: &[u8], offset: usize) -> u64 {
    let mut buf = [0u8; 8];
    if offset >= bytes.len() {
        return 0;
    }

    let available = bytes.len() - offset;
    if available >= 8 {
        let src = &bytes[offset..offset + 8];
        buf.copy_from_slice(src);
    } else {
        let src = &bytes[offset..];
        buf[..available].copy_from_slice(src);
    }

    u64::from_be_bytes(buf)
}

/// Performs an index-based sort on the provided collection.
///
/// This function does not modify the input collection. Instead, it returns a `Vec<usize>`
/// containing the indices that would strictly order the collection.
///
/// The input collection must implement the [`KeyAccessor`] trait, which abstracts
/// byte-slice access.
///
/// # Arguments
///
/// * `provider` - The collection to be sorted.
///
/// # Returns
///
/// A vector of indices such that `provider.get_key(indices[i]) <= provider.get_key(indices[i+1])`.
///
/// # Examples
///
/// ```
/// use orasort::orasort;
///
/// let data = vec!["banana", "apple", "cherry"];
/// let indices = orasort(&data);
///
/// assert_eq!(indices, vec![1, 0, 2]); // apple, banana, cherry
/// ```
pub fn orasort<T: KeyAccessor + ?Sized>(provider: &T) -> Vec<usize> {
    let len = provider.len();
    if len == 0 {
        return vec![];
    }

    // Initialize SortPtrs with the first 8 bytes.
    let mut pointers: Vec<SortPtr> = (0..len)
        .map(|index| {
            let key = provider.get_key(index);
            let cache = load_u64_be(key, 0);
            SortPtr { index, cache }
        })
        .collect();

    cps_quicksort(provider, &mut pointers, 0, true);

    pointers.into_iter().map(|p| p.index).collect()
}

/// Sorts a mutable slice in-place.
///
/// This is a convenience wrapper for [`orasort`] which computes the sorted indices
/// and then applies the permutation to the slice.
///
/// # Arguments
///
/// * `data` - A mutable slice of items that implement `AsRef<[u8]>` and `Clone`.
///
/// # Examples
///
/// ```
/// use orasort::orasort_mut;
///
/// let mut data = vec!["banana", "apple", "cherry"];
/// orasort_mut(&mut data);
///
/// assert_eq!(data, vec!["apple", "banana", "cherry"]);
/// ```
pub fn orasort_mut<T: AsRef<[u8]> + Clone>(data: &mut [T]) {
    // 1. Get indices
    let indices = orasort(data);

    // 2. Permute in-place (simplest via auxiliary vector if T is Clone)
    // Minimizing allocations for large T is hard without unsafe or specific traits.
    // For now, use simple auxiliary buffer approach for safety.
    apply_permutation(data, indices);
}

fn apply_permutation<T: Clone>(data: &mut [T], mut indices: Vec<usize>) {
    for i in 0..data.len() {
        let mut current = i;
        while indices[current] != i {
            let next = indices[current];
            data.swap(current, next);
            indices[current] = current; // Mark as visited/placed
            current = next;
        }
        indices[current] = current;
    }
}

/// Common Prefix Skipping Quicksort (CPS-QS).
///
/// Recursively sorts the `ptrs` slice.
/// * `cp_len`: The length of the common prefix shared by all keys in this slice.
/// * `allow_radix`: Whether to attempt switching to Adaptive Radix Sort (AQS) for large inputs.
fn cps_quicksort<T: KeyAccessor + ?Sized>(
    provider: &T,
    ptrs: &mut [SortPtr],
    cp_len: usize,
    allow_radix: bool,
) {
    let len = ptrs.len();

    if len < INSERTION_SORT_THRESHOLD {
        insertion_sort(provider, ptrs, cp_len);
        return;
    }

    if allow_radix && len > RADIX_SORT_THRESHOLD {
        aqs_radix(provider, ptrs, cp_len);
        return;
    }

    // Median of 3 pivot selection
    let mid = len / 2;
    ptrs.swap(0, mid);
    let pivot = ptrs[0];

    // 3-way Partitioning (<, =, >)
    let mut lt = 0;
    let mut i = 1;
    let mut gt = len;

    while i < gt {
        let ordering = compare_entries(provider, &ptrs[i], &pivot, cp_len);
        match ordering {
            Ordering::Less => {
                ptrs.swap(lt, i);
                lt += 1;
                i += 1;
            }
            Ordering::Greater => {
                gt -= 1;
                ptrs.swap(i, gt);
            }
            Ordering::Equal => {
                i += 1;
            }
        }
    }

    // Recurse
    if lt > 0 {
        cps_quicksort(provider, &mut ptrs[0..lt], cp_len, true);
    }

    if gt < len {
        cps_quicksort(provider, &mut ptrs[gt..len], cp_len, true);
    }

    // Equal partition is already sorted for this prefix depth.
}

/// Number of buckets for Radix sort (256 for byte-wise).
const RADIX_BUCKETS: usize = 256;

// Cache-aligned counts struct.
#[cuneiform]
struct RadixCounts {
    data: [usize; RADIX_BUCKETS],
}

/// Adaptive Radix Sort Step.
///
/// Distributes keys into 256 buckets based on the next byte of the key (from cache).
///
/// 1. Counts frequencies of each byte (histograms).
/// 2. Computes prefix sums to determine bucket starting positions.
/// 3. Permutes elements into a temporary buffer and writes them back in sorted bucket order.
/// 4. Recursively calls `cps_quicksort` on each bucket.
fn aqs_radix<T: KeyAccessor + ?Sized>(provider: &T, ptrs: &mut [SortPtr], mut cp_len: usize) {
    let mut bytes_since_load = 0; // Track how many bytes we consumed from the current cache load

    loop {
        // Optimization: Block Skip (Check for multiple common bytes)
        // Scans the cache to find how many leading bytes are identical across all items.
        // This is significantly faster than the histogram approach for long common prefixes.
        let anchor = ptrs[0].cache;
        let diff = ptrs.iter().fold(0, |acc, p| acc | (p.cache ^ anchor));
        let common_bits = diff.leading_zeros();
        let common_bytes = (common_bits / 8) as usize;

        if common_bytes > 0 {
            // Determine how many bytes are safe to skip (must be non-zero).
            // We stop at the first zero byte because 0 might represent end-of-key padding,
            // and blindly skipping it effectively creates infinite recursion on short keys.
            let mut safe_bytes = 0;
            for i in 0..common_bytes {
                let shift = 56 - (i * 8);
                let byte = (anchor >> shift) as u8;
                if byte == 0 {
                    break;
                }
                safe_bytes += 1;
            }

            if safe_bytes > 0 {
                cp_len += safe_bytes;
                bytes_since_load += safe_bytes;

                if bytes_since_load >= 8 {
                    // Exhausted cache, reload from memory
                    update_caches(provider, ptrs, cp_len);
                    bytes_since_load = 0;
                } else {
                    // Shift caches to expose next bytes
                    let shift_bits = safe_bytes * 8;
                    ptrs.iter_mut().for_each(|p| p.cache <<= shift_bits);
                }
                continue;
            }
        }

        let mut counts = RadixCounts {
            data: [0; RADIX_BUCKETS],
        };
        let counts = &mut counts.data;

        // 1. Count frequencies via cache
        // Note: cache >> 56 extracts the most significant byte (big-endian prefix)
        ptrs.iter().for_each(|p| {
            let b = (p.cache >> 56) as u8;
            counts[b as usize] += 1;
        });

        // Optimization: Degenerate Check removed (Block Skip handles it).
        // Exceptions:
        // - Degenerate Zero: Handled by falling through to standard Radix logic (which puts all in bucket 0 and recurses with !is_degenerate).

        // 2. Compute offsets (prefix sum)
        let mut offsets = [0usize; RADIX_BUCKETS];
        let mut sum = 0;
        offsets
            .iter_mut()
            .zip(counts.iter())
            .for_each(|(offset, &count)| {
                *offset = sum;
                sum += count;
            });

        // 3. Permute using aux buffer
        let buffer = ptrs.to_vec();
        let mut cur_offsets = offsets;
        buffer.iter().for_each(|p| {
            let b = (p.cache >> 56) as u8;
            let pos = cur_offsets[b as usize];
            ptrs[pos] = *p;
            cur_offsets[b as usize] += 1;
        });

        // 4. Recurse on buckets
        let mut start = 0;
        let total_len = ptrs.len();
        counts.iter().for_each(|&count| {
            let end = start + count;
            if end > start {
                let bucket = &mut ptrs[start..end];
                let new_cp = cp_len + 1;

                update_caches(provider, bucket, new_cp);

                let is_degenerate = (end - start) == total_len;
                cps_quicksort(provider, bucket, new_cp, !is_degenerate);
            }
            start = end;
        });

        break; // Done
    }
}

/// Reloads caches for `SortPtr`s using the new common prefix length.
///
/// This ensures that the `cache` field of each `SortPtr` contains the next 8 bytes
/// of the key starting at `new_cp`.
fn update_caches<T: KeyAccessor + ?Sized>(provider: &T, ptrs: &mut [SortPtr], new_cp: usize) {
    // Always reload to ensure correctness with 0-padding ambiguities.
    ptrs.iter_mut().for_each(|p| {
        let k = provider.get_key(p.index);
        p.cache = load_u64_be(k, new_cp);
    });
}

/// Compares a sort pointer against a pivot.
///
/// 1. **Fast path**: Compares cached `u64` values.
/// 2. **Slow path**: If caches match, loads full keys from `provider` and compares byte-by-byte
///    starting from `offset + 8` (since the first 8 bytes are known equal).
/// 3. Handles "ambiguous zones" where one key ends exactly within the cached region.
#[inline(always)]
fn compare_entries<T: KeyAccessor + ?Sized>(
    provider: &T,
    a: &SortPtr,
    pivot: &SortPtr,
    offset: usize,
) -> Ordering {
    // Fast path
    if a.cache != pivot.cache {
        return a.cache.cmp(&pivot.cache);
    }

    // Slow path: resolve ambiguity or check beyond cache
    let key_a = provider.get_key(a.index);
    let key_p = provider.get_key(pivot.index);

    let start_safe = offset + 8;

    // Ambiguous zone check (short keys vs padding)
    if key_a.len() < start_safe || key_p.len() < start_safe {
        let slice_a = if offset < key_a.len() {
            &key_a[offset..]
        } else {
            &[]
        };
        let slice_p = if offset < key_p.len() {
            &key_p[offset..]
        } else {
            &[]
        };
        return match slice_a.cmp(slice_p) {
            Ordering::Equal => key_a.len().cmp(&key_p.len()),
            other => other,
        };
    }

    // Full comparison beyond cache
    let slice_a = &key_a[start_safe..];
    let slice_p = &key_p[start_safe..];

    match slice_a.cmp(slice_p) {
        Ordering::Equal => key_a.len().cmp(&key_p.len()),
        other => other,
    }
}

/// Fallback insertion sort for small partitions.
///
/// Efficient for very small arrays due to low overhead, despite O(N^2) complexity.
/// Used when the partition size drops below `INSERTION_SORT_THRESHOLD`.
fn insertion_sort<T: KeyAccessor + ?Sized>(provider: &T, ptrs: &mut [SortPtr], cp_len: usize) {
    let len = ptrs.len();
    (1..len).for_each(|i| {
        let mut j = i;
        while j > 0 {
            match compare_entries(provider, &ptrs[j], &ptrs[j - 1], cp_len) {
                Ordering::Less => {
                    ptrs.swap(j, j - 1);
                    j -= 1;
                }
                _ => break,
            }
        }
    });
}
