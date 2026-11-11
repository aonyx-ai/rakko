#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod lint_yaml;
mod observation;
mod problem;
mod yamllint;

pub use self::lint_yaml::{LintYaml, LintYamlError};
pub use self::yamllint::report::ReadReportError;
