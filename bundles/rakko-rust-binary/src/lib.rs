#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use rakko_action::Bundle;

/// Returns the actions that a Rust project which ships a binary runs
///
/// The list is the one of the core, in the order in which `rakko-rust` lists
/// it, because a binary runs no check that a library does not run as well.
/// A check that only a binary runs joins the list here when there is one.
///
/// Each call builds a fresh list, because a mount takes ownership of every
/// action in it.
// rustbinary[impl actions]
#[must_use]
pub fn bundle() -> Bundle {
    rakko_rust::bundle()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // rustbinary[verify actions]
    #[test]
    fn bundle_exports_the_actions_of_the_core_and_no_other() {
        let names: Vec<String> = bundle()
            .actions()
            .iter()
            .map(|action| action.name().to_string())
            .collect();

        assert_eq!(
            names,
            [
                "build-internal-docs",
                "check-dependencies",
                "check-latest-deps",
                "check-unused-deps",
                "format-rust",
                "lint-rust",
                "test-rust",
            ]
        );
    }
}
