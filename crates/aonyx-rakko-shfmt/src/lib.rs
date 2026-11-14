//! The machinery that the actions which run shfmt share
//!
//! [Shfmt] formats shell scripts, and one action wraps that job. Shfmt also
//! finds the shell scripts of a project. It walks the project, and it takes a
//! file with a shell extension, or a file whose shebang starts a shell. An
//! action that examines shell scripts with another tool needs the same list.
//! This crate runs shfmt for both jobs, so that an action states the
//! operation that it wants and reads the answer as data.
//!
//! Shfmt reports as text. It writes the paths of files on its standard output
//! stream, and the errors that it found in a file on its standard error
//! stream. This crate reads that text. The shape of the text belongs to a
//! version of shfmt. So keeping the reading in one place keeps the version
//! surface in one place as well.
//!
//! For the same reason, the crate pins the version of shfmt that every such
//! action runs, in the file `mise.rakko.toml` that the build embeds. An action
//! declares that version with [`Shfmt::requirement`], and the version that the
//! project pins does not change what runs.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that,
//! and it panics without one.
//!
//! # Examples
//!
//! An action resolves shfmt and reads what one operation reported:
//!
//! ```no_run
//! use rakko::{Context, ProjectRoot};
//! use aonyx_rakko_shfmt::{Operation, Shfmt};
//! use rakko_test_utils::path;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
//!
//! let shfmt = Shfmt::resolve(&context).await?;
//! let observation = shfmt.observe(Operation::CheckFormat).await?;
//!
//! assert!(observation.files().is_empty());
//! # Ok(())
//! # }
//! ```
//!
//! [shfmt]: https://github.com/mvdan/sh
//! [tokio]: https://tokio.rs

mod diagnostic;
mod observation;
mod operation;
mod shfmt;

pub use self::diagnostic::ShfmtDiagnostic;
pub use self::observation::Observation;
pub use self::operation::Operation;
pub use self::shfmt::{ObserveShfmtError, Shfmt};
