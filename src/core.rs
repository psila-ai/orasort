//! Core traits and types for Orasort.
//!
//! This module defines:
//! - [`KeyAccessor`]: The main trait users implement to sort their custom types.
//! - SortPtr: Internal pointer/cache structure.

use std::collections::VecDeque;

/// Size of the prefix to be cached in the sort pointer.
pub const SPLICE_PREFIX_SIZE: usize = 8;

/// Pointer to an item, storing index and cached 8-byte key prefix.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SortPtr {
    pub index: usize,
    pub cache: u64,
}

/// A trait for accessing key data from a collection without copying.
///
/// This trait allows `orasort` to sort any collection where elements can be
/// represented as byte slices (e.g., `Vec<String>`, `Vec<Vec<u8>>`, or custom types
/// like Arrow arrays).
///
/// # Examples
///
/// Implementing for a custom struct:
///
/// ```
/// use orasort::core::KeyAccessor;
///
/// struct MyCollection {
///     data: Vec<String>,
/// }
///
/// impl KeyAccessor for MyCollection {
///     fn get_key(&self, index: usize) -> &[u8] {
///         self.data[index].as_bytes()
///     }
///
///     fn len(&self) -> usize {
///         self.data.len()
///     }
/// }
/// ```
pub trait KeyAccessor {
    /// Returns a byte slice representing the key at the given index.
    fn get_key(&self, index: usize) -> &[u8];

    /// Returns the number of items in the collection.
    fn len(&self) -> usize;

    /// Returns `true` if the collection is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Optimized method to get the first 8 bytes of the key at `offset`.
    ///
    /// This allows implementors (like Arrow arrays) to provide a faster path than
    /// creating a temporary `&[u8]` slice and copying from it.
    ///
    /// Returns 0 if offset is out of bounds or for padding.
    #[inline(always)]
    fn get_u64_prefix(&self, index: usize, offset: usize) -> u64 {
        let key = self.get_key(index);
        let len = key.len();

        if offset >= len {
            return 0;
        }

        let remaining = len - offset;
        if remaining >= SPLICE_PREFIX_SIZE {
            unsafe {
                let ptr = key.as_ptr().add(offset);
                let raw = std::ptr::read_unaligned(ptr as *const u64);
                u64::from_be(raw)
            }
        } else {
            let mut buf = [0u8; SPLICE_PREFIX_SIZE];
            // Safety: Checked bounds above
            buf[..remaining].copy_from_slice(&key[offset..]);
            u64::from_be_bytes(buf)
        }
    }
}

// Blanket implementation for indexable slices of byte-ref types.
impl<T: AsRef<[u8]>> KeyAccessor for [T] {
    fn get_key(&self, index: usize) -> &[u8] {
        self[index].as_ref()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

// Explicit Vec impl to improve ergonomics (avoiding .as_slice()).
impl<T: AsRef<[u8]>> KeyAccessor for Vec<T> {
    fn get_key(&self, index: usize) -> &[u8] {
        self[index].as_ref()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

// Implementation for VecDeque.
// Provides O(1) random access, so it is suitable for Orasort.
impl<T: AsRef<[u8]>> KeyAccessor for VecDeque<T> {
    fn get_key(&self, index: usize) -> &[u8] {
        self[index].as_ref()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

// Implementation for sorting characters in a string (by byte).
// Note: This returns INDICES of bytes.
// Warning: Sorting UTF-8 bytes arbitrarily might produce invalid UTF-8 if reassembled blindly.
// But for searching/indexing it is valid.
impl KeyAccessor for str {
    fn get_key(&self, index: usize) -> &[u8] {
        std::slice::from_ref(&self.as_bytes()[index])
    }

    fn len(&self) -> usize {
        self.len()
    }
}

impl KeyAccessor for String {
    fn get_key(&self, index: usize) -> &[u8] {
        std::slice::from_ref(&self.as_bytes()[index])
    }

    fn len(&self) -> usize {
        self.len()
    }
}
