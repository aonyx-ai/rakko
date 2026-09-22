//! The action that runs the tests of a Node project
//!
//! The [test runner] that Node carries does the work: it walks the project for
//! the files that hold tests, runs each of them in a process of its own, and
//! reports what every test did. The action starts the node that [mise]
//! installed for the project, at the version that the project pinned, and
//! translates what the runner reported into the outcome of the run.
//!
//! The runner needs nothing from a project. A test file imports `node:test`,
//! and the project adds no dependency, no configuration file, and no script,
//! which is what makes this worth mounting for a project that would otherwise
//! run no tests at all. Node also strips the types of a TypeScript file on the
//! way in, so a project that writes its tests in TypeScript needs no build
//! before the run.
//!
//! Node decides which files hold tests, with the patterns that it carries, so
//! a run of the action covers the tests of a bare run and no others. A run
//! only reports, so the action takes no argument.
//!
//! The report is the answer, and the status of the process decides only
//! whether a run without a failure is trusted. The runner reports every test
//! that failed, so a run that reported none and ended without success stopped
//! for a reason of its own, and it stops the action instead of passing.
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
//! use rakko_test_typescript::TestTypeScript;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(TestTypeScript)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [test runner]: https://nodejs.org/api/test.html
//! [tokio]: https://tokio.rs

/// Types for one test that failed
pub mod failure;
/// Types for the node that a project runs
pub mod node;
/// Types for the action that runs the tests of a Node project
pub mod test_typescript;

pub use self::failure::{Failure, Origin};
pub use self::node::report::{ReadReportError, Report};
pub use self::node::{Node, ObserveNodeError};
pub use self::test_typescript::{TestTypeScript, TestTypeScriptError};
