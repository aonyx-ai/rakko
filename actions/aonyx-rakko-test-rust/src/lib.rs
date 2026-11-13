#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod test_rust;

pub use self::test_rust::{TestRust, TestRustError};
