//! The action that validates the TOML files of a project
//!
//! [Taplo] does the work: it discovers the TOML files, reads its own
//! configuration, and validates what that configuration selects, so a run of
//! the action agrees with an editor and with a contributor that runs taplo
//! bare. The action starts the taplo that [mise] installed for the project,
//! at the version that the project pinned, and translates what taplo
//! reported into the outcome of the run.
//!
//! A run only reports. Taplo repairs nothing that a validation finds, so the
//! action takes no argument. It applies to a project that holds TOML files,
//! and it skips visibly otherwise.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_lint_toml::LintToml;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintToml)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [taplo]: https://taplo.tamasfe.dev

/// Types for the action that validates the TOML files of a project
pub mod lint_toml;

pub use self::lint_toml::{LintToml, LintTomlError};
