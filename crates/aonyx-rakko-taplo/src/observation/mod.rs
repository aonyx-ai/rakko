//! What one run of taplo produced
//!
//! Taplo reports as text on two streams, and this module turns that text into
//! data. A formatting run names the files that it would rewrite on its standard
//! output stream, and everything else arrives on its standard error stream. The
//! reading recognizes the lines that carry an answer and ignores everything
//! else. So a log line that a new version adds does not break it. What the
//! reading cannot find is absent from the observation, and the caller decides
//! what the absence means.
//!
//! One answer needs more than the text. A validating run names a file that it
//! could not read only on a line that taplo can lose when it exits. So the
//! reading of such a run also reads the files that taplo examined.

mod report;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

use crate::operation::Operation;
use crate::problem::{ProblemDetail, TaploProblem};
use crate::taplo::ObserveTaploError;

/// How a run of taplo ended
///
/// A run of taplo ends with success or failure, which changes how the
/// observation reads the report.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
enum Exit {
    /// Taplo ended the run with success
    Succeeded,

    /// Taplo ended the run with failure
    #[default]
    Failed,
}

/// Whether a report says that taplo found a configuration file
///
/// Taplo reports the configuration file that it found before it reads the
/// file. The line comes before the warning about a file that taplo rejected.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
enum Configuration {
    /// The report holds the line about the configuration file
    Reported,

    /// The report holds no line about a configuration file
    #[default]
    Unreported,
}

/// What one run of taplo produced
///
/// The value holds the answer of one run.
/// It includes what taplo said about a rejected configuration file, how many
/// files it examined, and the problems it reported.
/// A field that is `None` means that taplo wrote no such line.
///
/// An observation describes the run and judges nothing.
/// A file that is not formatted and a file that taplo refused are both
/// problems of the project.
/// The action that asked for the run decides what they mean for its
/// outcome.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_taplo::Observation;
///
/// let observation = Observation::builder().checked(3).build();
///
/// assert_eq!(observation.checked(), Some(3));
/// assert!(observation.problems().is_empty());
/// ```
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// How many files the run examined, after its configuration excluded
    ///
    /// Taplo counts every file that it matched and the files that the
    /// configuration excluded separately, and the difference is what a run
    /// examined.
    #[getset(get_copy = "pub")]
    checked: Option<u64>,

    /// Whether the report says that taplo found a configuration file
    configuration: Configuration,

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
    stderr: String,

    /// How the run of taplo ended
    succeeded: Exit,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of taplo builds one from what taplo wrote.
    /// A caller builds one where a test stands in for a taplo that nobody
    /// started.
    /// Every part is optional. A test therefore names what its case is about
    /// and leaves the rest at the answer of a run that reported nothing.
    ///
    /// # Examples
    ///
    /// A test names the one part that its case is about:
    ///
    /// ```
    /// use aonyx_rakko_taplo::Observation;
    ///
    /// let observation = Observation::builder()
    ///     .rejected_configuration("invalid taplo.toml".to_owned())
    ///     .build();
    ///
    /// assert_eq!(observation.checked(), None);
    /// assert_eq!(
    ///     observation.rejected_configuration().as_deref(),
    ///     Some("invalid taplo.toml")
    /// );
    /// ```
    #[builder]
    pub fn new(
        checked: Option<u64>,
        #[builder(default)] problems: Vec<TaploProblem>,
        rejected_configuration: Option<String>,
    ) -> Self {
        Self {
            checked,
            configuration: Configuration::default(),
            examined: Vec::new(),
            problems,
            rejected_configuration,
            stderr: String::new(),
            succeeded: Exit::default(),
        }
    }

    /// Returns whether the report holds the answer of the run
    ///
    /// A run that ended with success answered by ending that way. Taplo leaves
    /// nothing for a reader to find. No line that a report lost can turn a run
    /// that found problems into one that found none.
    ///
    /// A run that ended without success found something, so its report
    /// names at least one problem. A report of such a run that names none
    /// lost the lines that held them. The caller must not read a silent
    /// failure as an empty one.
    pub(crate) fn complete(&self) -> bool {
        match self.succeeded {
            Exit::Succeeded => true,
            Exit::Failed => !self.problems.is_empty(),
        }
    }

    /// Returns the error that leaves the run without an answer, if any
    ///
    /// The configuration is the file that taplo finds in the project, if
    /// the project holds one. A report must then say that taplo found a
    /// configuration file, because only such a report can show whether taplo
    /// rejected it. A report that does not is unanswered, whatever else it
    /// holds. Otherwise the run is unanswered when its report is not
    /// [complete][complete].
    ///
    /// [complete]: Observation::complete
    pub(crate) fn unanswered(&self, configuration: Option<&Path>) -> Option<ObserveTaploError> {
        if let Some(path) = configuration
            && !self.reports_configuration()
        {
            return Some(ObserveTaploError::UnreportedConfiguration {
                path: path.to_path_buf(),
                stderr: self.stderr.clone(),
            });
        }

        if !self.complete() {
            return Some(ObserveTaploError::IncompleteReport {
                stderr: self.stderr.clone(),
            });
        }

        None
    }

    /// Returns whether the report says that taplo found a configuration file
    ///
    /// Taplo writes the warning about a configuration file that it rejected
    /// only after the line that reports the file. A report without that line
    /// therefore cannot show whether taplo rejected a file. The caller must
    /// not trust such a report for a project that holds a configuration file.
    pub(crate) fn reports_configuration(&self) -> bool {
        match self.configuration {
            Configuration::Reported => true,
            Configuration::Unreported => false,
        }
    }

    /// Reads what a run of taplo produced
    ///
    /// The reading takes both streams of the run, its exit status, and the
    /// operation that the run did.
    ///
    /// A validating run names a file that it could not read only on a line
    /// that taplo can lose when it exits.
    /// The reading of a validating run that ended without success therefore
    /// also reads
    /// each file that taplo examined.
    /// A file that it cannot read becomes a problem even when taplo lost
    /// that line.
    /// The reading changes no file.
    #[builder]
    pub(crate) async fn read(execution: &Execution, operation: Operation) -> Self {
        let exit = if execution.status().success() {
            Exit::Succeeded
        } else {
            Exit::Failed
        };
        let observation = self::report::read()
            .stdout(&execution.stdout().to_string_lossy())
            .stderr(&execution.stderr().to_string_lossy())
            .succeeded(exit)
            .call();

        match operation {
            Operation::Lint => observation.with_unreadable_files().await,
            Operation::CheckFormat | Operation::Format => observation,
        }
    }

    /// Adds a problem for each examined file that cannot be read
    ///
    /// A validating run names a file that it could not read on a single line.
    /// That line comes just before the report end, where taplo can lose it.
    /// The line that counts the files comes early and lists every examined
    /// file.
    /// So the reading tries to read each of those files.
    /// A file that it cannot read becomes a problem with the operating system
    /// message (also the reason taplo gives).
    /// A file that a problem already names stays as taplo reported it.
    ///
    /// A run that ended with success read every file, so nothing is read for
    /// it.
    async fn with_unreadable_files(mut self) -> Self {
        match self.succeeded {
            Exit::Succeeded => return self,
            Exit::Failed => {}
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

                self.problems.push(
                    TaploProblem::builder()
                        .path(path.clone())
                        .detail(detail)
                        .build(),
                );
            }
        }

        self
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::Path;

    use tempfile::TempDir;

    use super::*;

    /// The configuration file of a project that a test stands in for
    const CONFIGURATION: &str = "/home/otter/project/.taplo.toml";

    /// Returns the observation of a failed validating run over the given
    /// files
    fn examined(files: Vec<PathBuf>, problems: Vec<TaploProblem>) -> Observation {
        Observation {
            examined: files,
            ..observation(Exit::Failed, problems)
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
    fn observation(succeeded: Exit, problems: Vec<TaploProblem>) -> Observation {
        Observation {
            checked: None,
            configuration: Configuration::default(),
            examined: Vec::new(),
            problems,
            rejected_configuration: None,
            stderr: String::new(),
            succeeded,
        }
    }

    /// Returns the problem of a file that cannot be read
    ///
    /// The reason is what the operating system says about the path, which
    /// is also what taplo says about it.
    fn unreadable(path: &Path) -> TaploProblem {
        let reason = std::fs::read(path)
            .expect_err("the test names a path that cannot be read")
            .to_string();

        TaploProblem::builder()
            .path(path.to_path_buf())
            .detail(ProblemDetail::Invalid { reason })
            .build()
    }

    /// Returns one problem, so that a report has something to hold
    fn problem() -> TaploProblem {
        TaploProblem::builder()
            .path(PathBuf::from("/home/otter/project/a.toml"))
            .detail(ProblemDetail::Unformatted)
            .build()
    }

    #[test]
    fn configuration_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Configuration>();
    }

    #[test]
    fn exit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Exit>();
    }

    #[test]
    fn failed_run_that_named_a_problem_is_complete() {
        let observation = observation(Exit::Failed, vec![problem()]);

        assert!(observation.complete());
    }

    #[test]
    fn failed_run_without_a_problem_is_incomplete() {
        let observation = observation(Exit::Failed, Vec::new());

        assert!(!observation.complete());
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }

    #[test]
    fn passing_run_without_a_count_is_complete() {
        let observation = observation(Exit::Succeeded, Vec::new());

        assert!(observation.complete());
    }

    #[test]
    fn unanswered_of_a_failed_run_without_a_problem_or_the_configuration_reports_the_configuration()
    {
        let observation = observation(Exit::Failed, Vec::new());

        let error = observation.unanswered(Some(Path::new(CONFIGURATION)));

        assert!(
            matches!(
                error,
                Some(ObserveTaploError::UnreportedConfiguration { .. })
            ),
            "expected an unreported configuration, got {error:?}"
        );
    }

    #[test]
    fn unanswered_of_a_failed_run_without_a_problem_reports_an_incomplete_report() {
        let observation = observation(Exit::Failed, Vec::new());

        let error = observation.unanswered(None);

        assert!(
            matches!(error, Some(ObserveTaploError::IncompleteReport { .. })),
            "expected an incomplete report, got {error:?}"
        );
    }

    #[test]
    fn unanswered_of_a_failed_run_without_a_problem_that_reported_the_configuration_reports_an_incomplete_report()
     {
        let observation = Observation {
            configuration: Configuration::Reported,
            ..observation(Exit::Failed, Vec::new())
        };

        let error = observation.unanswered(Some(Path::new(CONFIGURATION)));

        assert!(
            matches!(error, Some(ObserveTaploError::IncompleteReport { .. })),
            "expected an incomplete report, got {error:?}"
        );
    }

    #[test]
    fn unanswered_of_a_passing_run_that_left_out_the_configuration_reports_the_file() {
        let observation = observation(Exit::Succeeded, Vec::new());

        let error = observation.unanswered(Some(Path::new(CONFIGURATION)));

        assert!(
            matches!(&error, Some(ObserveTaploError::UnreportedConfiguration { path, .. }) if path == Path::new(CONFIGURATION)),
            "expected the configuration file, got {error:?}"
        );
    }

    #[test]
    fn unanswered_of_a_passing_run_that_reported_the_configuration_is_none() {
        let observation = Observation {
            configuration: Configuration::Reported,
            ..observation(Exit::Succeeded, Vec::new())
        };

        let error = observation.unanswered(Some(Path::new(CONFIGURATION)));

        assert!(error.is_none(), "expected an answer, got {error:?}");
    }

    #[test]
    fn unanswered_of_a_passing_run_without_a_configuration_file_is_none() {
        let observation = observation(Exit::Succeeded, Vec::new());

        let error = observation.unanswered(None);

        assert!(error.is_none(), "expected an answer, got {error:?}");
    }

    #[tokio::test]
    async fn with_unreadable_files_of_a_failed_run_reports_the_file() {
        let (_directory, readable, missing) = files();
        let observation = examined(vec![readable, missing.clone()], Vec::new());

        let observation = observation.with_unreadable_files().await;

        assert_eq!(observation.problems(), &vec![unreadable(&missing)]);
    }

    #[tokio::test]
    async fn with_unreadable_files_of_a_file_that_a_problem_names_keeps_that_problem() {
        let (_directory, _readable, missing) = files();
        let named = TaploProblem::builder()
            .path(missing.clone())
            .detail(ProblemDetail::Invalid {
                reason: "Permission denied (os error 13)".to_owned(),
            })
            .build();
        let observation = examined(vec![missing], vec![named.clone()]);

        let observation = observation.with_unreadable_files().await;

        assert_eq!(observation.problems(), &vec![named]);
    }

    #[tokio::test]
    async fn with_unreadable_files_of_a_passing_run_reads_nothing() {
        let (_directory, _readable, missing) = files();
        let observation = Observation {
            examined: vec![missing],
            ..observation(Exit::Succeeded, Vec::new())
        };

        let observation = observation.with_unreadable_files().await;

        assert!(observation.problems().is_empty());
    }

    #[tokio::test]
    async fn with_unreadable_files_of_a_report_that_lost_the_line_is_complete() {
        let (directory, readable, missing) = files();
        let stderr = format!(
            " INFO taplo:lint_files:collect_files: found files total=2 excluded=0 files=[{readable:?}, {missing:?}] cwd={:?}\n",
            directory.path()
        );
        let observation = super::report::read()
            .stdout("")
            .stderr(&stderr)
            .succeeded(Exit::Failed)
            .call();

        let observation = observation.with_unreadable_files().await;

        assert!(observation.complete());
    }
}
