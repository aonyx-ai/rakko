#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod format_rust;

pub use self::format_rust::{FormatRust, FormatRustArgs, FormatRustError, Operation};
