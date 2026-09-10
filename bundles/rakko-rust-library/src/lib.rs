#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use rakko_action::Bundle;
use rakko_check_minimal_deps::CheckMinimalDeps;
use rakko_check_msrv::CheckMsrv;
use rakko_test_rust_docs::TestRustDocs;

/// Returns the actions that a Rust project which publishes a library runs
///
/// The list starts with the actions of the core, in the order in which
/// `rakko-rust` lists them, and the three checks of this bundle follow in
/// alphabetical order. The order is for whoever reads the list, because the
/// command line that a harness builds decides for itself how it lists the
/// commands, and a run of one command starts no other.
///
/// Each call builds a fresh list, because a mount takes ownership of every
/// action in it.
// rustlibrary[impl actions]
#[must_use]
pub fn bundle() -> Bundle {
    let mut actions = rakko_rust::bundle();

    actions.push(Box::new(CheckMinimalDeps));
    actions.push(Box::new(CheckMsrv));
    actions.push(Box::new(TestRustDocs));

    actions
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // rustlibrary[verify actions]
    #[test]
    fn bundle_exports_the_core_and_the_checks_of_a_library() {
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
                "check-minimal-deps",
                "check-msrv",
                "test-rust-docs",
            ]
        );
    }
}
