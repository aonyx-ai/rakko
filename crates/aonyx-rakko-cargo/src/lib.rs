//! The machinery that the actions which wrap cargo share
//!
//! [Cargo] does several jobs of project maintenance, and one action wraps each
//! of them. Rustfmt formats the Rust files, clippy lints them, and nextest
//! runs the tests. Every one of those actions asks the same questions. Does
//! this project hold Rust code at all? Which cargo does it run? Which
//! workspaces make up the project? What does each workspace hold? Which
//! toolchain does the job need? What did cargo report about the build? This
//! crate answers all of them, so that an action writes the
//! command line of its job and reads the answer as data.
//!
//! A project can hold more than one workspace. The harness of a project is a
//! package of its own, outside the workspace of the crates that it maintains.
//! Cargo works on one workspace at a time. The crate therefore finds every
//! workspace root under the project, and an action runs its job at each of
//! them.
//!
//! A Rust toolchain is a decision of the project, so the project pins it and
//! an action states the range of toolchains that it accepts.
//! [`Cargo::requirement`] gives the range of the stable toolchain, and
//! [`Toolchain::nightly`] the range of the nightly. An action declares them
//! among its tools, and the crate resolves the toolchain that the project
//! pins in each range.
//!
//! Cargo reports the diagnostics of a build as JSON, and this crate reads
//! that JSON. The shape belongs to a version of cargo, so keeping the reading
//! in one place keeps the version surface in one place as well.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that,
//! and it panics without one.
//!
//! # Examples
//!
//! An action looks at the project, resolves cargo, and runs one job at every
//! root:
//!
//! ```no_run
//! use rakko::Context;
//! use aonyx_rakko_cargo::{Cargo, CargoReport};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let context = Context::builder().root("/home/otter/project").build();
//!
//! let cargo = Cargo::resolve(&context).await?;
//!
//! for workspace in cargo.roots().await? {
//!     let execution = cargo
//!         .invocation(&workspace)
//!         .args(["clippy", "--message-format=json"])
//!         .run()
//!         .await?;
//!     let report = CargoReport::read(&execution.stdout().to_string_lossy())?;
//!
//!     println!("{} diagnostics", report.diagnostics().len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [cargo]: https://doc.rust-lang.org/cargo/
//! [tokio]: https://tokio.rs

mod cargo;
pub mod lockfile;
mod report;
mod root;
mod toolchain;
mod version;

pub use self::cargo::{Cargo, DiscoverRootsError};
pub use self::report::{CargoDiagnostic, CargoReport, ReadReportError};
pub use self::root::{
    CargoPackage, CargoRoot, Documentation, DocumentedNames, PackageName, Publication,
};
pub use self::toolchain::Toolchain;
pub use self::version::RustVersion;
