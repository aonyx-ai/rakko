#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod format_shell;

pub use self::format_shell::{FormatMode, FormatShell, FormatShellArgs, FormatShellError};
