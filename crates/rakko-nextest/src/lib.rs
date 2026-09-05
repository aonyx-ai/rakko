//! The machinery that the actions which run the tests of a project share
//!
//! [Nextest] is a plugin of cargo, and every action that runs it asks the
//! same questions: which command does a run write, what did nextest and cargo
//! report, and can the caller answer from what they wrote? This crate answers
//! all three, so that an action names the workspace that it wants tested and
//! reads the answer as data.
//!
//! A run tests one workspace root, because cargo works on one workspace at a
//! time. The caller resolves cargo for the project and discovers the roots,
//! and it runs the crate once per root. Cargo builds every target of every
//! package with every feature, and nextest reads its own configuration and
//! runs the tests, so a run agrees with the terminal of a contributor that
//! runs nextest bare.
//!
//! The caller also decides what a run does with the lockfile of the
//! workspace. An action that tests the project as it stands lets cargo
//! resolve the dependencies of the build. An action that first resolved the
//! dependencies itself holds cargo to that resolution instead, so that a
//! version which nobody chose cannot join the build unannounced.
//!
//! Nextest reports the tests as JSON, and cargo reports the diagnostics of
//! the build as JSON as well. The shape of the two reports belongs to a
//! version of the tools, so keeping the reading in one place keeps the
//! version surface in one place as well: a nextest that reports differently
//! breaks one crate, and not one crate per action.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that,
//! and a run panics without one.
//!
//! # Examples
//!
//! An action resolves cargo, discovers the workspaces, and tests each of
//! them:
//!
//! ```no_run
//! use rakko_action::ProjectRoot;
//! use rakko_cargo::Cargo;
//! use rakko_nextest::{Lockfile, Nextest};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let root = ProjectRoot::new("/home/otter/project".into());
//! let cargo = Cargo::resolve(root.clone()).await?;
//! let roots = cargo.roots().await?;
//! let nextest = Nextest::new(cargo, Lockfile::Writable);
//!
//! for workspace in &roots {
//!     let observation = nextest.observe(workspace, &root).await?;
//!
//!     println!("{} tests, {} findings", observation.ran(), observation.findings().len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [nextest]: https://nexte.st
//! [tokio]: https://tokio.rs

/// Types for the nextest that a project runs
pub mod nextest;
/// Types for what one run of nextest produced
pub mod observation;
/// Types for what nextest reported about a run
pub mod report;

pub use self::nextest::{Lockfile, Nextest, ObserveNextestError};
pub use self::observation::Observation;
pub use self::report::{NextestReport, Panic, ReadNextestReportError, TestFailure};
