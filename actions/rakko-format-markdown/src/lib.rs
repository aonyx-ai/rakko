#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod format_markdown;

pub use self::format_markdown::{FormatMarkdown, FormatMarkdownArgs, FormatMarkdownError};
