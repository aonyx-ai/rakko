//! The action that formats the Rust code of a project
//!
//! [Rustfmt] does the work. Cargo reads the manifests of the project, and
//! rustfmt reads its own configuration and formats every target. So a run of
//! the action agrees with an editor and with a contributor that runs rustfmt
//! bare. The action starts the cargo that [mise] installed for the project on
//! the nightly toolchain that the project pinned. It translates what rustfmt
//! reported into the outcome of the run.
//!
//! A run reports by default and rewrites with the `fix` argument. The action
//! skips visibly in a project whose cargo discovers no workspace.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_format_rust::FormatRust;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(FormatRust)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [rustfmt]: https://github.com/rust-lang/rustfmt

pub mod format_rust;

pub use self::format_rust::{
    Exit, FormatRust, FormatRustArgs, FormatRustError, Operation, RustfmtProblem,
    RustfmtProblemDetail, RustfmtReport,
};
