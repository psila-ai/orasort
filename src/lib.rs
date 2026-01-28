//! # Orasort
//!
//! `orasort` is a high-performance, cache-efficient sorting library designed specifically for
//! strings, byte arrays, and other data types that share common prefixes.
//!
//! It implements the [**Orasort**](https://patents.google.com/patent/US7680791B2) algorithm, which combines the strengths of **Quicksort**
//! and **Radix Sort** while optimizing for modern CPU architectures by minimizing memory accesses
//! and maximizing cache locality.
//!
//! ## Key Features
//!
//! - **Cache Locality**: Stores an 8-byte prefix of the key directly in the sort pointer, allowing
//!   many comparisons to be resolved using only CPU registers without fetching the full data from memory.
//! - **Adaptive Strategy**: Dynamically switches between Quicksort (for small partitions) and
//!   Radix Sort (for large partitions) to maintain optimal performance across various distributions.
//! - **Zero-Copy abstractions**: The [`KeyAccessor`] trait allows sorting arbitrary data structures
//!   (e.g., Arrow arrays, `Vec<Vec<u8>>`) without copying the underlying data.
//! - **In-Place Mutation**: Provides [`orasort_mut`] for sorting `Vec`s in-place with minimal allocation.
//!
//! ## Usage
//!
//! ### Basic Usage
//!
//! For standard collections like `Vec<String>` or `Vec<Vec<u8>>`, you can use [`orasort`] (index-based)
//! or [`orasort_mut`] (in-place).
//!
//! ```rust
//! use orasort::orasort_mut;
//!
//! let mut data = vec!["banana", "apple", "cherry", "date"];
//! orasort_mut(&mut data);
//!
//! assert_eq!(data, vec!["apple", "banana", "cherry", "date"]);
//! ```
//!
//! ### Custom Types
//!
//! To sort custom types or complex data structures without creating intermediate strings,
//! implement the [`KeyAccessor`] trait.
//!
//! ```rust
//! use orasort::{orasort, KeyAccessor};
//!
//! struct User {
//!     username: String,
//! }
//!
//! // Wrapper struct to avoid orphan rule violation (impl foreign trait on foreign type).
//! struct Users(Vec<User>);
//!
//! impl KeyAccessor for Users {
//!     fn get_key(&self, index: usize) -> &[u8] {
//!         self.0[index].username.as_bytes()
//!     }
//!
//!     fn len(&self) -> usize {
//!         self.0.len()
//!     }
//! }
//!
//! let users = Users(vec![
//!     User { username: "Alice".to_string() },
//!     User { username: "Bob".to_string() },
//! ]);
//!
//! // Returns indices: [0, 1] (Alice, Bob)
//! let indices = orasort(&users);
//! ```
//!
//! ## Performance Characteristics
//!
//! - **Best Case**: O(N) when keys are distinct and distinguishable by their prefixes.
//! - **Worst Case**: O(N log N) similar to Quicksort, but heavily optimized for common prefix handling.
//! - **Memory Overhead**: Creates a temporary vector of pointers (`16 bytes` per item) to perform the sort.
//!
//! This library is particularly effective for datasets where cache misses are the primary bottleneck,
//! such as sorting large arrays of data.

pub mod algo;
pub mod core;
pub use algo::{orasort, orasort_mut};
pub use core::KeyAccessor;

pub mod prelude {
    pub use crate::algo::{orasort, orasort_mut};
    pub use crate::core::KeyAccessor;
}
