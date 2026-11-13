#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod check_minimal_deps;

pub use self::check_minimal_deps::{CheckMinimalDeps, CheckMinimalDepsError};
