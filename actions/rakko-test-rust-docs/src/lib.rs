//! The action that runs the examples in the documentation of a project
//!
//! An example in a doc comment is a test. The compiler builds it against the
//! library that documents it, and the test harness runs it, so an example
//! that names a function which no longer exists fails like any other test.
//! It is also the part of the documentation that rots most quietly, because
//! nothing else in a project names the symbols that an example uses.
//!
//! The action exists beside the action that runs the tests, because
//! [nextest] does not run the examples and says nothing about leaving them
//! out. Cargo does the work here: it builds the examples of every package of
//! a workspace with every feature and runs them, so a run agrees with a
//! contributor that runs `cargo test --doc` bare. The cargo that runs is the
//! one that [mise] installed for the project, at the version that the
//! project pinned.
//!
//! A run only reports, and it takes no argument. An example that failed
//! becomes a finding at the line that documents it, and a build that does
//! not finish becomes findings from the diagnostics of the compiler. The
//! action skips visibly in a project whose workspaces hold no library,
//! because cargo can test the documentation of a library and of nothing
//! else.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_test_rust_docs::TestRustDocs;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(TestRustDocs)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [nextest]: https://nexte.st

/// Types for what the test harness reported about the examples of a
/// workspace
pub mod report;
/// Types for the action that runs the examples of a project
pub mod test_rust_docs;

pub use self::report::{DoctestFailure, DoctestReport, ReadDoctestReportError};
pub use self::test_rust_docs::{TestRustDocs, TestRustDocsError};
