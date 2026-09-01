//! The action that formats the Rust code of a project
//!
//! [Rustfmt] does the work: cargo reads the manifests of the project, and
//! rustfmt reads its own configuration and formats every target, so a run
//! of the action agrees with an editor and with a contributor that runs
//! rustfmt bare. The action starts the cargo that [mise] installed for the
//! project, on the nightly toolchain that the project pinned, and translates
//! what rustfmt reported into the outcome of the run.
//!
//! A run reports by default and rewrites with the `fix` argument. The action
//! applies to a project that holds a manifest of cargo, and it skips visibly
//! otherwise.
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

/// Types for the action that formats the Rust code of a project
pub mod format_rust;

pub use self::format_rust::{
    FormatRust, FormatRustArgs, FormatRustError, RustfmtProblem, RustfmtProblemDetail,
    RustfmtReport,
};
