#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod format_json;

pub use self::format_json::{FormatJson, FormatJsonArgs, FormatJsonError};
