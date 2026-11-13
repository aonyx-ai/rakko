#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod lint_toml;

pub use self::lint_toml::{LintToml, LintTomlError};
