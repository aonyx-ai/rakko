#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod format_toml;

pub use self::format_toml::{FormatMode, FormatToml, FormatTomlArgs, FormatTomlError};
