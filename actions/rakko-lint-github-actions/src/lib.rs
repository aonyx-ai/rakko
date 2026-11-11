#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod lint_github_actions;
mod observation;
mod problem;
mod zizmor;

pub use self::lint_github_actions::{LintGitHubActions, LintGitHubActionsError};
