//! The action that formats the JSON files of a project
//!
//! The action wraps [prettier] as a subprocess. Prettier reads its own
//! configuration and formats the files that the action names. So a run agrees
//! with an editor and with a contributor that runs prettier bare. The action
//! declares the version of prettier that it runs. It also declares the range of
//! Node that prettier runs on. It starts that version, which [mise] installed,
//! on the Node that the project pins.
//!
//! Prettier formats more than JSON, and this action wraps the JSON
//! files alone, so that a check names the language that needs attention. The
//! actions for the other languages sit in crates of their own.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_format_json::FormatJson;
//!
//! let actions = Bundle::default().with(FormatJson);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [prettier]: https://prettier.io

mod format_json;

pub use self::format_json::{FormatJson, FormatJsonArgs, FormatJsonError};
