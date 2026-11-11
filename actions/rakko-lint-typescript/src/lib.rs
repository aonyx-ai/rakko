#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod lint_typescript;
mod observation;
mod oxlint;
mod problem;

pub use self::lint_typescript::{LintTypeScript, LintTypeScriptError};
pub use self::oxlint::report::ReadReportError;
