//! The action that lints the Rust code of a project
//!
//! [Clippy] does the work: cargo reads the manifests of the project, selects
//! the lints that the project configured, and examines every target with
//! every feature, so a run of the action agrees with an editor and with a
//! contributor that runs clippy bare. The action starts the cargo that
//! [mise] installed for the project, at the version that the project
//! pinned, and translates what cargo reported into the outcome of the run.
//!
//! A run only reports, and it takes no argument. Every diagnostic that
//! clippy raises becomes a finding, whether the project warns about the lint
//! or denies it, so a run with a warning fails. The action applies to a
//! project that holds a manifest of cargo, and it skips visibly otherwise.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_lint_rust::LintRust;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintRust)];
//! ```
//!
//! [clippy]: https://doc.rust-lang.org/clippy/
//! [mise]: https://mise.jdx.dev

/// Types for the action that lints the Rust code of a project
pub mod lint_rust;

pub use self::lint_rust::{LintRust, LintRustError};
