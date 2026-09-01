//! The action that runs the tests of a project
//!
//! [Nextest] does the work: cargo builds every target of every package with
//! every feature, and nextest reads its own configuration and runs the
//! tests, so a run of the action agrees with a contributor that runs nextest
//! bare. The action starts the cargo that [mise] installed for the project,
//! at the version that the project pinned, and translates what nextest and
//! cargo reported into the outcome of the run.
//!
//! A run only reports, and it takes no argument. A test that failed becomes
//! a finding at the position where it panicked, and a build that does not
//! finish becomes findings from the diagnostics of the compiler. The action
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
//! use rakko_test_rust::TestRust;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(TestRust)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [nextest]: https://nexte.st

/// Types for the action that runs the tests of a project
pub mod test_rust;

pub use self::test_rust::{NextestReport, Panic, TestFailure, TestRust, TestRustError};
