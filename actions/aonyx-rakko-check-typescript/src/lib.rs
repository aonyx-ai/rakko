#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod check_typescript;
mod diagnostic;
mod tsc;

pub use self::check_typescript::{CheckTypeScript, CheckTypeScriptError};
pub use self::tsc::report::ReadReportError;
