//! The action that examines the dependencies of a project
//!
//! A project depends on code that it did not write, and [cargo-deny] answers
//! three questions about that code: whether every package carries a license
//! that the project accepts, whether the packages come from a registry that
//! the project trusts, and whether the graph holds a shape that the project
//! banned, such as two versions of one package or a version requirement that
//! accepts any future release. The action starts the cargo-deny that [mise]
//! installed for the project, at the version that the project pinned, and
//! translates what cargo-deny reported into the outcome of the run.
//!
//! Cargo-deny is its own program, and not a subcommand that cargo carries, so
//! the action starts it directly. Cargo answers a different question for the
//! same run: which workspaces make up the project. Cargo-deny checks one
//! workspace at a time, so a run asks cargo for the workspace roots and
//! checks each of them.
//!
//! Cargo-deny weighs everything that it reports with the level that the
//! configuration of the project gave the check: `deny` for a shape that must
//! not appear, `warn` for one that a maintainer wants to read about, and
//! `allow` for one that the project does not care about. An error therefore
//! fails a run, a warning does not, and a passing run says how many warnings
//! it read, so the middle level keeps the meaning that the project gave it.
//!
//! A finding names the workspace that the error came from and no path. The
//! place that cargo-deny underlines is a line of a lock file, or of a
//! manifest that lies in the registry cache of the machine, so a finding that
//! claimed a path would name a file that a reader cannot open.
//!
//! A run only reports, so the action takes no argument. The action applies to
//! a project that holds a manifest of cargo, and it skips visibly otherwise.
//!
//! # Asynchronous Runtime
//!
//! The look at a project reads directories, and a run starts a program and
//! waits for it. A [Tokio] runtime drives both, and they panic without one.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_check_dependencies::CheckDependencies;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckDependencies)];
//! ```
//!
//! [cargo-deny]: https://embarkstudios.github.io/cargo-deny/
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs

/// Types for the action that examines the dependencies of a project
pub mod check_dependencies;
/// Types for the cargo-deny that a project runs
pub mod deny;
/// Types for one thing that cargo-deny reported about a workspace
pub mod problem;

pub use self::check_dependencies::{CheckDependencies, CheckDependenciesError};
pub use self::deny::{CheckWorkspaceError, Deny};
pub use self::problem::{DenyProblem, Package, Severity};
