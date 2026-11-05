//! The action that examines the dependencies of a project
//!
//! A project depends on code that it did not write. [cargo-deny] answers three
//! questions about that code. The first is whether every package carries a
//! license that the project accepts. The second is whether the packages come
//! from a registry that the project trusts. The third is whether the graph
//! holds a shape that the project banned. Examples are two versions of one
//! package and a version requirement that accepts any future release. The
//! action starts
//! the cargo-deny that [mise] installed for the project, at the version that
//! the project pinned. It translates what cargo-deny reported into the outcome
//! of the run.
//!
//! Cargo-deny is its own program, and not a subcommand that cargo carries, so
//! the action starts it directly. Cargo answers a different question for the
//! same run: which workspaces make up the project. Cargo-deny checks one
//! workspace at a time, so a run asks cargo for the workspace roots and
//! checks each of them.
//!
//! Cargo-deny weighs everything that it reports with the level that the
//! configuration of the project gave the check. `deny` marks a shape that must
//! not appear. `warn` marks one that a maintainer wants to read about. `allow`
//! marks one that the project does not care about. An error therefore fails a
//! run. A warning does not. A passing run says how many warnings it read, so
//! the
//! middle level keeps the meaning that the project gave it.
//!
//! A finding names the workspace that the error came from and no path. The
//! place that cargo-deny underlines is a line of a lock file, or of a
//! manifest that lies in the registry cache of the machine. So a finding that
//! claimed a path would name a file that a reader cannot open.
//!
//! A run only reports, so the action takes no argument. The action applies to
//! a project that holds a manifest of cargo, and it skips visibly otherwise.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that,
//! and it panics without one.
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

pub mod check_dependencies;
pub mod deny;
pub mod problem;

pub use self::check_dependencies::{CheckDependencies, CheckDependenciesError};
pub use self::deny::{CheckWorkspaceError, Deny};
pub use self::problem::{DenyProblem, Package, Severity};
