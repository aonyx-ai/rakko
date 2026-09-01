//! The action that formats the TOML files of a project
//!
//! This module holds the action, the arguments that a run reads, and the
//! error that stops a run. The action wraps taplo as a subprocess: taplo
//! discovers the files, reads its own configuration, and does the
//! formatting, and the action translates what taplo reported into an
//! outcome.

/// The arguments that a run of the action reads
mod args;
/// The error that stops a run of the action
mod error;

use std::collections::HashSet;
use std::path::PathBuf;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason, Summary,
    action_name,
};
use rakko_taplo::{Observation, Operation, ProblemDetail, Taplo, TaploProblem};

pub use self::args::FormatTomlArgs;
pub use self::error::FormatTomlError;

/// The reason of a run that found no TOML file
const NO_TOML_FILES: &str = "the project holds no file with the .toml extension";

/// The message of a finding about a file that is not formatted
const UNFORMATTED_FINDING: &str = "the file is not properly formatted";

/// The message of a repair of a file that was not formatted
const UNFORMATTED_REPAIR: &str = "the file was not properly formatted";

/// The action that formats the TOML files of a project
///
/// The action wraps [taplo]: taplo discovers the TOML files of the project,
/// reads its own configuration, and formats what its configuration selects,
/// so a run agrees with an editor and with a contributor that runs taplo
/// bare. The taplo that runs is the one that [mise] installed for the
/// project, at the version that the project pinned, and the action installs
/// nothing.
///
/// A run reports by default. A file that is not formatted becomes a finding
/// that names the file, and a file that taplo cannot parse becomes a finding
/// at the position that taplo reports. With the `fix` argument, taplo
/// rewrites the files that it can format, and the outcome carries one repair
/// for each file that it rewrote, next to the problems that remain.
///
/// The action applies to a project that holds TOML files, and it skips
/// visibly otherwise. A run stops with an error when mise reports no taplo,
/// when taplo rejects a configuration file of the project, and when taplo
/// writes a report that the action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_format_toml::FormatToml;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(FormatToml)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [taplo]: https://taplo.tamasfe.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct FormatToml;

impl Action for FormatToml {
    type Args = FormatTomlArgs;

    // formattoml[impl name]
    fn name(&self) -> Name {
        action_name!("format-toml")
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args).await {
            Ok(outcome) => outcome,
            // formattoml[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves taplo, checks, and fixes when the
/// arguments ask for it. An error that this function returns stops the run,
/// and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, a taplo run, or the reading of the report.
async fn drive(context: &Context, args: &FormatTomlArgs) -> Result<Outcome, FormatTomlError> {
    // formattoml[impl skip.git]
    // formattoml[impl skip.links]
    // formattoml[impl skip.missing]
    if !Taplo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_TOML_FILES),
        });
    }

    // formattoml[impl tool.taplo]
    // formattoml[impl tool.missing]
    let taplo = Taplo::resolve(context.root().clone())
        .await
        .map_err(|source| FormatTomlError::UnresolvedTool { source })?;

    // formattoml[impl check.read]
    let observation = taplo.observe(Operation::CheckFormat).await?;

    // formattoml[impl check.configuration]
    rejected(&observation)?;

    if observation.problems().is_empty() {
        return passed(&observation);
    }

    if args.fix() {
        fix(&taplo, observation.problems(), context.root()).await
    } else {
        // formattoml[impl check.unformatted]
        // formattoml[impl check.invalid]
        Ok(Outcome::Failed {
            findings: findings(observation.problems(), context.root())?,
            repairs: Vec::new(),
        })
    }
}

/// Returns the finding that reports one problem of the project
///
/// A file that is not formatted gets a finding at the level of the file, and
/// a diagnostic gets one at the position that taplo named. The message for
/// an unformatted file comes from the caller, because a finding states a
/// problem that the project has and a repair states one that the run took
/// away.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of the problem.
///
/// [foreign]: FormatTomlError::ForeignPath
fn finding(
    problem: &TaploProblem,
    root: &ProjectRoot,
    unformatted_message: &str,
) -> Result<Finding, FormatTomlError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| FormatTomlError::ForeignPath {
            path: problem.path().clone(),
        })?;

    let finding = match problem.detail() {
        ProblemDetail::Diagnostic {
            line,
            column,
            message,
        } => Finding::builder()
            .message(message.clone())
            .location(Location::Position {
                path,
                position: Position::builder().line(*line).column(*column).build(),
            })
            .build(),
        ProblemDetail::Invalid { reason } => Finding::builder()
            .message(reason.clone())
            .location(Location::File { path })
            .build(),
        ProblemDetail::Unformatted => Finding::builder()
            .message(unformatted_message)
            .location(Location::File { path })
            .build(),
    };

    Ok(finding)
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of a problem.
///
/// [foreign]: FormatTomlError::ForeignPath
fn findings(
    problems: &[TaploProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, FormatTomlError> {
    problems
        .iter()
        .map(|problem| finding(problem, root, UNFORMATTED_FINDING))
        .collect()
}

/// Repairs the problems that the check found and reports what remains
///
/// Taplo does not report which files a rewrite touched, so the repairs
/// derive from the two reports: a problem of the check whose file the
/// rewrite does not report again was repaired, and what the rewrite still
/// reports remains. The files that taplo cannot parse remain this way,
/// because a rewrite repairs formatting and a syntax error needs a hand.
///
/// # Errors
///
/// Returns the error of a rewrite that could not finish: taplo did not run,
/// it rejected a configuration file, it wrote a report that the action does
/// not recognize, or it reported a path outside the project.
// formattoml[impl fix.write]
async fn fix(
    taplo: &Taplo,
    problems: &[TaploProblem],
    root: &ProjectRoot,
) -> Result<Outcome, FormatTomlError> {
    let observation = taplo.observe(Operation::Format).await?;

    // formattoml[impl check.configuration]
    rejected(&observation)?;

    // formattoml[impl check.unrecognized]
    if observation.problems().is_empty() && !observation.succeeded() {
        return Err(unrecognized(&observation));
    }

    let remaining: HashSet<&PathBuf> = observation
        .problems()
        .iter()
        .map(TaploProblem::path)
        .collect();
    let repaired = problems
        .iter()
        .filter(|problem| !remaining.contains(problem.path()));
    let repairs = repaired
        .map(|problem| finding(problem, root, UNFORMATTED_REPAIR))
        .collect::<Result<Vec<Finding>, FormatTomlError>>()?;
    let findings = findings(observation.problems(), root)?;

    if findings.is_empty() {
        // formattoml[impl fix.changed]
        Ok(Outcome::Changed { repairs })
    } else {
        // formattoml[impl fix.partial]
        Ok(Outcome::Failed { findings, repairs })
    }
}

/// Returns the outcome of a check that reported no problem
///
/// A pass needs the count of the files that taplo checked, so that a reader
/// can question a pass that examined nothing. A check that ended without
/// success, and a check that passed without the count, both wrote a report
/// that the action does not recognize, and the run stops instead of hiding
/// problems behind a green result.
///
/// # Errors
///
/// Returns [`UnrecognizedReport`][unrecognized] when the check ended without
/// success or reported no count of the files.
///
/// [unrecognized]: FormatTomlError::UnrecognizedReport
// formattoml[impl check.passed]
// formattoml[impl check.unrecognized]
fn passed(observation: &Observation) -> Result<Outcome, FormatTomlError> {
    if !observation.succeeded() {
        return Err(unrecognized(observation));
    }

    let Some(checked) = observation.checked() else {
        return Err(unrecognized(observation));
    };

    Ok(Outcome::Passed {
        summary: Some(summary(checked)),
    })
}

/// Stops a run whose taplo rejected a configuration file of the project
///
/// Taplo warns and then runs with its defaults, and a run on the defaults
/// quietly does what the project asked it not to do.
///
/// # Errors
///
/// Returns [`RejectedConfiguration`][rejected] when taplo rejected a
/// configuration file.
///
/// [rejected]: FormatTomlError::RejectedConfiguration
// formattoml[impl check.configuration]
fn rejected(observation: &Observation) -> Result<(), FormatTomlError> {
    match observation.rejected_configuration() {
        Some(details) => Err(FormatTomlError::RejectedConfiguration {
            details: details.clone(),
        }),
        None => Ok(()),
    }
}

/// Returns the summary that tells how many files taplo checked
// formattoml[impl check.passed]
fn summary(checked: u64) -> Summary {
    if checked == 1 {
        Summary::new("checked 1 file")
    } else {
        Summary::new(format!("checked {checked} files"))
    }
}

/// Returns the error of a report that the action cannot answer from
// formattoml[impl check.unrecognized]
fn unrecognized(observation: &Observation) -> FormatTomlError {
    FormatTomlError::UnrecognizedReport {
        stderr: observation.stderr().clone(),
    }
}
