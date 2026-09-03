//! The action that formats the JSON files of a project
//!
//! The action wraps [prettier] as a subprocess: prettier reads its own
//! configuration and formats the files that the action names, so a run agrees
//! with an editor and with a contributor that runs prettier bare. The prettier
//! that runs is the one that [mise] installed for the project, at the version
//! that the project pinned.
//!
//! Prettier formats more than JSON, and this action wraps the JSON
//! files alone, so that a check names the language that needs attention. The
//! actions for the other languages sit in crates of their own.
//!
//! # Examples
//!
//! A harness mounts the action:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_format_json::FormatJson;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(FormatJson)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [prettier]: https://prettier.io

/// The action, its arguments, and the error that stops a run
pub mod format_json;

pub use self::format_json::{FormatJson, FormatJsonArgs, FormatJsonError};
