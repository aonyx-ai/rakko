//! The tracey that a project runs
//!
//! This module holds the program that mise installed for a project, the look
//! that tells whether tracey has anything to do there, and the questions that
//! an action asks it. An action states which question it wants, and everything
//! between the action and the process lives here.
//!
//! The module judges nothing. It reports what tracey said, and the action that
//! asked decides what the answer means for its outcome.

/// The error that leaves a question without an answer
mod error;
/// The reports that tracey writes as data
mod report;

use std::path::Path;

use rakko_action::ProjectRoot;
use rakko_tool::{Execution, Invocation, ResolveToolError, Tool, ToolName};

pub use self::error::ObserveTraceyError;
pub use self::report::Coverage;
use self::report::{Status, Validation};
use crate::environment::inherited;
use crate::problem::TraceyProblem;

/// The name that mise knows the tool by
const TRACEY: &str = "tracey";

/// The subcommand of tracey that stops the daemon of a workspace
const KILL: &str = "kill";

/// The subcommand of tracey that answers a question about a workspace
const QUERY: &str = "query";

/// The subcommand of tracey that validates the specifications against the code
const VALIDATE: &str = "validate";

/// The subcommand of tracey that reports the coverage of the specifications
const STATUS: &str = "status";

/// The subcommand of tracey that checks a staged change of a requirement
const PRE_COMMIT: &str = "pre-commit";

/// The flag that asks tracey for a report as data
const JSON: &str = "--json";

/// The flag that names the diagnostics that end a run without success
const DENY: &str = "--deny";

/// The diagnostics that a run refuses to pass over
const WARNINGS: &str = "warnings";

/// The file that tracey reads the specifications of a project from
const CONFIGURATION: &str = ".config/tracey/config.styx";

/// The tracey that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a tracey that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_check_specs::Tracey;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let tracey = Tracey::resolve(ProjectRoot::new("/home/otter/project".into())).await?;
///
/// tracey.stop().await?;
/// let problems = tracey.validate().await?;
///
/// println!("{} problems", problems.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Tracey {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Tracey {
    /// Returns whether the project configures tracey
    ///
    /// Tracey reads the specifications of a project from one file, and a
    /// project without that file has none for tracey to check. The look reads
    /// that path and nothing else, so it costs one question to the file
    /// system.
    // checkspecs[impl skip.missing]
    pub async fn applies(root: &ProjectRoot) -> bool {
        tokio::fs::metadata(root.get().join(CONFIGURATION))
            .await
            .is_ok_and(|configuration| configuration.is_file())
    }

    /// Returns the coverage of the specifications of the project
    ///
    /// # Errors
    ///
    /// Returns [`TraceyUnavailable`][unavailable] when tracey does not run,
    /// and [`UnreadableReport`][unreadable] when the report of tracey is not
    /// one that the crate reads.
    ///
    /// [unavailable]: ObserveTraceyError::TraceyUnavailable
    /// [unreadable]: ObserveTraceyError::UnreadableReport
    // checkspecs[impl coverage.summary]
    pub async fn coverage(&self) -> Result<Coverage, ObserveTraceyError> {
        let execution = self.start(Question::Coverage).await?;
        let status: Status = parse(&execution.stdout().to_string_lossy(), || {
            diagnosis(&execution)
        })?;

        Ok(status.coverage())
    }

    /// Returns the tracey that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no tracey for the
    /// project.
    // checkspecs[impl tool.tracey]
    // checkspecs[impl tool.missing]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(TRACEY), root).await?;

        Ok(Self { tool })
    }

    /// Stops the daemon that answers the questions of a workspace
    ///
    /// Tracey answers from a daemon that serves what it scanned, and that scan
    /// can be seconds old. A caller that stops the daemon first gets an answer
    /// about the tree as it is now, because the next question starts a daemon
    /// that scans the workspace before it answers.
    ///
    /// A workspace without a daemon is not a failure. Tracey says so and ends
    /// with success, and the caller wanted no daemon running either way.
    ///
    /// # Errors
    ///
    /// Returns [`TraceyUnavailable`][unavailable] when tracey does not run.
    ///
    /// [unavailable]: ObserveTraceyError::TraceyUnavailable
    // checkspecs[impl daemon.stop]
    // checkspecs[impl daemon.absent]
    pub async fn stop(&self) -> Result<(), ObserveTraceyError> {
        self.start(Question::Daemon).await?;

        Ok(())
    }

    /// Returns the problems that tracey found in the specifications
    ///
    /// The question asks tracey to treat a warning as fatal, so that every
    /// link which no longer holds reaches the caller.
    ///
    /// # Errors
    ///
    /// Returns [`TraceyUnavailable`][unavailable] when tracey does not run,
    /// and [`UnreadableReport`][unreadable] when the report of tracey is not
    /// one that the crate reads.
    ///
    /// [unavailable]: ObserveTraceyError::TraceyUnavailable
    /// [unreadable]: ObserveTraceyError::UnreadableReport
    // checkspecs[impl check.read]
    // checkspecs[impl check.structured]
    // checkspecs[impl check.warnings]
    // checkspecs[impl check.unreadable]
    pub async fn validate(&self) -> Result<Vec<TraceyProblem>, ObserveTraceyError> {
        let execution = self.start(Question::References).await?;
        let validations: Vec<Validation> = parse(&execution.stdout().to_string_lossy(), || {
            diagnosis(&execution)
        })?;

        Ok(validations.iter().flat_map(Validation::problems).collect())
    }

    /// Returns what tracey said about a staged change without a version bump
    ///
    /// Tracey compares the staged content of a specification with the
    /// committed one, in the repository that the directory belongs to. The
    /// caller names that directory, because the comparison that a pull request
    /// needs is not the one that the checkout of a contributor holds.
    ///
    /// A run that found nothing to report answers `None`. A run that found a
    /// change without a bump answers with what tracey wrote about it.
    ///
    /// # Errors
    ///
    /// Returns [`TraceyUnavailable`][unavailable] when tracey does not run.
    ///
    /// [unavailable]: ObserveTraceyError::TraceyUnavailable
    // checkspecs[impl version.staged]
    pub async fn versions(&self, directory: &Path) -> Result<Option<String>, ObserveTraceyError> {
        let execution = self
            .invocation()
            .arg(PRE_COMMIT)
            .arg(directory.as_os_str())
            .run()
            .await
            .map_err(|source| ObserveTraceyError::TraceyUnavailable { source })?;

        if execution.status().success() {
            return Ok(None);
        }

        Ok(Some(diagnosis(&execution)))
    }

    /// Returns the command that runs tracey
    ///
    /// Tracey reads a repository for the question about a staged change, and
    /// it starts git to do so. A run inside a git hook would hand it the
    /// repository of the hook, whatever directory the command names, so the
    /// variables that git reads for itself leave the command.
    // checkspecs[impl version.checkout]
    fn invocation(&self) -> Invocation {
        inherited().fold(self.tool.invocation(), Invocation::env_remove)
    }

    /// Starts tracey with the arguments and collects what it produced
    ///
    /// # Errors
    ///
    /// Returns [`TraceyUnavailable`][unavailable] when tracey does not run.
    ///
    /// [unavailable]: ObserveTraceyError::TraceyUnavailable
    async fn start(&self, question: Question) -> Result<Execution, ObserveTraceyError> {
        self.invocation()
            .args(arguments(question).iter().copied())
            .run()
            .await
            .map_err(|source| ObserveTraceyError::TraceyUnavailable { source })
    }
}

/// A question that an action asks tracey about a project
///
/// One place holds the command line of every question, so an action states
/// what it wants to know and never a flag, and a change to how tracey is
/// called reaches every action at once.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Question {
    /// How much of the specifications the code answers for
    Coverage,

    /// Whether a daemon of the workspace is still running
    Daemon,

    /// Whether every reference points at a requirement that exists
    References,
}

/// Returns the command line of one question to tracey
// checkspecs[impl check.structured]
// checkspecs[impl check.warnings]
// checkspecs[impl coverage.summary]
// checkspecs[impl daemon.stop]
fn arguments(question: Question) -> &'static [&'static str] {
    match question {
        Question::Coverage => &[QUERY, JSON, STATUS],
        Question::Daemon => &[KILL],
        Question::References => &[QUERY, JSON, VALIDATE, DENY, WARNINGS],
    }
}

/// Returns what tracey wrote about a run that it ended without success
///
/// Tracey writes the reason to one stream and the hint that follows it to
/// another, depending on the question. Both belong to the answer, so the
/// diagnosis holds whatever arrived.
fn diagnosis(execution: &Execution) -> String {
    let mut streams = [execution.stdout(), execution.stderr()]
        .into_iter()
        .map(|stream| stream.to_string_lossy().trim().to_owned())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if streams.is_empty() {
        streams = "tracey wrote nothing about it".to_owned();
    }

    streams
}

/// Returns the report of tracey, read into the shape of the crate
///
/// The diagnosis is built only for a report that the crate cannot read, so a
/// run that tracey answered pays nothing for the text that a failure needs.
///
/// # Errors
///
/// Returns [`UnreadableReport`][unreadable] when the report is not one that
/// the crate reads.
///
/// [unreadable]: ObserveTraceyError::UnreadableReport
// checkspecs[impl check.unreadable]
fn parse<T: serde::de::DeserializeOwned>(
    report: &str,
    diagnosis: impl FnOnce() -> String,
) -> Result<T, ObserveTraceyError> {
    serde_json::from_str(report).map_err(|source| ObserveTraceyError::UnreadableReport {
        report: diagnosis(),
        source,
    })
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // checkspecs[verify check.structured]
    #[test]
    fn arguments_of_every_report_ask_for_data() {
        let reports = [Question::Coverage, Question::References];

        let structured = reports
            .into_iter()
            .all(|question| arguments(question).contains(&JSON));

        assert!(structured, "expected every report to be asked for as data");
    }

    // checkspecs[verify check.warnings]
    #[test]
    fn arguments_of_a_validation_deny_warnings() {
        let selected = arguments(Question::References);

        assert_eq!(
            selected,
            ["query", "--json", "validate", "--deny", "warnings"]
        );
    }

    // checkspecs[verify daemon.stop]
    #[test]
    fn arguments_of_a_daemon_question_stop_it() {
        let selected = arguments(Question::Daemon);

        assert_eq!(selected, ["kill"]);
    }

    // checkspecs[verify check.unreadable]
    #[test]
    fn parse_of_a_report_that_is_not_data_holds_what_tracey_wrote() {
        let report = "tracey: something new happened";

        let outcome = parse::<Status>(report, || report.to_owned());

        assert!(
            matches!(&outcome, Err(ObserveTraceyError::UnreadableReport { report: held, .. }) if held == report),
            "expected the report of tracey, got {outcome:?}"
        );
    }

    // A scheduler runs actions in parallel and can move a run to a different
    // thread, so a tracey that an action holds across a wait travels with it.
    #[test]
    fn tracey_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<Tracey>();
    }
}
