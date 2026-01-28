//! Core traits and types for Orasort.
//!
//! This module defines:
//! - [`KeyAccessor`]: The main trait users implement to sort their custom types.
//! - SortPtr: Internal pointer/cache structure.

use std::collections::VecDeque;

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
