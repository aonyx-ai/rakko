#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod check_dependencies;
mod deny;
mod problem;

pub use self::check_dependencies::{CheckDependencies, CheckDependenciesError};
