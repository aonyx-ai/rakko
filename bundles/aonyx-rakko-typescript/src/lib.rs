#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use aonyx_rakko_check_typescript::CheckTypeScript;
use aonyx_rakko_format_typescript::FormatTypeScript;
use aonyx_rakko_lint_typescript::LintTypeScript;
use aonyx_rakko_test_typescript::TestTypeScript;
use rakko::Bundle;

/// Returns the actions that any TypeScript project runs
///
/// The list is in alphabetical order. The order is for whoever reads the list,
/// because the command line that a harness builds decides for itself how it
/// lists the commands. A run of one command starts no other.
///
/// Each call builds a fresh list, because a mount takes ownership of every
/// action in it.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_typescript::bundle;
///
/// let actions = bundle();
/// assert!(!actions.actions().is_empty());
/// ```
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

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
