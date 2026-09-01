//! The machinery that the actions which wrap taplo share
//!
//! [Taplo] does several jobs of project maintenance, and one action wraps
//! each of them. Every one of those actions asks the same questions: does
//! this project hold TOML files at all, which taplo does it run, and what
//! did that taplo report? This crate answers all three, so that an action
//! states the operation that it wants and reads the answer as data.
//!
//! Taplo reports as text on its standard error stream, and this crate reads
//! that text. The shape of the text belongs to a version of taplo, so
//! keeping the reading in one place keeps the version surface in one place
//! as well: a taplo that reports differently breaks one crate, and not one
//! crate per operation.
//!
//! # Asynchronous Runtime
//!
//! The look at a project reads directories, and a run starts a program and
//! waits for it. A [Tokio] runtime drives both, and they panic without one.
//!
//! # Examples
//!
//! An action looks at the project, resolves taplo, and reads what one
//! operation reported:
//!
//! ```no_run
//! use rakko_action::ProjectRoot;
//! use rakko_taplo::{Operation, Taplo};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let root = ProjectRoot::new("/home/otter/project".into());
//!
//! if Taplo::applies(&root).await {
//!     let taplo = Taplo::resolve(root).await?;
//!     let observation = taplo.observe(Operation::Lint).await?;
//!
//!     println!("{} problems", observation.problems().len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [taplo]: https://taplo.tamasfe.dev
//! [tokio]: https://tokio.rs

/// Types for what one run of taplo produced
pub mod observation;
/// Types for the operations of taplo that an action runs
pub mod operation;
/// Types for one problem that taplo reported about a file
pub mod problem;
/// Types for the taplo that a project runs
pub mod taplo;

pub use self::observation::Observation;
pub use self::operation::Operation;
pub use self::problem::{ProblemDetail, TaploProblem};
pub use self::taplo::{ObserveTaploError, Taplo};
