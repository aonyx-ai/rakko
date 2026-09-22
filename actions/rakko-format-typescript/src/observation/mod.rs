//! What one run of oxfmt produced
//!
//! Oxfmt reports over both of its streams: the files that a run listed travel
//! on the standard output stream, and everything that oxfmt could not do
//! travels on the standard error stream. This module turns both into data. The
//! reading recognizes what carries an answer and ignores the rest, so a line
//! that a new version adds does not break it. What the reading cannot find is
//! absent from the observation, and the caller decides what the absence means.

/// The reading of the text that oxfmt wrote
mod report;

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

use crate::problem::OxfmtProblem;

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
pub struct Observation {
    /// The problems that oxfmt reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<OxfmtProblem>,

    /// What oxfmt said about a configuration file that it refused
    ///
    /// Oxfmt formats nothing at all when it cannot accept a configuration, so
    /// this is a run that never looked at a file of the project.
    #[getset(get = "pub")]
    rejected_configuration: Option<String>,

    /// What oxfmt wrote to its standard error stream
    #[getset(get = "pub")]
    stderr: String,

    /// Whether oxfmt ended with success
    #[getset(get_copy = "pub")]
    succeeded: bool,

    /// Whether oxfmt refused the pattern of the run because nothing matched it
    ///
    /// Oxfmt treats a pattern that matches nothing as an error, so this is how
    /// a run reports that the project holds no file of the language. A caller
    /// answers it as a run that had nothing to examine, not as a failure.
    #[getset(get_copy = "pub")]
    unmatched_pattern: bool,
}

#[bon]
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
    pub fn new(
        #[builder(default)] problems: Vec<OxfmtProblem>,
        rejected_configuration: Option<String>,
        #[builder(into, default)] stderr: String,
        #[builder(default)] succeeded: bool,
        #[builder(default)] unmatched_pattern: bool,
    ) -> Self {
        Self {
            problems,
            rejected_configuration,
            stderr,
            succeeded,
            unmatched_pattern,
        }
    }

    /// Reads what a run of oxfmt produced
    ///
    /// The reading takes both streams of the run and its exit status.
    pub fn read(execution: &Execution) -> Self {
        self::report::read(
            &execution.stdout().to_string_lossy(),
            &execution.stderr().to_string_lossy(),
            execution.status().success(),
        )
    }
}
