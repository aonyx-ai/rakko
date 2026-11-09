//! The machinery that the actions which wrap prettier share
//!
//! [Prettier] formats many languages, and one action wraps each group of files
//! that a project keeps apart, so that a check names the language that needs
//! attention. Every one of those actions asks the same questions: does this
//! project hold files of the group, which prettier does it run, and what did
//! that prettier report? This crate answers all three, so that an action
//! states the operation and the files that it wants, and reads the answer as
//! data.
//!
//! Prettier discovers no files of its own. A run names the files that it
//! examines, and a [`Filter`] holds that name: an action states the extensions
//! of its group, and the crate writes the pattern that prettier reads. A
//! filter that names no extension covers every language that prettier
//! understands.
//!
//! Prettier reports as text, and this crate reads that text. The shape of the
//! text belongs to a version of prettier, so keeping the reading in one place
//! keeps the version surface in one place as well: a prettier that reports
//! differently breaks one crate, and not one crate per group of files.
//!
//! For the same reason, the crate pins the version of prettier that every
//! such action runs, in the file `mise.rakko.toml` that the build embeds.
//! Prettier runs on Node, which is a toolchain of the project. An action
//! declares the version of prettier with [`Prettier::requirement`] and the
//! range of Node with [`Prettier::node_requirement`]. The version of
//! prettier that the project pins does not change what runs.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that,
//! and it panics without one.
//!
//! # Examples
//!
//! An action resolves prettier and reads what one operation reported:
//!
//! ```no_run
//! use rakko::Context;
//! use rakko_prettier::{FileExtension, Filter, Operation, Prettier};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let context = Context::builder().root("/home/otter/project").build();
//! let filter = Filter::new([FileExtension::new("yaml"), FileExtension::new("yml")]);
//!
//! let prettier = Prettier::resolve(&context).await?;
//! let observation = prettier.observe(Operation::Report, &filter).await?;
//!
//! if observation.unmatched_pattern() {
//!     println!("nothing to examine");
//! } else {
//!     println!("{} problems", observation.problems().len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [prettier]: https://prettier.io
//! [tokio]: https://tokio.rs

/// Types for the files that a run of prettier examines
mod filter;
/// Types for what one run of prettier produced
mod observation;
/// Types for the operations of prettier that an action runs
mod operation;
/// Types for the prettier that a project runs
mod prettier;
/// Types for one problem that prettier reported about a file
mod problem;

pub use self::filter::{FileExtension, Filter};
pub use self::observation::Observation;
pub use self::operation::Operation;
pub use self::prettier::{ObservePrettierError, Prettier};
pub use self::problem::{PrettierProblem, ProblemDetail};
