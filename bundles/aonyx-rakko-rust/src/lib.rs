#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use aonyx_rakko_build_internal_docs::BuildInternalDocs;
use aonyx_rakko_check_crate_owners::{CheckCrateOwners, CrateOwner};
use aonyx_rakko_check_dependencies::CheckDependencies;
use aonyx_rakko_check_latest_deps::CheckLatestDeps;
use aonyx_rakko_check_unused_deps::CheckUnusedDeps;
use aonyx_rakko_format_rust::FormatRust;
use aonyx_rakko_lint_rust::LintRust;
use aonyx_rakko_lint_stiles::LintStiles;
use aonyx_rakko_test_rust::TestRust;
use rakko::Bundle;

/// Returns the actions that any Rust project runs
///
/// The list is in alphabetical order. The order is for whoever reads the list,
/// because the command line that a harness builds decides for itself how it
/// lists the commands. A run of one command starts no other.
///
/// Each call builds a fresh list, because a mount takes ownership of every
/// action in it.
///
/// `check-crate-owners` checks that each crate of the project has the owners
/// of Aonyx on crates.io: the users `jdno` and `martijnarts`, and the team
/// `github:aonyx-ai:crates-io`. Every project that mounts the bundle checks
/// the same owners.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_rust::bundle;
///
/// let actions = bundle();
/// assert!(!actions.actions().is_empty());
/// ```
#[must_use]
pub fn bundle() -> Bundle {
    Bundle::new(vec![
        Box::new(BuildInternalDocs),
        Box::new(CheckCrateOwners::new(owners())),
        Box::new(CheckDependencies),
        Box::new(CheckLatestDeps),
        Box::new(CheckUnusedDeps),
        Box::new(FormatRust),
        Box::new(LintRust),
        Box::new(LintStiles),
        Box::new(TestRust),
    ])
}

/// Returns the owners that every crate of Aonyx must have on crates.io
///
/// Only a user owner of a crate can change its owners or set up trusted
/// publishing. So two users keep the crate in reach when one account becomes
/// unavailable. The team shows on crates.io that the crate belongs to the
/// organization.
fn owners() -> [CrateOwner; 3] {
    [
        CrateOwner::new("jdno"),
        CrateOwner::new("martijnarts"),
        CrateOwner::new("github:aonyx-ai:crates-io"),
    ]
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

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
                "check-crate-owners",
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
