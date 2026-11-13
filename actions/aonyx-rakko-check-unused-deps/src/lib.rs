#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod check_unused_deps;
mod report;
mod udeps;

pub use self::check_unused_deps::{CheckUnusedDeps, CheckUnusedDepsError};
pub use self::report::ReadUdepsReportError;
