#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod issue;
mod lint_terraform;
mod tflint;

pub use self::lint_terraform::{LintTerraform, LintTerraformError};
