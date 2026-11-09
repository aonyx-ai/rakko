//! The machinery that the actions which wrap taplo share
//!
//! [Taplo] does several jobs of project maintenance, and one action wraps each
//! of them. Every one of those actions asks the same questions. Does this
//! project hold TOML files at all? Which taplo does it run? What did that taplo
//! report? This crate answers all three, so that an action states the operation
//! that it wants and reads the answer as data.
//!
//! Taplo reports as text on its standard error stream, and this crate reads
//! that text. The shape of the text belongs to a version of taplo. So keeping
//! the reading in one place keeps the version surface in one place as well. A
//! taplo that reports differently breaks one crate, and not one crate per
//! operation.
//!
//! For the same reason, the crate pins the version of taplo that every such
//! action runs, in the file `mise.rakko.toml` that the build embeds. An
//! action declares that version with [`Taplo::requirement`], and the version
//! that the project pins does not change what runs.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that,
//! and it panics without one.
//!
//! # Examples
//!
//! An action resolves taplo and reads what one operation reported:
//!
//! ```no_run
//! use rakko::{Context, ProjectRoot};
//! use rakko_taplo::{Operation, Taplo};
//! use rakko_test_utils::path;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
//!
//! let taplo = Taplo::resolve(&context).await?;
//! let observation = taplo.observe(Operation::Lint).await?;
//!
//! assert!(observation.problems().is_empty());
//! # Ok(())
//! # }
//! ```
//!
//! [taplo]: https://taplo.tamasfe.dev
//! [tokio]: https://tokio.rs

mod observation;
mod operation;
mod problem;
mod taplo;

pub use self::observation::Observation;
pub use self::operation::Operation;
pub use self::problem::{ProblemDetail, TaploProblem};
pub use self::taplo::{ObserveTaploError, Taplo};
