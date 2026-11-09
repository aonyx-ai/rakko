#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use rakko::Bundle;
use rakko_check_mise_lock::CheckMiseLock;
use rakko_check_renovate_config::CheckRenovateConfig;
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
/// use rakko_baseline::bundle;
///
/// let names: Vec<String> = bundle()
///     .actions()
///     .iter()
///     .map(|action| action.name().to_string())
///     .collect();
///
/// assert_eq!(names.first().map(String::as_str), Some("check-mise-lock"));
/// assert!(names.contains(&"lint-yaml".to_owned()));
/// ```
#[must_use]
pub fn bundle() -> Bundle {
    Bundle::new(vec![
        Box::new(CheckMiseLock),
        Box::new(CheckRenovateConfig),
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

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
                "check-mise-lock",
                "check-renovate-config",
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
