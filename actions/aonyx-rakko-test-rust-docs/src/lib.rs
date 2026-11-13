#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod report;
mod test_rust_docs;

pub use self::report::ReadDoctestReportError;
pub use self::test_rust_docs::{TestRustDocs, TestRustDocsError};
