//! The stiles that a project runs
//!
//! This module holds the program that mise installed for a project, and the
//! two runs that the action asks of it: the listing of the rules that the
//! project runs, and the check of the project. An action asks for a run, and
//! everything between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the reports that stiles wrote
mod report;

use rakko_action::ProjectRoot;
use rakko_tool::{Execution, ResolveToolError, Tool, ToolName};

pub use self::error::ObserveStilesError;
use crate::observation::Observation;
use crate::problem::{Severity, StilesProblem};

/// The name that mise knows the tool by
const STILES: &str = "stiles";

/// The command of stiles that lists the rules of the project
const RULES: &str = "rules";

/// The command of stiles that checks the project
const CHECK: &str = "check";

/// The option that asks stiles for its report as data
///
/// Stiles writes a block of source per diagnostic by default, with the rule
/// and the message drawn around it. The same run writes one object per
/// diagnostic on request, which carries the rule, the severity, the file, and
/// the position in fields instead. The option selects the presentation of the
/// report and not the behavior of the tool.
const JSON: &str = "--json";

/// The place that a run tells stiles to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and stiles reports every path relative to it.
const HERE: &str = ".";

/// The text that opens the message of an error that ended a run
const RUN_ERROR: &str = "Error:";

/// The text that opens the message of a file that stiles could not check
const FILE_ERROR: &str = "error:";

/// The words of the error that stiles ends a run with when it found no file
///
/// Stiles ends a run that found nothing to check with an error, and not with
/// a pass, so the action reads its words to tell that run from one that
/// stopped for a reason of its own. The words leave out the name of the
/// program, which the error opens with.
const NOTHING_TO_CHECK: &str = "analyzed no files under";

/// The details of a run that ended without success and explained nothing
const NO_DIAGNOSIS: &str = "stiles ended without success and reported no error";

/// The status of a stiles that checked the project and found no error
const CLEAN: i32 = 0;

/// The status of a stiles that failed the run
///
/// Stiles ends with this status when a diagnostic fails the run, and also
/// when it stopped before it had checked the project. Only the report and the
/// standard error stream tell the two apart.
const FAILED: i32 = 1;

/// The stiles that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a stiles that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_lint_stiles::Stiles;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// let stiles = Stiles::resolve(root).await?;
///
/// if !stiles.enabled_rules().await?.is_empty() {
///     let observation = stiles.observe().await?;
///
///     println!("{} problems", observation.problems().len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Stiles {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Stiles {
    /// Returns the identifiers of the rules that the project runs
    ///
    /// The run asks stiles to list the rules that the lints of its
    /// configuration declare, and it keeps the rules that the configuration
    /// does not turn off. Stiles loads the lints to list their rules, so it
    /// builds a lint that it has not built yet, as a check would.
    ///
    /// A project that configures no lint gets no rule. Stiles checks such a
    /// project with nothing, and a check of it passes whatever the code holds,
    /// so the answer tells a caller that a check would examine nothing.
    ///
    /// # Errors
    ///
    /// Returns [`StilesUnavailable`][unavailable] when stiles does not run,
    /// [`IncompleteCheck`][incomplete] when stiles cannot read its
    /// configuration or load a lint, and [`UnreadableReport`][unreadable]
    /// when it wrote a listing that this crate cannot read.
    ///
    /// [incomplete]: ObserveStilesError::IncompleteCheck
    /// [unavailable]: ObserveStilesError::StilesUnavailable
    /// [unreadable]: ObserveStilesError::UnreadableReport
    pub async fn enabled_rules(&self) -> Result<Vec<String>, ObserveStilesError> {
        let execution = self.run(RULES).await?;

        if !execution.status().success() {
            return Err(incomplete(&execution));
        }

        let stdout = execution.stdout().to_string_lossy();

        self::report::enabled_rules(&stdout).map_err(|source| {
            ObserveStilesError::UnreadableReport {
                report: stdout.to_string(),
                source,
            }
        })
    }

    /// Checks the project with stiles and reads what it reported
    ///
    /// The run names the root of the project and asks for the report as data.
    /// It leaves the threshold of stiles as it is, so stiles ends without
    /// success only for an error, and the observation carries every
    /// diagnostic that the run reported, whatever its severity.
    ///
    /// A run that found no file to check is no failure. The observation
    /// reports it, because stiles reports no diagnostic for a project that it
    /// checked and found clean either.
    ///
    /// # Errors
    ///
    /// Returns [`StilesUnavailable`][unavailable] when stiles does not run,
    /// [`IncompleteCheck`][incomplete] when stiles stopped before it had
    /// checked the project, and [`UnreadableReport`][unreadable] when it
    /// wrote a report that this crate cannot read.
    ///
    /// [incomplete]: ObserveStilesError::IncompleteCheck
    /// [unavailable]: ObserveStilesError::StilesUnavailable
    /// [unreadable]: ObserveStilesError::UnreadableReport
    pub async fn observe(&self) -> Result<Observation, ObserveStilesError> {
        let execution = self.run(CHECK).await?;
        let stderr = execution.stderr().to_string_lossy();

        if found_nothing(&stderr) {
            return Ok(Observation::builder().examined(false).build());
        }

        if stopped(&stderr) {
            return Err(incomplete(&execution));
        }

        let stdout = execution.stdout().to_string_lossy();

        let problems = self::report::problems(&stdout).map_err(|source| {
            ObserveStilesError::UnreadableReport {
                report: stdout.to_string(),
                source,
            }
        })?;

        if !finished(execution.status().code(), &problems) {
            return Err(incomplete(&execution));
        }

        Ok(Observation::builder().problems(problems).build())
    }

    /// Returns the stiles that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no stiles for the
    /// project.
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(STILES), root).await?;

        Ok(Self { tool })
    }

    /// Runs a command of stiles over the project and asks for data
    ///
    /// # Errors
    ///
    /// Returns [`StilesUnavailable`][unavailable] when stiles does not run.
    ///
    /// [unavailable]: ObserveStilesError::StilesUnavailable
    async fn run(&self, command: &str) -> Result<Execution, ObserveStilesError> {
        self.tool
            .invocation()
            .arg(command)
            .arg(JSON)
            .arg(HERE)
            .run()
            .await
            .map_err(|source| ObserveStilesError::StilesUnavailable { source })
    }
}

/// Returns whether a check ended with an answer that its report holds
///
/// Stiles ends with success when no diagnostic fails the run, and with its
/// failure status when one does. It ends with the same failure status when it
/// stopped, so a failure status counts as an answer only when the report
/// holds a diagnostic that stiles fails a run for. Every other status, and a
/// run that no status ended, belongs to a stiles that stopped.
fn finished(status: Option<i32>, problems: &[StilesProblem]) -> bool {
    match status {
        Some(CLEAN) => true,
        Some(FAILED) => problems
            .iter()
            .any(|problem| problem.severity() == Severity::Error),
        _ => false,
    }
}

/// Returns whether stiles ended the run because it found no file to check
fn found_nothing(stderr: &str) -> bool {
    stderr
        .lines()
        .any(|line| line.starts_with(RUN_ERROR) && line.contains(NOTHING_TO_CHECK))
}

/// Returns the error of a run that stopped, with what stiles wrote about it
fn incomplete(execution: &Execution) -> ObserveStilesError {
    let stderr = execution.stderr().to_string_lossy();

    ObserveStilesError::IncompleteCheck {
        details: diagnosis(&stderr),
    }
}

/// Returns what stiles wrote about a run that it stopped
///
/// Stiles builds the lints of the project before it checks anything, and the
/// log of that build shares the standard error stream with the errors of the
/// run. The diagnosis starts at the first error, so that a reader sees the
/// reason of the stop and not the log in front of it. A stream without an
/// error is the whole diagnosis.
fn diagnosis(stderr: &str) -> String {
    let mut offset = 0;
    let mut start = None;

    for line in stderr.split_inclusive('\n') {
        if is_error(line) {
            start = Some(offset);
            break;
        }

        offset += line.len();
    }

    let text = stderr[start.unwrap_or(0)..].trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

/// Returns whether a line of the standard error stream opens with an error
fn is_error(line: &str) -> bool {
    line.starts_with(RUN_ERROR) || line.starts_with(FILE_ERROR)
}

/// Returns whether stiles wrote an error on its standard error stream
///
/// Stiles writes its diagnostics on the standard output stream when a run asks
/// for data, so a line of the standard error stream that opens with an error
/// is never a diagnostic. It is the error that ended the run, or a file that
/// stiles could not read or analyze. Either way the report describes less than
/// the project.
fn stopped(stderr: &str) -> bool {
    stderr.lines().any(is_error)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use rakko_action::Position;

    use super::*;

    /// What stiles writes when it found no file to check
    const NOTHING: &str = "Error: stiles analyzed no files under .: either nothing there is \
                           written in a language stiles has a grammar for, or the ignore files \
                           and configured patterns excluded all of it\n";

    /// What stiles writes when it cannot read its configuration
    const BROKEN_CONFIGURATION: &str = "Error: failed to load the stiles configuration\n\n\
                                        Caused by:\n    0: unknown field `bogus`\n";

    /// What stiles writes about a file that it could not analyze
    const FILE_FAILURE: &str = "error: ./src/lib.rs: failed to read the file: invalid UTF-8\n";

    /// What cargo writes while stiles builds a lint
    const BUILD_LOG: &str =
        "   Compiling todo_comment v0.1.0\n    Finished `release` profile [optimized] target(s)\n";

    /// Returns a diagnostic of the given severity
    fn problem(severity: Severity) -> StilesProblem {
        StilesProblem::new(
            PathBuf::from("./src/lib.rs"),
            Position::builder().line(7).column(5).build(),
            severity,
            "lint.todo-comment".to_owned(),
            "a TODO comment marks unfinished work".to_owned(),
        )
    }

    #[test]
    fn diagnosis_of_a_run_after_a_build_starts_at_the_error() {
        let diagnosis = diagnosis(&format!("{BUILD_LOG}{FILE_FAILURE}"));

        assert_eq!(diagnosis, FILE_FAILURE.trim());
    }

    #[test]
    fn diagnosis_of_a_run_without_an_error_is_the_whole_stream() {
        let diagnosis = diagnosis(BUILD_LOG);

        assert_eq!(diagnosis, BUILD_LOG.trim());
    }

    #[test]
    fn diagnosis_of_a_silent_run_says_so() {
        let diagnosis = diagnosis("");

        assert_eq!(diagnosis, NO_DIAGNOSIS);
    }

    #[test]
    fn finished_of_a_clean_run_is_true() {
        assert!(finished(Some(CLEAN), &[]));
    }

    #[test]
    fn finished_of_a_failed_run_with_an_error_is_true() {
        assert!(finished(Some(FAILED), &[problem(Severity::Error)]));
    }

    #[test]
    fn finished_of_a_failed_run_with_only_a_warning_is_false() {
        assert!(!finished(Some(FAILED), &[problem(Severity::Warning)]));
    }

    #[test]
    fn finished_of_a_failed_run_without_a_diagnostic_is_false() {
        assert!(!finished(Some(FAILED), &[]));
    }

    #[test]
    fn finished_of_a_run_that_no_status_ended_is_false() {
        assert!(!finished(None, &[problem(Severity::Error)]));
    }

    #[test]
    fn finished_of_an_unknown_status_is_false() {
        assert!(!finished(Some(101), &[problem(Severity::Error)]));
    }

    #[test]
    fn found_nothing_of_a_broken_configuration_is_false() {
        assert!(!found_nothing(BROKEN_CONFIGURATION));
    }

    #[test]
    fn found_nothing_of_a_run_without_a_file_is_true() {
        assert!(found_nothing(NOTHING));
    }

    #[test]
    fn stopped_of_a_broken_configuration_is_true() {
        assert!(stopped(BROKEN_CONFIGURATION));
    }

    #[test]
    fn stopped_of_a_build_log_is_false() {
        assert!(!stopped(BUILD_LOG));
    }

    #[test]
    fn stopped_of_a_file_that_stiles_could_not_analyze_is_true() {
        assert!(stopped(FILE_FAILURE));
    }
}
