//! Project maintenance as versioned Rust crates
//!
//! This crate is the entry point of Rakko. It is a placeholder: Rakko keeps
//! its features in separate crates, and this crate gets an API when there is
//! something to tie together.

/// Returns the sum of two unsigned 64-bit integers
///
/// This function is a placeholder. It exists so that the crate has an item to
/// build and to test.
///
/// # Panics
///
/// This function panics in a debug build when the sum is more than
/// [`u64::MAX`]. In a release build, the sum wraps around.
// rakko[impl placeholder.add]
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // rakko[verify placeholder.add]
    #[test]
    fn add_two_and_two_returns_four() {
        let result = add(2, 2);

        assert_eq!(result, 4);
    }
}
