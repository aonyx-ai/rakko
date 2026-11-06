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
//! skips visibly in a project whose cargo discovers no workspace.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_test_rust::TestRust;
//!
//! let actions = Bundle::default().with(TestRust);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [nextest]: https://nexte.st

/// Types for the action that runs the tests of a project
pub mod test_rust;

pub use self::test_rust::{TestRust, TestRustError};
