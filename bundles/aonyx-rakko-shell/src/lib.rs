#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use aonyx_rakko_format_shell::FormatShell;
use aonyx_rakko_lint_shell::LintShell;
use rakko::Bundle;

/// Returns the actions that any project with shell scripts runs
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
/// use aonyx_rakko_shell::bundle;
///
/// let actions = bundle();
/// assert!(!actions.actions().is_empty());
/// ```
#[must_use]
pub fn bundle() -> Bundle {
    Bundle::new(vec![Box::new(FormatShell), Box::new(LintShell)])
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn bundle_exports_the_actions_that_any_project_with_shell_scripts_runs() {
        let names: Vec<String> = bundle()
            .actions()
            .iter()
            .map(|action| action.name().to_string())
            .collect();

        assert_eq!(names, ["format-shell", "lint-shell"]);
    }
}
