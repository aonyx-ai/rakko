//! The machinery that the actions which run the tests of a project share
//!
//! [Nextest] is a plugin of cargo, and every action that runs it asks the same
//! questions. Which command does a run write? What did nextest and cargo
//! report? Can the caller answer from what they wrote? This crate answers all
//! three, so that an action names the workspace that it wants tested and reads
//! the answer as data.
//!
//! The crate pins the version of nextest that every such action runs, in
//! the file `mise.rakko.toml` that the build embeds. An action declares that
//! version with [`Nextest::requirement`], and the version that the project
//! pins does not change what runs.
//!
//! A run tests one workspace root, because cargo works on one workspace at a
//! time. The caller resolves cargo for the project and discovers the roots, and
//! it runs the crate once per root. Cargo builds every target of every package
//! with every feature. Nextest reads its own configuration and runs the tests.
//! So a run agrees with the terminal of a contributor that runs nextest bare.
//!
//! The caller also decides what a run does with the lockfile of the
//! workspace. An action that only examines the project, whether it tests the
//! committed resolution or one that it resolved itself first, holds cargo to
//! the lockfile. Cargo then refuses a build that would have to write it, so a
//! version that nobody chose cannot join the build unannounced. A run that
//! may change the project can let cargo resolve the dependencies and write
//! the lockfile instead.
//!
//! Nextest reports the tests as JSON, and cargo reports the diagnostics of the
//! build as JSON as well. The shape of the two reports belongs to a version of
//! the tools. So keeping the reading in one place keeps the version surface in
//! one place as well. A nextest that reports differently breaks one crate, and
//! not one crate per action.
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
//! use rakko::{Context, ProjectRoot};
//! use aonyx_rakko_cargo::Cargo;
//! use aonyx_rakko_nextest::{Lockfile, Nextest};
//! use rakko_test_utils::path;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
//! let cargo = Cargo::resolve(&context).await?;
//! let roots = cargo.roots().await?;
//! let nextest = Nextest::resolve()
//!     .cargo(cargo)
//!     .lockfile(Lockfile::Locked)
//!     .context(&context)
//!     .call()
//!     .await?;
//!
//! for workspace in &roots {
//!     let observation = nextest.observe(workspace, context.root()).await?;
//!
//!     assert!(observation.findings().is_empty());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [nextest]: https://nexte.st
//! [tokio]: https://tokio.rs

mod nextest;
mod observation;
mod report;

pub use self::nextest::{Lockfile, Nextest, ObserveNextestError};
pub use self::observation::Observation;
pub use self::report::ReadNextestReportError;
