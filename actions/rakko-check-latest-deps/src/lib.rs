#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod check_latest_deps;

pub use self::check_latest_deps::{CheckLatestDeps, CheckLatestDepsError};
