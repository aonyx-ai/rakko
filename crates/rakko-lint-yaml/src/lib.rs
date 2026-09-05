//! The action that lints the YAML files of a project
//!
//! [Yamllint] does the work: it discovers the YAML files, reads its own
//! configuration, and applies the rules that the project turned on, so a run
//! of the action agrees with an editor and with a contributor that runs
//! yamllint bare. The action starts the yamllint that [mise] installed for
//! the project, at the version that the project pinned, and translates what
//! yamllint reported into the outcome of the run.
//!
//! A run only reports, so the action takes no argument, and yamllint repairs
//! nothing that it finds. The action applies to a project that holds YAML
//! files, and it skips visibly otherwise.
//!
//! Yamllint gives each problem a level, and the configuration of the project
//! decides that level for each rule. This crate reports a warning and an
//! error alike, because both are problems that the project asked yamllint to
//! look for, and a run with either fails.
//!
//! A run starts yamllint twice. The first run asks which files yamllint
//! examines, because a yamllint that collected no file writes the same empty
//! report as one that examined the whole project and found nothing. The
//! second run lints those files. A line of the report that this crate cannot
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
//! use rakko_lint_yaml::LintYaml;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintYaml)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs
//! [yamllint]: https://github.com/adrienverge/yamllint

/// Types for the action that lints the YAML files of a project
pub mod lint_yaml;
/// Types for what one run of yamllint produced
pub mod observation;
/// Types for one rule that yamllint reported about a file
pub mod problem;
/// Types for the yamllint that a project runs
pub mod yamllint;

pub use self::lint_yaml::{LintYaml, LintYamlError};
pub use self::observation::Observation;
pub use self::problem::{ProblemLevel, YamllintProblem};
pub use self::yamllint::report::ReadReportError;
pub use self::yamllint::{ListFilesError, ObserveYamllintError, Yamllint};
