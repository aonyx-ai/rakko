#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod comment;
mod lint_shell;
mod shellcheck;

pub use self::lint_shell::{LintShell, LintShellError};
