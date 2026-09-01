//! The machinery that the actions which wrap cargo share
//!
//! [Cargo] does several jobs of project maintenance, and one action wraps
//! each of them: rustfmt formats the Rust files, clippy lints them, and
//! nextest runs the tests. Every one of those actions asks the same
//! questions: does this project hold Rust code at all, which cargo does it
//! run, which workspaces make up the project, which toolchain does the job
//! need, and what did cargo report about the build? This crate answers all
//! of them, so that an action writes the command line of its job and reads
//! the answer as data.
//!
//! A project can hold more than one workspace. The harness of a project is a
//! package of its own, outside the workspace of the crates that it maintains,
//! and cargo works on one workspace at a time. The crate therefore finds
//! every workspace root under the project, and an action runs its job at
//! each of them.
//!
//! Cargo reports the diagnostics of a build as JSON, and this crate reads
//! that JSON. The shape belongs to a version of cargo, so keeping the reading
//! in one place keeps the version surface in one place as well.
//!
//! # Asynchronous Runtime
//!
//! The look at a project reads directories, and a run starts a program and
//! waits for it. A [Tokio] runtime drives both, and they panic without one.
//!
//! # Examples
//!
//! An action looks at the project, resolves cargo, and runs one job at every
//! root:
//!
//! ```no_run
//! use rakko_action::ProjectRoot;
//! use rakko_cargo::{Cargo, CargoReport};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let root = ProjectRoot::new("/home/otter/project".into());
//!
//! if Cargo::applies(&root).await {
//!     let cargo = Cargo::resolve(root).await?;
//!
//!     for workspace in cargo.roots().await? {
//!         let execution = cargo
//!             .invocation(&workspace)
//!             .args(["clippy", "--message-format=json"])
//!             .run()
//!             .await?;
//!         let report = CargoReport::read(&execution.stdout().to_string_lossy());
//!
//!         println!("{} diagnostics", report.diagnostics().len());
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [cargo]: https://doc.rust-lang.org/cargo/
//! [tokio]: https://tokio.rs

/// Types for the cargo that a project runs
pub mod cargo;
/// Types for what cargo reported about a build
pub mod report;
/// Types for one workspace root of a project
pub mod root;
/// Types for the Rust toolchain that a job runs on
pub mod toolchain;

pub use self::cargo::{Cargo, DiscoverRootsError};
pub use self::report::{CargoDiagnostic, CargoReport, DiagnosticLevel, DiagnosticSpan};
pub use self::root::CargoRoot;
pub use self::toolchain::{Channel, ResolveToolchainError, Toolchain};
