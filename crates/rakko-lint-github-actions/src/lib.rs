//! The action that audits the GitHub Actions workflows of a project
//!
//! [Zizmor] does the work: it collects the workflows, reads its own
//! configuration, and applies the audits that it knows, so a run of the action
//! agrees with an editor and with a contributor that runs zizmor bare. The
//! action starts the zizmor that [mise] installed for the project, at the
//! version that the project pinned, and translates what zizmor reported into
//! the outcome of the run.
//!
//! Zizmor looks for the patterns that turn a workflow into a way into the
//! repository, such as a template that expands attacker-controlled text into a
//! shell, a job that carries more permissions than it needs, and an action that
//! no digest pins. Whether the same file is well-formed YAML, and whether it
//! obeys the layout rules of the project, are the questions of the actions that
//! wrap yamllint and prettier.
//!
//! A run only reports, so the action takes no argument. The action applies to a
//! project that holds GitHub Actions workflows, and it skips visibly otherwise.
//!
//! A finding of zizmor names one or more places of a workflow: where the
//! finding is, and what a reader needs to read it. Zizmor draws them together
//! in one block of source, which an outcome of Rakko has no place for, so each
//! place becomes a finding of its own at the range that zizmor named. The audit
//! and the severity in each message say which of them belong together.
//!
//! Zizmor gives each finding a severity, and this crate reports every severity,
//! because each of them is a pattern that zizmor was asked to look for. A run
//! with any of them fails.
//!
//! A run asks zizmor for the pedantic persona, which reports the code smells of
//! a workflow as well. Zizmor takes a persona on its command line alone, and
//! its configuration file has no key for one, so a project cannot ask for a
//! persona of its own. A run also asks zizmor to stop at a file that it
//! collected and cannot read, so that no workflow leaves the audit through a
//! warning.
//!
//! # Asynchronous Runtime
//!
//! The look at a project reads a directory, and a run starts a program and
//! waits for it. A [Tokio] runtime drives both, and they panic without one.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_lint_github_actions::LintGitHubActions;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintGitHubActions)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs
//! [zizmor]: https://docs.zizmor.sh

/// Types for the action that audits the GitHub Actions workflows of a project
pub mod lint_github_actions;
/// Types for what one run of zizmor produced
pub mod observation;
/// Types for one place that zizmor reported about a workflow
pub mod problem;
/// Types for the zizmor that a project runs
pub mod zizmor;

pub use self::lint_github_actions::{LintGitHubActions, LintGitHubActionsError};
pub use self::observation::Observation;
pub use self::problem::{Severity, ZizmorProblem};
pub use self::zizmor::{ObserveZizmorError, Zizmor};
