#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use rakko_action::Bundle;
use rakko_build_internal_docs::BuildInternalDocs;
use rakko_check_dependencies::CheckDependencies;
use rakko_check_latest_deps::CheckLatestDeps;
use rakko_check_unused_deps::CheckUnusedDeps;
use rakko_format_rust::FormatRust;
use rakko_lint_rust::LintRust;
use rakko_test_rust::TestRust;

/// Returns the actions that any Rust project runs
///
/// The list is in alphabetical order. The order is for whoever reads the
/// list, because the command line that a harness builds decides for itself
/// how it lists the commands, and a run of one command starts no other.
///
/// Each call builds a fresh list, because a mount takes ownership of every
/// action in it.
// rust[impl actions]
#[must_use]
pub fn bundle() -> Bundle {
    Bundle::new(vec![
        Box::new(BuildInternalDocs),
        Box::new(CheckDependencies),
        Box::new(CheckLatestDeps),
        Box::new(CheckUnusedDeps),
        Box::new(FormatRust),
        Box::new(LintRust),
        Box::new(TestRust),
    ])
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // rust[verify actions]
    #[test]
    fn bundle_exports_the_actions_that_any_rust_project_runs() {
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
