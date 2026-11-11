#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod lint_stiles;
mod observation;
mod problem;
mod stiles;

pub use self::lint_stiles::{LintStiles, LintStilesError};
