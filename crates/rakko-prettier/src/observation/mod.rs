//! What one run of prettier produced
//!
//! Prettier reports over both of its streams: the files that a run named
//! travel on the standard output stream, and everything that prettier could
//! not do travels on the standard error stream. This module turns both into
//! data. The reading recognizes the lines that carry an answer and ignores
//! everything else, so a line that a new version adds does not break it. What
//! the reading cannot find is absent from the observation, and the caller
//! decides what the absence means.

/// The reading of the text that prettier wrote
mod report;

use std::path::PathBuf;

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

use crate::operation::Operation;
use crate::problem::PrettierProblem;

/// What one run of prettier produced
///
/// The value holds the answer of one run: whether prettier ended with
/// success, what it said about a configuration that did not reach the run,
/// whether it refused the pattern of the run, the problems that it named, the
/// files that it rewrote, and the text that it wrote about its failures.
///
/// An observation describes the run and judges nothing. A file that is not
/// formatted and a file that prettier refused are both problems of the
/// project, and the action that asked for the run decides what they mean for
/// its outcome.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The problems that prettier reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<PrettierProblem>,

    /// What prettier said about a configuration that did not reach the run
    ///
    /// Prettier ignores an option that it does not know with a warning and
    /// then runs without it, so this is the only trace of the loss.
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
    succeeded: bool,

    /// Whether prettier refused the pattern of the run because nothing
    /// matched it
    ///
    /// The look of a project runs before prettier and keeps this from
    /// happening, so a run that reports it met a tree that changed under it,
    /// or a look that disagrees with the pattern.
    #[getset(get_copy = "pub")]
    unmatched_pattern: bool,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of prettier builds one through [`read`][read], and a caller
    /// builds one where a test stands in for a prettier that nobody started.
    /// Every part is optional, so a test names what its case is about and
    /// leaves the rest at the answer of a run that reported nothing.
    ///
    /// [read]: Observation::read
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<PrettierProblem>,
        rejected_configuration: Option<String>,
        #[builder(default)] rewritten: Vec<PathBuf>,
        #[builder(into, default)] stderr: String,
        #[builder(default)] succeeded: bool,
        #[builder(default)] unmatched_pattern: bool,
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
    /// and marks the files that it left alone, while a report names paths and
    /// nothing else.
    // prettier[impl report.status]
    pub fn read(execution: &Execution, operation: Operation) -> Self {
        self::report::read(
            &execution.stdout().to_string_lossy(),
            &execution.stderr().to_string_lossy(),
            execution.status().success(),
            operation,
        )
    }
}
