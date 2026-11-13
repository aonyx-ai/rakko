#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod lint_rust;

pub use self::lint_rust::{LintRust, LintRustError};
