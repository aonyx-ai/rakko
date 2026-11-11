#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod format_typescript;
mod observation;
mod oxfmt;
mod problem;

pub use self::format_typescript::{
    FormatTypeScript, FormatTypeScriptArgs, FormatTypeScriptError, Operation,
};
