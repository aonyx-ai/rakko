//! What one run of taplo produced
//!
//! Taplo reports as text on its standard error stream, and this module turns
//! that text into data. The reading recognizes the lines that carry an
//! answer and ignores everything else, so a log line that a new version adds
//! does not break it. What the reading cannot find is absent from the
//! observation, and the caller decides what the absence means.

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

    /// Whether the report closes with the summary of a failed run
    ///
    /// A run that ends without success sums its failure up on its last
    /// line. A report of such a run without this line lost its tail, and
    /// the problems that it holds can be incomplete.
    #[getset(get_copy = "pub")]
    failure_reported: bool,

    /// The problems that taplo reported, in the order of the report
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
        #[builder(default)] failure_reported: bool,
        #[builder(default)] problems: Vec<TaploProblem>,
        rejected_configuration: Option<String>,
        #[builder(into, default)] stderr: String,
        #[builder(default)] succeeded: bool,
    ) -> Self {
        Self {
            checked,
            failure_reported,
            problems,
            rejected_configuration,
            stderr,
            succeeded,
        }
    }

    /// Returns whether the report carries what its exit status promises
    ///
    /// A run that ended with success closes its report with the count of
    /// the files, and a run that ended without success closes it with the
    /// summary of the failure. Taplo can lose the tail of its report when it
    /// exits, so a report without its closing line arrived incomplete, and
    /// the problems that it holds can be missing some of their company.
    // taplo[impl run.complete]
    pub fn complete(&self) -> bool {
        if self.succeeded {
            self.checked.is_some()
        } else {
            self.failure_reported
        }
    }

    /// Reads what a run of taplo produced
    ///
    /// The reading takes the text of the run and its exit status. A caller
    /// that holds a run of its own therefore reads it the same way as the
    /// machinery of this crate does.
    pub fn read(execution: &Execution) -> Self {
        self::report::read(
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

    use super::*;

    /// Returns the observation of a run with the given count and summary
    fn observation(succeeded: bool, checked: Option<u64>, failure_reported: bool) -> Observation {
        Observation::builder()
            .maybe_checked(checked)
            .failure_reported(failure_reported)
            .succeeded(succeeded)
            .build()
    }

    // taplo[verify run.complete]
    #[test]
    fn failed_run_that_summed_up_its_failure_is_complete() {
        let observation = observation(false, None, true);

        assert!(observation.complete());
    }

    // taplo[verify run.complete]
    #[test]
    fn failed_run_without_a_summary_is_incomplete() {
        let observation = observation(false, None, false);

        assert!(!observation.complete());
    }

    // taplo[verify run.complete]
    #[test]
    fn passing_run_that_counted_its_files_is_complete() {
        let observation = observation(true, Some(3), false);

        assert!(observation.complete());
    }

    // taplo[verify run.complete]
    #[test]
    fn passing_run_without_a_count_is_incomplete() {
        let observation = observation(true, None, false);

        assert!(!observation.complete());
    }
}
