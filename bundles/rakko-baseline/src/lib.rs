#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use rakko_action::Bundle;
use rakko_format_json::FormatJson;
use rakko_format_markdown::FormatMarkdown;
use rakko_format_toml::FormatToml;
use rakko_format_yaml::FormatYaml;
use rakko_lint_github_actions::LintGitHubActions;
use rakko_lint_markdown::LintMarkdown;
use rakko_lint_toml::LintToml;
use rakko_lint_yaml::LintYaml;

/// Returns the actions that any project runs
///
/// The list holds the formatters before the linters, and each group in
/// alphabetical order. The order is for whoever reads the list, because the
/// command line that a harness builds decides for itself how it lists the
/// commands.
///
/// Each call builds a fresh list, because a mount takes ownership of every
/// action in it.
// baseline[impl actions]
#[must_use]
pub fn bundle() -> Bundle {
    Bundle::new(vec![
        Box::new(FormatJson),
        Box::new(FormatMarkdown),
        Box::new(FormatToml),
        Box::new(FormatYaml),
        Box::new(LintGitHubActions),
        Box::new(LintMarkdown),
        Box::new(LintToml),
        Box::new(LintYaml),
    ])
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // baseline[verify actions]
    #[test]
    fn bundle_exports_the_actions_that_any_project_runs() {
        let names: Vec<String> = bundle()
            .actions()
            .iter()
            .map(|action| action.name().to_string())
            .collect();

        assert_eq!(
            names,
            [
                "format-json",
                "format-markdown",
                "format-toml",
                "format-yaml",
                "lint-github-actions",
                "lint-markdown",
                "lint-toml",
                "lint-yaml",
            ]
        );
    }
}
