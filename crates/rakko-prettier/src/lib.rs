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
//! use rakko_action::ProjectRoot;
//! use rakko_prettier::{FileExtension, Filter, Operation, Prettier};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let root = ProjectRoot::new("/home/otter/project".into());
//! let filter = Filter::new([FileExtension::new("yaml"), FileExtension::new("yml")]);
//!
//! let prettier = Prettier::resolve(root).await?;
//! let observation = prettier.observe(Operation::Report, &filter).await?;
//!
//! if observation.unmatched_pattern() == rakko_prettier::Pattern::Unmatched {
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

pub mod filter;
pub mod observation;
pub mod operation;
pub mod prettier;
pub mod problem;

pub use self::filter::{FileExtension, Filter};
pub use self::observation::{Exit, Observation, Pattern};
pub use self::operation::Operation;
pub use self::prettier::{ObservePrettierError, Prettier};
pub use self::problem::{PrettierProblem, ProblemDetail};
