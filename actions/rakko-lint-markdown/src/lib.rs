//! The action that lints the Markdown files of a project
//!
//! [Markdownlint] does the work: it discovers the Markdown files, reads its
//! own configuration, and applies the rules that the project turned on, so a
//! run of the action agrees with an editor and with a contributor that runs
//! markdownlint bare. The action starts the markdownlint that [mise]
//! installed for the project, at the version that the project pinned, and
//! translates what markdownlint reported into the outcome of the run.
//!
//! A run only reports, so the action takes no argument. Markdownlint can
//! repair a part of what it finds, and it names none of the files that it
//! rewrote, so a repair here would arrive without a name. It applies to a
//! project that holds Markdown files, and it skips visibly otherwise.
//!
//! Markdownlint reports as JSON, and this crate reads that JSON. The shape
//! belongs to a version of markdownlint, so a report that the crate cannot
//! read stops the run instead of passing quietly.
//!
//! # Asynchronous Runtime
//!
//! The look at a project reads directories, and a run starts a program and
//! waits for it. A [Tokio] runtime drives both, and they panic without one.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_lint_markdown::LintMarkdown;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintMarkdown)];
//! ```
//!
//! [markdownlint]: https://github.com/DavidAnson/markdownlint
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs

/// Types for the action that lints the Markdown files of a project
pub mod lint_markdown;
/// Types for the markdownlint that a project runs
pub mod markdownlint;
/// Types for what one run of markdownlint produced
pub mod observation;
/// Types for one rule that markdownlint reported about a file
pub mod problem;

pub use self::lint_markdown::{LintMarkdown, LintMarkdownError};
pub use self::markdownlint::{Markdownlint, ObserveMarkdownlintError};
pub use self::observation::Observation;
pub use self::problem::MarkdownlintProblem;
