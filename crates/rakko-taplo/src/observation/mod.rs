//! What one run of taplo produced
//!
//! Taplo reports as text on two streams, and this module turns that text
//! into data. A formatting run names the files that it would rewrite on its
//! standard output stream, and everything else arrives on its standard error
//! stream. The reading recognizes the lines that carry an answer and ignores
//! everything else, so a log line that a new version adds does not break it.
//! What the reading cannot find is absent from the observation, and the
//! caller decides what the absence means.

/// The reading of the text that taplo wrote
mod report;

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

use crate::problem::TaploProblem;

/// What one run of taplo produced
///
/// The value holds the answer of one run: whether taplo ended with success,
/// what it said about a configuration file that it rejected, how many files
/// it examined, the problems that it named, and the text that it wrote. A
/// field that is `None` means that taplo wrote no such line.
///
/// An observation describes the run and judges nothing. A file that is not
/// formatted and a file that taplo refused are both problems of the project,
/// and the action that asked for the run decides what they mean for its
/// outcome.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// How many files the run examined, after its configuration excluded
    ///
    /// Taplo counts every file that it matched and the files that the
    /// configuration excluded separately, and the difference is what a run
    /// examined.
    #[getset(get_copy = "pub")]
    checked: Option<u64>,

    /// The problems that taplo reported
    ///
    /// The problems of one stream stand in the order of that stream, and
    /// the problems that taplo wrote to its standard error stream come
    /// first. The two streams describe different files, because taplo
    /// offers no difference for a file that it could not parse.
    #[getset(get = "pub")]
    problems: Vec<TaploProblem>,

    /// What taplo said about a configuration file that it rejected
    ///
    /// Taplo warns about a configuration that it cannot read and then runs
    /// with its defaults, so this is the only trace of the rejection.
    #[getset(get = "pub")]
    rejected_configuration: Option<String>,

    /// What taplo wrote to its standard error stream
    #[getset(get = "pub")]
    stderr: String,

    /// Whether taplo ended with success
    #[getset(get_copy = "pub")]
    succeeded: bool,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of taplo builds one through [`read`][read], and a caller builds
    /// one where a test stands in for a taplo that nobody started. Every
    /// part is optional, so a test names what its case is about and leaves
    /// the rest at the answer of a run that reported nothing.
    ///
    /// [read]: Observation::read
    #[builder]
    pub fn new(
        checked: Option<u64>,
        #[builder(default)] problems: Vec<TaploProblem>,
        rejected_configuration: Option<String>,
        #[builder(into, default)] stderr: String,
        #[builder(default)] succeeded: bool,
    ) -> Self {
        Self {
            checked,
            problems,
            rejected_configuration,
            stderr,
            succeeded,
        }
    }

    /// Returns whether the report holds the answer of the run
    ///
    /// A run that ended with success answered by ending that way: taplo
    /// leaves nothing for a reader to find, and no line that a report lost
    /// can turn a run that found problems into one that found none.
    ///
    /// A run that ended without success found something, so its report
    /// names at least one problem. A report of such a run that names none
    /// lost the lines that held them, and the caller must not read a silent
    /// failure as an empty one.
    // taplo[impl run.complete+2]
    pub fn complete(&self) -> bool {
        self.succeeded || !self.problems.is_empty()
    }

    /// Reads what a run of taplo produced
    ///
    /// The reading takes both streams of the run and its exit status. A
    /// caller that holds a run of its own therefore reads it the same way as
    /// the machinery of this crate does.
    pub fn read(execution: &Execution) -> Self {
        self::report::read(
            &execution.stdout().to_string_lossy(),
            &execution.stderr().to_string_lossy(),
            execution.status().success(),
        )
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;
    use crate::problem::ProblemDetail;

    /// Returns the observation of a run that ended the given way
    fn observation(succeeded: bool, problems: Vec<TaploProblem>) -> Observation {
        Observation::builder()
            .problems(problems)
            .succeeded(succeeded)
            .build()
    }

    /// Returns one problem, so that a report has something to hold
    fn problem() -> TaploProblem {
        TaploProblem::new(
            PathBuf::from("/home/otter/project/a.toml"),
            ProblemDetail::Unformatted,
        )
    }

    // taplo[verify run.complete+2]
    #[test]
    fn failed_run_that_named_a_problem_is_complete() {
        let observation = observation(false, vec![problem()]);

        assert!(observation.complete());
    }

    // taplo[verify run.complete+2]
    #[test]
    fn failed_run_without_a_problem_is_incomplete() {
        let observation = observation(false, Vec::new());

        assert!(!observation.complete());
    }

    // taplo[verify run.complete+2]
    #[test]
    fn passing_run_without_a_count_is_complete() {
        let observation = observation(true, Vec::new());

        assert!(observation.complete());
    }
}
