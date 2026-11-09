//! The tracey that a project runs
//!
//! This module holds the version that the crate declares, the program that
//! mise installed at that version, and the questions that an action asks
//! it. An action states which question it wants, and everything between the
//! action and the process lives here.
//!
//! The module judges nothing. It reports what tracey said, and the action that
//! asked decides what the answer means for its outcome.

mod error;
mod report;

use std::path::Path;

use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
use rakko_tool::{Execution, Invocation, Launch, MiseFile, ResolveRequirementError, Tool};

pub use self::error::ObserveTraceyError;
pub(crate) use self::report::Coverage;
use self::report::{Status, Validation};
use crate::environment::inherited;
use crate::problem::TraceyProblem;

/// The name that mise knows the tool by
///
/// Tracey is in no registry of mise. So the name holds the backend that
/// builds a crate from a Git repository, and the address of that repository.
const TOOL: &str = "cargo:https://github.com/bearcove/tracey";

/// The program of the tool that a run starts
const TRACEY: &str = "tracey";

/// The version of tracey that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(TOOL);

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

/// The tracey that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it. Nothing here
/// installs a tool: provisioning is the job of mise, and a tracey that mise
/// does not report stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Tracey {
    /// The program that mise installed at the declared version
    tool: Tool,
}

impl Tracey {
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
    pub(crate) async fn coverage(&self) -> Result<Coverage, ObserveTraceyError> {
        let execution = self.start(Question::Coverage).await?;
        let status: Status = parse(&execution.stdout().to_string_lossy(), || {
            diagnosis(&execution)
        })?;

        Ok(status.coverage())
    }

    /// Returns the requirement of the tracey that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(TOOL)
            .executable(TRACEY)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the tracey that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of tracey in the project does not change the answer.
    /// A run of tracey starts in the root of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, or when
    /// it reports no tracey at that version, for example because nothing
    /// installed it.
    ///
    /// [requirement]: Tracey::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
            .context(context)
            .call()
            .await?;

        Ok(Self { tool })
    }

    /// Stops the daemon that answers the questions of a workspace
    ///
    /// Tracey answers from a daemon that serves what it scanned. That scan can
    /// be seconds old. A caller that stops the daemon first gets an answer
    /// about the tree as it is now. This works because the next question starts
    /// a daemon that scans the workspace before it answers.
    ///
    /// A workspace without a daemon is not a failure. Tracey says so and ends
    /// with success, and the caller wanted no daemon running either way.
    ///
    /// # Errors
    ///
    /// Returns [`TraceyUnavailable`][unavailable] when tracey does not run.
    ///
    /// [unavailable]: ObserveTraceyError::TraceyUnavailable
    pub(crate) async fn stop(&self) -> Result<(), ObserveTraceyError> {
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
    pub(crate) async fn validate(&self) -> Result<Vec<TraceyProblem>, ObserveTraceyError> {
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
    pub(crate) async fn versions(
        &self,
        directory: &Path,
    ) -> Result<Option<String>, ObserveTraceyError> {
        let execution = self
            .invocation()
            .arg(PRE_COMMIT)
            .arg(directory.as_os_str())
            .launch()
            .await
            .map_err(|source| ObserveTraceyError::TraceyUnavailable { source })?;

        if execution.status().success() {
            return Ok(None);
        }

        Ok(Some(diagnosis(&execution)))
    }

    /// Returns the command that runs tracey
    ///
    /// Tracey reads a repository for the question about a staged change and
    /// starts git to do so. A run inside a git hook would hand it the
    /// repository of the hook, whatever directory the command names. So the
    /// variables that git reads for itself leave the command.
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
            .launch()
            .await
            .map_err(|source| ObserveTraceyError::TraceyUnavailable { source })
    }
}

/// A question that an action asks tracey about a project
///
/// One place holds the command line of every question. So an action states what
/// it wants to know and never a flag. A change to how tracey is called reaches
/// every action at once.
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
/// The diagnosis is built only for a report that the crate cannot read. So a
/// run that tracey answered pays nothing for the text that a failure needs.
///
/// # Errors
///
/// Returns [`UnreadableReport`][unreadable] when the report is not one that
/// the crate reads.
///
/// [unreadable]: ObserveTraceyError::UnreadableReport
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn arguments_of_a_daemon_question_stop_it() {
        let selected = arguments(Question::Daemon);

        assert_eq!(selected, ["kill"]);
    }

    #[test]
    fn arguments_of_a_validation_deny_warnings() {
        let selected = arguments(Question::References);

        assert_eq!(
            selected,
            ["query", "--json", "validate", "--deny", "warnings"]
        );
    }

    #[test]
    fn arguments_of_every_report_ask_for_data() {
        let reports = [Question::Coverage, Question::References];

        let structured = reports
            .into_iter()
            .all(|question| arguments(question).contains(&JSON));

        assert!(structured, "expected every report to be asked for as data");
    }

    #[test]
    fn parse_of_a_report_that_is_not_data_holds_what_tracey_wrote() {
        let report = "tracey: something new happened";

        let outcome = parse::<Status>(report, || report.to_owned());

        assert!(
            matches!(&outcome, Err(ObserveTraceyError::UnreadableReport { report: held, .. }) if held == report),
            "expected the report of tracey, got {outcome:?}"
        );
    }

    #[test]
    fn question_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Question>();
    }

    /// Shows that tracey is send and sync
    ///
    /// A scheduler runs actions in parallel and can move a run to a different
    /// thread. So a tracey that an action holds across a wait travels with it.
    #[test]
    fn tracey_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<Tracey>();
    }

    #[test]
    fn tracey_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Tracey>();
    }
}
