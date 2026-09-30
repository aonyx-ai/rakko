#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use rakko_action::Bundle;
use rakko_check_typescript::CheckTypeScript;
use rakko_format_typescript::FormatTypeScript;
use rakko_lint_typescript::LintTypeScript;
use rakko_test_typescript::TestTypeScript;

/// Returns the actions that any TypeScript project runs
///
/// The list is in alphabetical order. The order is for whoever reads the
/// list, because the command line that a harness builds decides for itself
/// how it lists the commands, and a run of one command starts no other.
///
/// Each call builds a fresh list, because a mount takes ownership of every
/// action in it.
// typescript[impl actions]
#[must_use]
pub fn bundle() -> Bundle {
    Bundle::new(vec![
        Box::new(CheckTypeScript),
        Box::new(FormatTypeScript),
        Box::new(LintTypeScript),
        Box::new(TestTypeScript),
    ])
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // typescript[verify actions]
    #[test]
    fn bundle_exports_the_actions_that_any_typescript_project_runs() {
        let names: Vec<String> = bundle()
            .actions()
            .iter()
            .map(|action| action.name().to_string())
            .collect();

        assert_eq!(
            names,
            [
                "check-typescript",
                "format-typescript",
                "lint-typescript",
                "test-typescript",
            ]
        );
    }
}
