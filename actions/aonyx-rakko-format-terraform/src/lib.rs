#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod format_terraform;
mod terraform;

pub use self::format_terraform::{
    FormatMode, FormatTerraform, FormatTerraformArgs, FormatTerraformError,
};
