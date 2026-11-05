//! The action that lints the TypeScript of a project
//!
//! [Oxlint] does the work: it discovers the files, reads its own configuration,
//! and applies the rules that the project turned on. So a run of the action
//! agrees with an editor and with a contributor that runs oxlint bare. The
//! action starts the oxlint that [mise] installed for the project, at the
//! version that the project pinned. It translates what oxlint reported into the
//! outcome of the run.
//!
//! A run only reports, so the action takes no argument. The action applies to
//! a project that holds files which oxlint lints, and it skips visibly
//! otherwise.
//!
//! Oxlint gives each diagnostic a severity, and the configuration of the
//! project decides that severity for each rule. This crate reports a warning
//! and an error alike. A run with either fails, because a rule at either
//! severity is one that the project asked oxlint to look for.
//!
//! One run of oxlint answers everything that the action needs. The report
//! carries the diagnostics and the number of files that the run examined. So
//! the project is discovered once, by the tool that the contributor also runs.
//!
//! The report is also the only thing that answers. Oxlint ends with success
//! when every rule that a file broke is one that the project weighs as a
//! warning. It ends without success for a project that holds no file to lint.
//! So the status of the process would pass a project with problems and fail a
//! project with none. A report that this crate cannot read stops the run
//! instead of passing quietly.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that, and
//! it panics without one.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_lint_typescript::LintTypeScript;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintTypeScript)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [oxlint]: https://oxc.rs/docs/guide/usage/linter.html
//! [tokio]: https://tokio.rs

pub mod lint_typescript;
pub mod observation;
pub mod oxlint;
pub mod problem;

pub use self::lint_typescript::{LintTypeScript, LintTypeScriptError};
pub use self::observation::Observation;
pub use self::oxlint::report::ReadReportError;
pub use self::oxlint::{ObserveOxlintError, Oxlint};
pub use self::problem::{OxlintProblem, Place, Severity};
