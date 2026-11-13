#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use rakko::Bundle;

/// Returns the actions that a Rust project which ships a binary runs
///
/// The list is the one of the core, in the order in which `aonyx-rakko-rust`
/// lists it. It is the same list because a binary runs no check that a library
/// does not run as well. A check that only a binary runs joins the list here when
/// there is one.
///
/// Each call builds a fresh list, because a mount takes ownership of every
/// action in it.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_rust_binary::bundle;
///
/// let names: Vec<String> = bundle()
///     .actions()
///     .iter()
///     .map(|action| action.name().to_string())
///     .collect();
///
/// assert!(names.contains(&"test-rust".to_owned()));
/// assert!(!names.contains(&"lint-yaml".to_owned()));
/// ```
#[must_use]
pub fn bundle() -> Bundle {
    aonyx_rakko_rust::bundle()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

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
                "lint-stiles",
                "test-rust",
            ]
        );
    }
}
