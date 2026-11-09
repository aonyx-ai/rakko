//! The action that runs the tests of a project
//!
//! [Nextest] does the work: cargo builds every target of every package with
//! every feature, and nextest reads its own configuration and runs the tests.
//! So a run of the action agrees with a contributor that runs nextest bare. The
//! action declares the version of nextest that it runs. It starts that version,
//! which [mise] installed, with the cargo of the stable toolchain that the
//! project pins. It translates what nextest and cargo reported into the outcome
//! of the run.
//!
//! A run only reports, and it takes no argument. A test that failed becomes
//! a finding at the position where it panicked. A build that does not finish
//! becomes findings from the diagnostics of the compiler. The action skips
//! visibly in a project whose cargo discovers no workspace.
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

mod test_rust;

pub use self::test_rust::{TestRust, TestRustError};
