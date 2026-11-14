#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use aonyx_rakko_format_terraform::FormatTerraform;
use aonyx_rakko_lint_terraform::LintTerraform;
use rakko::Bundle;

/// Returns the actions that any project with Terraform runs
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
/// use aonyx_rakko_terraform::bundle;
///
/// let actions = bundle();
/// assert!(!actions.actions().is_empty());
/// ```
#[must_use]
pub fn bundle() -> Bundle {
    Bundle::new(vec![Box::new(FormatTerraform), Box::new(LintTerraform)])
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn bundle_exports_the_actions_that_any_project_with_terraform_runs() {
        let names: Vec<String> = bundle()
            .actions()
            .iter()
            .map(|action| action.name().to_string())
            .collect();

        assert_eq!(names, ["format-terraform", "lint-terraform"]);
    }
}
