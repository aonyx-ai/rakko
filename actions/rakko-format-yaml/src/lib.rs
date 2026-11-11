#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod format_yaml;

pub use self::format_yaml::{FormatYaml, FormatYamlArgs, FormatYamlError};
