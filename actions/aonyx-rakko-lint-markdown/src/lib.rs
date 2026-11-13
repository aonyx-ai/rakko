#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod lint_markdown;
mod markdownlint;
mod observation;
mod problem;

pub use self::lint_markdown::{LintMarkdown, LintMarkdownError};
