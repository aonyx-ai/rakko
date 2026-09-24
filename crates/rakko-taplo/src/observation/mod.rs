//! What one run of taplo produced
//!
//! Taplo reports as text on two streams, and this module turns that text
//! into data. A formatting run names the files that it would rewrite on its
//! standard output stream, and everything else arrives on its standard error
//! stream. The reading recognizes the lines that carry an answer and ignores
//! everything else, so a log line that a new version adds does not break it.
//! What the reading cannot find is absent from the observation, and the
//! caller decides what the absence means.
//!
//! One answer needs more than the text. A validating run names a file that it
//! could not read only on a line that taplo can lose when it exits, so the
//! reading of such a run also reads the files that taplo examined.

/// The reading of the text that taplo wrote
mod report;

use std::collections::HashSet;
use std::path::PathBuf;

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

use crate::operation::Operation;
use crate::problem::{ProblemDetail, TaploProblem};

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

    /// The files that the run examined, in the order in which taplo listed
    /// them
    ///
    /// Taplo lists the files on the line that counts them, and the list is
    /// empty when the report lost that line. The reading of a failed
    /// validating run reads each of these files, to find the files that
    /// taplo could not read.
    examined: Vec<PathBuf>,

    /// The problems that taplo reported
    ///
    /// The problems of one stream stand in the order of that stream, and
    /// the problems that taplo wrote to its standard error stream come
    /// first. The two streams describe different files, because taplo
    /// offers no difference for a file that it could not parse. A file that
    /// the reading itself could not read comes last.
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
            examined: Vec::new(),
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
    /// The reading takes both streams of the run, its exit status, and the
    /// operation that the run did. A caller that holds a run of its own
    /// therefore reads it the same way as the machinery of this crate does.
    ///
    /// A validating run names a file that it could not read only on a line
    /// that taplo can lose when it exits. The reading of a validating run
    /// that ended without success therefore also reads each file that taplo
    /// examined, and a file that it cannot read becomes a problem even when
    /// taplo lost that line. The reading changes no file.
    pub async fn read(execution: &Execution, operation: Operation) -> Self {
        let observation = self::report::read(
            &execution.stdout().to_string_lossy(),
            &execution.stderr().to_string_lossy(),
            execution.status().success(),
        );

        match operation {
            Operation::Lint => observation.with_unreadable_files().await,
            Operation::CheckFormat | Operation::Format => observation,
        }
    }

    /// Adds a problem for each examined file that cannot be read
    ///
    /// A validating run names a file that it could not read on a single line,
    /// and that line comes just before the end of the report, where taplo
    /// can lose it. The line that counts the files comes early and lists
    /// every file that the run examined, so the reading tries to read each of
    /// those files. A file that it cannot read becomes a problem with the
    /// message of the operating system, which is also the reason that taplo
    /// gives. A file that a problem already names stays as taplo reported it.
    ///
    /// A run that ended with success read every file, so nothing is read for
    /// it.
    // taplo[impl report.unreadable]
    async fn with_unreadable_files(mut self) -> Self {
        if self.succeeded {
            return self;
        }

        let named: HashSet<PathBuf> = self
            .problems
            .iter()
            .map(|problem| problem.path().clone())
            .collect();

        for path in self.examined.iter().filter(|path| !named.contains(*path)) {
            if let Err(error) = tokio::fs::read(path).await {
                let detail = ProblemDetail::Invalid {
                    reason: error.to_string(),
                };

                self.problems.push(TaploProblem::new(path.clone(), detail));
            }
        }

        self
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use tempfile::TempDir;

    use super::*;

    /// Returns the observation of a failed validating run over the given
    /// files
    fn examined(files: Vec<PathBuf>, problems: Vec<TaploProblem>) -> Observation {
        Observation {
            examined: files,
            ..observation(false, problems)
        }
    }

    /// Returns a directory with a file that can be read, and a path in it
    /// that cannot
    ///
    /// The path that cannot be read names no file. Every platform refuses to
    /// read it, and a test that uses it takes no permission away from a file.
    fn files() -> (TempDir, PathBuf, PathBuf) {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let readable = directory.path().join("clean.toml");
        std::fs::write(&readable, "x = 1\n").expect("the test writes a file");
        let missing = directory.path().join("missing.toml");

        (directory, readable, missing)
    }

    /// Returns the observation of a run that ended the given way
    fn observation(succeeded: bool, problems: Vec<TaploProblem>) -> Observation {
        Observation::builder()
            .problems(problems)
            .succeeded(succeeded)
            .build()
    }

    /// Returns the problem of a file that cannot be read
    ///
    /// The reason is what the operating system says about the path, which
    /// is also what taplo says about it.
    fn unreadable(path: &Path) -> TaploProblem {
        let reason = std::fs::read(path)
            .expect_err("the test names a path that cannot be read")
            .to_string();

        TaploProblem::new(path.to_path_buf(), ProblemDetail::Invalid { reason })
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

    // taplo[verify report.unreadable]
    #[tokio::test]
    async fn with_unreadable_files_of_a_failed_run_reports_the_file() {
        let (_directory, readable, missing) = files();
        let observation = examined(vec![readable, missing.clone()], Vec::new());

        let observation = observation.with_unreadable_files().await;

        assert_eq!(observation.problems(), &vec![unreadable(&missing)]);
    }

    // taplo[verify report.unreadable]
    #[tokio::test]
    async fn with_unreadable_files_of_a_file_that_a_problem_names_keeps_that_problem() {
        let (_directory, _readable, missing) = files();
        let named = TaploProblem::new(
            missing.clone(),
            ProblemDetail::Invalid {
                reason: "Permission denied (os error 13)".to_owned(),
            },
        );
        let observation = examined(vec![missing], vec![named.clone()]);

        let observation = observation.with_unreadable_files().await;

        assert_eq!(observation.problems(), &vec![named]);
    }

    // taplo[verify report.unreadable]
    #[tokio::test]
    async fn with_unreadable_files_of_a_passing_run_reads_nothing() {
        let (_directory, _readable, missing) = files();
        let observation = Observation {
            examined: vec![missing],
            ..observation(true, Vec::new())
        };

        let observation = observation.with_unreadable_files().await;

        assert!(observation.problems().is_empty());
    }

    // taplo[verify report.unreadable]
    // taplo[verify run.complete+2]
    #[tokio::test]
    async fn with_unreadable_files_of_a_report_that_lost_the_line_is_complete() {
        let (directory, readable, missing) = files();
        let stderr = format!(
            " INFO taplo:lint_files:collect_files: found files total=2 excluded=0 files=[{readable:?}, {missing:?}] cwd={:?}\n",
            directory.path()
        );
        let observation = super::report::read("", &stderr, false);

        let observation = observation.with_unreadable_files().await;

        assert!(observation.complete());
    }
}
