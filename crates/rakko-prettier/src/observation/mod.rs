//! What one run of prettier produced
//!
//! Prettier reports over both of its streams. The files that a run named travel
//! on the standard output stream. Everything that prettier could not do travels
//! on the standard error stream. This module turns both into data. The reading
//! recognizes the lines that carry an answer and ignores everything else, so a
//! line that a new version adds does not break it. What the reading cannot find
//! is absent from the observation, and the caller decides what the absence
//! means.

mod report;

use std::path::PathBuf;

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

use crate::operation::Operation;
use crate::problem::PrettierProblem;

/// The exit status of a prettier run
///
/// # Examples
///
/// ```
/// use rakko_prettier::observation::{Exit, Observation};
///
/// // A run that reports nothing is a failure until prettier says otherwise.
/// let observation = Observation::builder().build();
///
/// assert_eq!(observation.exit(), Exit::Failed);
/// assert_eq!(Exit::default(), Exit::Failed);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum Exit {
    /// Prettier ended with success
    Succeeded,

    /// Prettier encountered an error
    #[default]
    Failed,
}

/// Whether a prettier run matched the given file pattern
///
/// # Examples
///
/// ```
/// use rakko_prettier::observation::{Observation, Pattern};
///
/// let observation = Observation::builder()
///     .unmatched_pattern(Pattern::Unmatched)
///     .build();
///
/// assert_eq!(observation.unmatched_pattern(), Pattern::Unmatched);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum Pattern {
    /// The pattern matched at least one file
    #[default]
    Matched,

    /// The pattern matched no files
    Unmatched,
}

/// What one run of prettier produced
///
/// The value holds the answer of one run. The answer holds whether prettier
/// ended with success and what it said about a configuration that did not reach
/// the run. The answer also holds whether prettier refused the pattern of the
/// run and the problems that prettier named. Last, the answer holds the files
/// that prettier rewrote and the text that prettier wrote about its failures.
///
/// An observation describes the run and judges nothing. A file that is not
/// formatted and a file that prettier refused are both problems of the project.
/// The action that asked for the run decides what they mean for its outcome.
///
/// # Examples
///
/// ```
/// use rakko_prettier::observation::{Exit, Observation};
///
/// let observation = Observation::builder()
///     .exit(Exit::Succeeded)
///     .stderr("[warn] Ignored unknown option")
///     .build();
///
/// assert!(observation.problems().is_empty());
/// assert!(observation.rewritten().is_empty());
/// assert_eq!(observation.stderr(), "[warn] Ignored unknown option");
/// ```
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The problems that prettier reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<PrettierProblem>,

    /// What prettier said about a configuration that did not reach the run
    ///
    /// Prettier ignores an option that it does not know with a warning and then
    /// runs without it. So this is the only trace of the loss.
    #[getset(get = "pub")]
    rejected_configuration: Option<String>,

    /// The files that a rewrite changed, in the order of the report
    ///
    /// A run that only reports leaves this empty, because such a run changes
    /// nothing.
    #[getset(get = "pub")]
    rewritten: Vec<PathBuf>,

    /// What prettier wrote to its standard error stream
    #[getset(get = "pub")]
    stderr: String,

    /// Whether prettier ended with success
    #[getset(get_copy = "pub")]
    exit: Exit,

    /// Whether prettier refused the pattern of the run because nothing
    /// matched it
    ///
    /// Prettier treats a pattern that matches nothing as an error. So this is
    /// how a run reports that the project holds no file of the group. A caller
    /// answers it as a run that had nothing to examine, not as a failure.
    #[getset(get_copy = "pub")]
    unmatched_pattern: Pattern,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of prettier builds one through [`read`][read], and a caller builds
    /// one where a test stands in for a prettier that nobody started. Every
    /// part is optional. So a test names what its case is about and leaves the
    /// rest at the answer of a run that reported nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_prettier::observation::{Exit, Observation};
    ///
    /// let observation = Observation::builder().exit(Exit::Succeeded).build();
    ///
    /// assert_eq!(observation.exit(), Exit::Succeeded);
    /// ```
    ///
    /// [read]: Observation::read
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<PrettierProblem>,
        rejected_configuration: Option<String>,
        #[builder(default)] rewritten: Vec<PathBuf>,
        #[builder(into, default)] stderr: String,
        #[builder(default)] exit: Exit,
        #[builder(default)] unmatched_pattern: Pattern,
    ) -> Self {
        Self {
            problems,
            rejected_configuration,
            rewritten,
            stderr,
            exit,
            unmatched_pattern,
        }
    }

    /// Reads what a run of prettier produced
    ///
    /// The reading takes both streams of the run and its exit status. It also
    /// takes the operation, because a rewrite states how long each file took
    /// and marks the files that it left alone. In contrast, a report names
    /// paths and nothing else.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_prettier::observation::{Exit, Observation};
    /// use rakko_prettier::Operation;
    /// use rakko_tool::Invocation;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let execution = Invocation::new("prettier")
    ///     .args(["--list-different", "README.md"])
    ///     .run()
    ///     .await?;
    /// let observation = Observation::read()
    ///     .execution(&execution)
    ///     .operation(Operation::Report)
    ///     .call();
    ///
    /// assert_eq!(observation.exit(), Exit::Succeeded);
    /// # Ok(())
    /// # }
    /// ```
    #[builder]
    pub fn read(execution: &Execution, operation: Operation) -> Self {
        let exit = if execution.status().success() {
            Exit::Succeeded
        } else {
            Exit::Failed
        };
        self::report::read()
            .stdout(&execution.stdout().to_string_lossy())
            .stderr(&execution.stderr().to_string_lossy())
            .exit(exit)
            .operation(operation)
            .call()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Exit>();
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }

    #[test]
    fn pattern_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Pattern>();
    }
}
