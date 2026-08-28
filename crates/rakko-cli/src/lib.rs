//! The command-line projection of the actions that a harness mounts
//!
//! A harness is the small binary that a project runs to maintain itself. It
//! mounts the actions that the project uses, and this crate turns them into a
//! command-line interface: one command for each action, the help text of each
//! command, and the flags that every command shares.
//!
//! Only a harness depends on this crate. An action depends on the contract
//! crate alone, so the command-line framework stays out of the build of an
//! action.
//!
//! This crate is a placeholder. It gets an API when there is a mount to
//! project.

/// Returns the sum of two unsigned 64-bit integers
///
/// This function is a placeholder. It exists so that the crate has an item to
/// build and to test.
///
/// # Panics
///
/// This function panics in a debug build when the sum is more than
/// [`u64::MAX`]. In a release build, the sum wraps around.
// cli[impl placeholder.add]
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // cli[verify placeholder.add]
    #[test]
    fn add_two_and_two_returns_four() {
        let result = add(2, 2);

        assert_eq!(result, 4);
    }
}
