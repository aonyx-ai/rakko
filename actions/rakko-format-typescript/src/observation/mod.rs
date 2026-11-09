//! What one run of oxfmt produced
//!
//! Oxfmt reports over both of its streams: the files that a run listed travel
//! on the standard output stream, and everything that oxfmt could not do
//! travels on the standard error stream. This module turns both into data. The
//! reading recognizes what carries an answer and ignores the rest, so a line
//! that a new version adds does not break it. What the reading cannot find is
//! absent from the observation, and the caller decides what the absence means.

mod report;

use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

use crate::problem::OxfmtProblem;

/// How a run of oxfmt ended
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub(crate) enum Exit {
    /// Oxfmt ended the run with success
    Succeeded,

    /// Oxfmt ended the run without success
    #[default]
    Failed,
}

/// Whether the pattern of a run matched a file
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub(crate) enum Matching {
    /// The pattern matched at least one file
    #[default]
    Matched,

    /// Oxfmt refused the pattern because nothing matched it
    Unmatched,
}

/// What one run of oxfmt produced
///
/// The value holds the answer of one run: whether oxfmt ended with success,
/// what it said about a configuration that it refused, whether it refused the
/// pattern of the run, the problems that it named, and the text that it wrote
/// about its failures.
///
/// An observation describes the run and judges nothing. A file that is not
/// formatted and a file that oxfmt refused are both problems of the project,
/// and the action that asked for the run decides what they mean for its
/// outcome.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub(crate) struct Observation {
    /// The problems that oxfmt reported, in the order of the report
    #[getset(get = "pub(crate)")]
    problems: Vec<OxfmtProblem>,

    /// What oxfmt said about a configuration file that it refused
    ///
    /// Oxfmt formats nothing at all when it cannot accept a configuration, so
    /// this is a run that never looked at a file of the project.
    #[getset(get = "pub(crate)")]
    rejected_configuration: Option<String>,

    /// What oxfmt wrote to its standard error stream
    #[getset(get = "pub(crate)")]
    stderr: String,

    /// How the run ended
    #[getset(get_copy = "pub(crate)")]
    exit: Exit,

    /// Whether the pattern of the run matched a file
    ///
    /// Oxfmt treats a pattern that matches nothing as an error, so this is how
    /// a run reports that the project holds no file of the language. A caller
    /// answers it as a run that had nothing to examine, not as a failure.
    #[getset(get_copy = "pub(crate)")]
    matching: Matching,
}

impl Observation {
    /// Reads what a run of oxfmt produced
    ///
    /// The reading takes both streams of the run and its exit status.
    pub(crate) fn read(execution: &Execution) -> Self {
        let exit = if execution.status().success() {
            Exit::Succeeded
        } else {
            Exit::Failed
        };
        self::report::read()
            .stdout(&execution.stdout().to_string_lossy())
            .stderr(&execution.stderr().to_string_lossy())
            .exit(exit)
            .call()
    }
}

#[cfg(test)]
#[bon::bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of oxfmt builds one through [`read`][read], and a caller builds
    /// one where a test stands in for an oxfmt that nobody started. Every part
    /// is optional, so a test names what its case is about and leaves the rest
    /// at the answer of a run that reported nothing.
    ///
    /// [read]: Observation::read
    #[builder]
    pub(crate) fn new(
        #[builder(default)] problems: Vec<OxfmtProblem>,
        rejected_configuration: Option<String>,
        #[builder(into, default)] stderr: String,
        #[builder(default)] exit: Exit,
        #[builder(default)] matching: Matching,
    ) -> Self {
        Self {
            problems,
            rejected_configuration,
            stderr,
            exit,
            matching,
        }
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
    fn matching_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Matching>();
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }
}
