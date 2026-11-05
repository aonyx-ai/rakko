//! The validator that a project runs
//!
//! This module holds the program that mise installed for a project and the
//! run that produces its log. An action asks for a run, and everything between
//! the action and the process lives here.

mod error;
pub mod report;

use rakko_action::ProjectRoot;
use rakko_tool::{Invocation, Launch, ResolveToolError, Tool, ToolName};

pub use self::error::ObserveValidatorError;
use self::report::Report;
use crate::observation::Observation;

/// The name that mise knows the program by
const VALIDATOR: &str = "renovate-config-validator";

/// The option that asks the validator for strict validation
///
/// The validator reports a needed migration either way. Without the option it
/// still ends with success. With it, the status of the validator agrees with
/// the outcome of the action. A contributor who runs the validator with the
/// same option gets the same verdict. The option changes no option of the
/// configuration.
const STRICT: &str = "--strict";

/// The variable that selects the format of the log
///
/// The validator writes text for a reader by default. As JSON records, each
/// problem carries the file, the topic, and the message in fields. This
/// replaces a sentence that a reader has to take apart.
const LOG_FORMAT: &str = "LOG_FORMAT";

/// The format that writes one JSON record per line
const JSON: &str = "json";

/// The variable that selects which records the log holds
///
/// A record that reports a migration names no file, and the announcement of
/// the configuration before it is what names it. The level of the
/// announcements is the level of information, so a run asks for that level
/// whatever the environment of the run asks for.
const LOG_LEVEL: &str = "LOG_LEVEL";

/// The level that holds the announcement of each configuration
const INFO: &str = "info";

/// The variable that asks the validator for the regular expressions of
/// JavaScript
///
/// The validator prefers RE2. This is an engine that a native module of its
/// package provides. It falls back to the regular expressions of JavaScript
/// when the module is not built. A warning about the machine appears in every
/// run. A run that asks for the fallback gets the same engine on every
/// machine. No warning appears about it.
///
/// RE2 refuses some patterns that JavaScript accepts, such as a lookahead.
/// So where the module is built, a run that asks for the fallback accepts a
/// pattern that Renovate with RE2 can refuse.
const IGNORE_RE2: &str = "RENOVATE_X_IGNORE_RE2";

/// The variable that names a file for a second copy of the log
///
/// The validator writes its log to the file as well when the environment of
/// the run names one. A relative name puts the file in the project. A run
/// removes the variable, so that it changes no file of the project.
const LOG_FILE: &str = "LOG_FILE";

/// The value that turns a switch of the validator on
const ON: &str = "true";

/// The status of a validator that found nothing wrong, or nothing to validate
const VALID: i32 = 0;

/// The status of a validator that found a problem
const INVALID: i32 = 1;

/// The validator that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned. So a run reaches the same program as the
/// terminal of a contributor. Nothing here installs a tool: provisioning is the
/// job of mise.
/// A validator that mise does not report stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_check_renovate_config::Validator;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// let validator = Validator::resolve(root).await?;
/// let observation = validator.observe().await?;
///
/// println!("{} problems", observation.problems().len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Validator {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Validator {
    /// Runs the validator over the project and reads what it reported
    ///
    /// The run starts in the root of the project and names no configuration,
    /// so the validator finds every configuration of the project itself. It
    /// asks for strict validation, for the log as JSON records at the level
    /// of information, and for the regular expressions of JavaScript. The
    /// validator writes its log on the standard output stream, and a crash on
    /// the standard error stream.
    ///
    /// A run that found no configuration is no failure. The observation
    /// reports it, because the validator reports no problem for such a
    /// project, the same as for one that it validated and found valid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    /// use rakko_check_renovate_config::Validator;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let root = ProjectRoot::new("/home/otter/project".into());
    /// let validator = Validator::resolve(root).await?;
    /// let observation = validator.observe().await?;
    /// assert!(observation.problems().is_empty());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ValidatorUnavailable`][unavailable] when the validator does
    /// not run. Returns [`AbortedValidation`][aborted] when it stopped before
    /// it had validated the project. Returns [`UnreadableReport`][unreadable]
    /// when it wrote a log that this crate cannot read. Returns
    /// [`MissingReport`][missing] when it reported no problem and ended without
    /// success.
    ///
    /// [aborted]: ObserveValidatorError::AbortedValidation
    /// [missing]: ObserveValidatorError::MissingReport
    /// [unavailable]: ObserveValidatorError::ValidatorUnavailable
    /// [unreadable]: ObserveValidatorError::UnreadableReport
    pub async fn observe(&self) -> Result<Observation, ObserveValidatorError> {
        let execution = self
            .invocation()
            .launch()
            .await
            .map_err(|source| ObserveValidatorError::ValidatorUnavailable { source })?;

        answer()
            .code(execution.status().code())
            .stdout(&execution.stdout().to_string_lossy())
            .stderr(&execution.stderr().to_string_lossy())
            .call()
    }

    /// Returns the validator that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names. So
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    /// use rakko_check_renovate_config::Validator;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let root = ProjectRoot::new("/home/otter/project".into());
    /// let _validator = Validator::resolve(root).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no validator for the
    /// project.
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve()
            .name(ToolName::new(VALIDATOR))
            .root(root)
            .call()
            .await?;

        Ok(Self { tool })
    }

    /// Returns the command that runs the validator over the project
    fn invocation(&self) -> Invocation {
        self.tool
            .invocation()
            .arg(STRICT)
            .env(LOG_FORMAT, JSON)
            .env(LOG_LEVEL, INFO)
            .env(IGNORE_RE2, ON)
            .env_remove(LOG_FILE)
    }
}

/// Returns the answer of a run from its status and its two streams
///
/// A run that the validator did not finish stops with what it wrote, and so
/// does a log that ends with a fatal record. A log that this crate cannot read
/// stops as well. A run that ended without success and reported no problem
/// failed for a reason that the validator did not name, so it stops too.
///
/// # Errors
///
/// Returns [`AbortedValidation`][aborted] for a run that the validator did not
/// finish. Returns [`UnreadableReport`][unreadable] for a log that this crate
/// cannot read. Returns [`MissingReport`][missing] for a failed run without a
/// problem.
///
/// [aborted]: ObserveValidatorError::AbortedValidation
/// [missing]: ObserveValidatorError::MissingReport
/// [unreadable]: ObserveValidatorError::UnreadableReport
#[bon::builder]
fn answer(
    #[builder(required)] code: Option<i32>,
    stdout: &str,
    stderr: &str,
) -> Result<Observation, ObserveValidatorError> {
    let Some(status) = status(code) else {
        return Err(ObserveValidatorError::AbortedValidation {
            details: diagnosis().stderr(stderr).stdout(stdout).call(),
        });
    };

    let report =
        self::report::read(stdout).map_err(|source| ObserveValidatorError::UnreadableReport {
            report: stdout.to_owned(),
            source,
        })?;

    let observation = match report {
        Report::Finished(observation) => observation,
        Report::Halted { details } => {
            return Err(ObserveValidatorError::AbortedValidation { details });
        }
    };

    if status == Status::Invalid && observation.problems().is_empty() {
        return Err(ObserveValidatorError::MissingReport {
            details: diagnosis().stderr(stderr).stdout(stdout).call(),
        });
    }

    Ok(observation)
}

/// How a run of the validator that finished ended
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Status {
    /// The validator found nothing wrong, or nothing to validate
    Valid,

    /// The validator found a problem
    Invalid,
}

/// Returns how a run ended, or `None` for a run that the validator did not
/// finish
///
/// The validator ends with success or with a status of one when it finished,
/// and with a status of its own when it crashed. Every other status, and a run
/// that no status ended, belongs to a validator that stopped before it was
/// done.
fn status(code: Option<i32>) -> Option<Status> {
    match code? {
        VALID => Some(Status::Valid),
        INVALID => Some(Status::Invalid),
        _ => None,
    }
}

/// Returns what the validator wrote about a run that gave no answer
///
/// The validator writes its log on the standard output stream and a crash on
/// the standard error stream, so the standard error stream answers first. A
/// run that wrote nothing there leaves the reason in the log, or says nothing
/// at all.
#[bon::builder]
fn diagnosis(stderr: &str, stdout: &str) -> String {
    let written = stderr.trim();

    if written.is_empty() {
        return stdout.trim().to_owned();
    }

    written.to_owned()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "An assertion in a test panics by design, and a test that resolves the validator of this repository expects mise to report it. A `# Panics` section on every test would repeat that and give the reader no information."
    )]
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use super::*;

    /// A log that announces one configuration and reports nothing about it
    const VALID_LOG: &str = "{\"level\":30,\"msg\":\"Validating renovate.json\"}\n\
                             {\"level\":30,\"msg\":\"Config validated successfully against 1 \
                             file(s)\"}\n";

    /// A log that ends with a fatal record
    const FATAL_LOG: &str = "{\"level\":60,\"msg\":\"Could not parse config file\"}\n";

    /// Returns the arguments of a command, as the validator reads them
    fn arguments(invocation: &Invocation) -> Vec<String> {
        invocation
            .arguments()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Returns the variables that a command sets, as `NAME=value`
    fn environment(invocation: &Invocation) -> Vec<String> {
        invocation
            .environment()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Returns the root of the repository that the tests run in
    ///
    /// The repository pins the validator, so the tests resolve it there.
    fn root() -> ProjectRoot {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository");

        ProjectRoot::from(root)
    }

    /// Returns the validator of this repository
    async fn validator() -> Validator {
        Validator::resolve(root())
            .await
            .expect("expected mise to report the validator that this repository pins")
    }

    #[test]
    fn answer_of_a_crash_stops_with_what_the_validator_wrote() {
        let answer = answer()
            .code(Some(99))
            .stdout("")
            .stderr("TypeError: boom\n")
            .call();

        assert!(
            matches!(&answer, Err(ObserveValidatorError::AbortedValidation { details }) if details == "TypeError: boom"),
            "expected the run to stop, got {answer:?}"
        );
    }

    #[test]
    fn answer_of_a_failed_run_without_a_problem_stops() {
        let answer = answer()
            .code(Some(INVALID))
            .stdout(VALID_LOG)
            .stderr("")
            .call();

        assert!(
            matches!(answer, Err(ObserveValidatorError::MissingReport { .. })),
            "expected the run to stop, got {answer:?}"
        );
    }

    #[test]
    fn answer_of_a_fatal_record_stops() {
        let answer = answer()
            .code(Some(INVALID))
            .stdout(FATAL_LOG)
            .stderr("")
            .call();

        assert!(
            matches!(&answer, Err(ObserveValidatorError::AbortedValidation { details }) if details == "Could not parse config file"),
            "expected the run to stop, got {answer:?}"
        );
    }

    #[test]
    fn answer_of_a_valid_run_holds_the_observation() {
        let answer = answer()
            .code(Some(VALID))
            .stdout(VALID_LOG)
            .stderr("")
            .call();

        assert!(
            matches!(&answer, Ok(observation) if observation.problems().is_empty()),
            "expected a valid run, got {answer:?}"
        );
    }

    #[test]
    fn answer_of_an_unreadable_log_stops_with_the_log() {
        let answer = answer()
            .code(Some(VALID))
            .stdout("Config validated\n")
            .stderr("")
            .call();

        assert!(
            matches!(&answer, Err(ObserveValidatorError::UnreadableReport { report, .. }) if report == "Config validated\n"),
            "expected the run to stop, got {answer:?}"
        );
    }

    #[test]
    fn diagnosis_of_a_crash_holds_the_standard_error_stream() {
        let details = diagnosis()
            .stderr("TypeError: boom\n")
            .stdout("{\"level\":30}\n")
            .call();

        assert_eq!(details, "TypeError: boom");
    }

    #[test]
    fn diagnosis_without_an_error_stream_holds_the_log() {
        let details = diagnosis().stderr("").stdout("{\"level\":30}\n").call();

        assert_eq!(details, "{\"level\":30}");
    }

    #[tokio::test]
    async fn invocation_asks_for_json_records_at_the_level_of_information() {
        let invocation = validator().await.invocation();

        assert!(
            ["LOG_FORMAT=json", "LOG_LEVEL=info"]
                .iter()
                .all(|variable| environment(&invocation).iter().any(|set| set == variable)),
            "expected the log as JSON at the level of information, got {:?}",
            environment(&invocation)
        );
    }

    #[tokio::test]
    async fn invocation_asks_for_strict_validation() {
        let invocation = validator().await.invocation();

        assert!(
            arguments(&invocation)
                .iter()
                .any(|argument| argument == STRICT),
            "expected strict validation, got {:?}",
            arguments(&invocation)
        );
    }

    #[tokio::test]
    async fn invocation_asks_for_the_regular_expressions_of_javascript() {
        let invocation = validator().await.invocation();

        assert!(
            environment(&invocation).contains(&format!("{IGNORE_RE2}={ON}")),
            "expected the switch of the engine, got {:?}",
            environment(&invocation)
        );
    }

    #[tokio::test]
    async fn invocation_names_no_configuration() {
        let invocation = validator().await.invocation();

        assert_eq!(arguments(&invocation), [STRICT]);
    }

    #[tokio::test]
    async fn invocation_removes_the_file_of_the_log() {
        let invocation = validator().await.invocation();

        assert_eq!(
            invocation
                .removals()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [LOG_FILE]
        );
    }

    #[tokio::test]
    async fn invocation_runs_the_program_that_mise_reports() {
        let validator = validator().await;

        let invocation = validator.invocation();

        assert_eq!(invocation.program(), validator.tool.program());
    }

    #[test]
    fn status_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Status>();
    }

    #[test]
    fn status_of_a_crash_reports_nothing() {
        let status = status(Some(99));

        assert_eq!(status, None);
    }

    #[test]
    fn status_of_a_run_that_found_a_problem_reports_an_invalid_run() {
        let status = status(Some(INVALID));

        assert_eq!(status, Some(Status::Invalid));
    }

    #[test]
    fn status_of_a_run_that_no_status_ended_reports_nothing() {
        let status = status(None);

        assert_eq!(status, None);
    }

    #[test]
    fn status_of_a_successful_run_reports_a_valid_run() {
        let status = status(Some(VALID));

        assert_eq!(status, Some(Status::Valid));
    }

    #[test]
    fn validator_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Validator>();
    }
}
