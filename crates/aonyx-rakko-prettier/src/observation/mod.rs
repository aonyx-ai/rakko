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

use crate::exit::Exit;
use crate::matching::Matching;
use crate::operation::Operation;
use crate::problem::PrettierProblem;

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
/// use aonyx_rakko_prettier::{Exit, Observation};
///
/// let observation = Observation::builder()
///     .succeeded(Exit::Succeeded)
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

    /// How the run of prettier ended
    #[getset(get_copy = "pub")]
    succeeded: Exit,

    /// Whether the pattern of the run matched a file
    ///
    /// Prettier treats a pattern that matches nothing as an error. So this is
    /// how a run reports that the project holds no file of the group. A caller
    /// answers it as a run that had nothing to examine, not as a failure.
    #[getset(get_copy = "pub")]
    unmatched_pattern: Matching,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of prettier builds one from what prettier wrote. A caller
    /// builds one where a test stands in for a prettier that nobody started.
    /// Every part is optional. A test therefore names what its case is about and
    /// leaves the rest at the answer of a run that reported nothing.
    ///
    /// # Examples
    ///
    /// A test names the one part that its case is about:
    ///
    /// ```
    /// use aonyx_rakko_prettier::{Exit, Observation};
    ///
    /// let observation = Observation::builder().succeeded(Exit::Failed).build();
    ///
    /// assert_eq!(observation.succeeded(), Exit::Failed);
    /// assert!(observation.problems().is_empty());
    /// ```
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<PrettierProblem>,
        rejected_configuration: Option<String>,
        #[builder(default)] rewritten: Vec<PathBuf>,
        #[builder(into, default)] stderr: String,
        #[builder(default)] succeeded: Exit,
        #[builder(default)] unmatched_pattern: Matching,
    ) -> Self {
        Self {
            problems,
            rejected_configuration,
            rewritten,
            stderr,
            succeeded,
            unmatched_pattern,
        }
    }

    /// Reads what a run of prettier produced
    ///
    /// The reading takes both streams of the run and its exit status. It also
    /// takes the operation, because a rewrite states how long each file took
    /// and marks the files that it left alone. A report names paths and
    /// nothing else.
    #[builder]
    pub(crate) fn read(execution: &Execution, operation: Operation) -> Self {
        let exit = if execution.status().success() {
            Exit::Succeeded
        } else {
            Exit::Failed
        };

        self::report::read()
            .stdout(&execution.stdout().to_string_lossy())
            .stderr(&execution.stderr().to_string_lossy())
            .succeeded(exit)
            .operation(operation)
            .call()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }
}
