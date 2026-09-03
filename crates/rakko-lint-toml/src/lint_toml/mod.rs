//! The action that validates the TOML files of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps taplo as a subprocess: taplo discovers the files, reads its own
//! configuration, and does the validation, and the action translates what
//! taplo reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason, Summary,
    action_name,
};
use rakko_taplo::{Observation, Operation, ProblemDetail, Taplo, TaploProblem};

pub use self::error::LintTomlError;

/// The reason of a run that found no TOML file
const NO_TOML_FILES: &str = "the project holds no file with the .toml extension";

/// The message of a finding about a file that is not formatted
///
/// A validation never reports one. The message exists because the problems
/// of taplo carry every level that any of its operations can name, and a
/// finding that says what taplo meant is better than one that guesses.
const UNFORMATTED: &str = "the file is not properly formatted";

/// The action that validates the TOML files of a project
///
/// The action wraps [taplo]: taplo discovers the TOML files of the project,
/// reads its own configuration, and validates what its configuration
/// selects, so a run agrees with an editor and with a contributor that runs
/// taplo bare. The taplo that runs is the one that [mise] installed for the
/// project, at the version that the project pinned, and the action installs
/// nothing.
///
/// A run only reports, and it takes no argument. A file that taplo cannot
/// parse, and a file whose content its schema refuses, become findings at
/// the position that taplo reports. A file that taplo cannot open becomes a
/// finding that names the file and carries the reason of taplo, because
/// taplo never read a character of it.
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
/// use rakko_lint_toml::LintToml;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintToml)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [taplo]: https://taplo.tamasfe.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct LintToml;

impl Action for LintToml {
    // linttoml[impl args.none]
    type Args = ();

    // linttoml[impl name]
    fn name(&self) -> Name {
        action_name!("lint-toml")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // linttoml[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves taplo, and validates. An error
/// that this function returns stops the run, and the caller reports it in
/// the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, the taplo run, or the reading of the report.
async fn drive(context: &Context) -> Result<Outcome, LintTomlError> {
    // linttoml[impl skip.git]
    // linttoml[impl skip.links]
    // linttoml[impl skip.missing]
    if !Taplo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_TOML_FILES),
        });
    }

    // linttoml[impl tool.taplo]
    // linttoml[impl tool.missing]
    let taplo = Taplo::resolve(context.root().clone())
        .await
        .map_err(|source| LintTomlError::UnresolvedTool { source })?;

    // linttoml[impl check.read]
    let observation = taplo.observe(Operation::Lint).await?;

    // linttoml[impl check.configuration]
    if let Some(details) = observation.rejected_configuration() {
        return Err(LintTomlError::RejectedConfiguration {
            details: details.clone(),
        });
    }

    if observation.problems().is_empty() {
        return Ok(passed(&observation));
    }

    // linttoml[impl check.diagnostic]
    // linttoml[impl check.refused]
    Ok(Outcome::Failed {
        findings: findings(observation.problems(), context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one problem of the project
///
/// A problem that taplo placed in a file gets a finding at that position,
/// and a problem that taplo could only attach to a file gets one at the
/// level of the file. Every message comes from taplo, so a reader of a
/// finding reads what the tool itself would have told them.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of the problem.
///
/// [foreign]: LintTomlError::ForeignPath
fn finding(problem: &TaploProblem, root: &ProjectRoot) -> Result<Finding, LintTomlError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| LintTomlError::ForeignPath {
            path: problem.path().clone(),
        })?;

    let (message, location) = match problem.detail() {
        ProblemDetail::Diagnostic {
            line,
            column,
            message,
        } => (
            message.clone(),
            Location::Position {
                path,
                position: Position::builder().line(*line).column(*column).build(),
            },
        ),
        ProblemDetail::Invalid { reason } => (reason.clone(), Location::File { path }),
        ProblemDetail::Unformatted => (UNFORMATTED.to_owned(), Location::File { path }),
    };

    Ok(Finding::builder()
        .message(message)
        .location(location)
        .build())
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of a problem.
///
/// [foreign]: LintTomlError::ForeignPath
fn findings(problems: &[TaploProblem], root: &ProjectRoot) -> Result<Vec<Finding>, LintTomlError> {
    problems
        .iter()
        .map(|problem| finding(problem, root))
        .collect()
}

/// Returns the outcome of a run that reported no problem
///
/// A pass says how many files taplo checked, so that a reader can question a
/// pass that examined nothing. Taplo can lose the line that carries the
/// count, and a pass without it stays a pass: taplo ended the run with
/// success, and no line that a report loses can turn a run that found
/// problems into one that found none.
// linttoml[impl check.passed+2]
fn passed(observation: &Observation) -> Outcome {
    Outcome::Passed {
        summary: observation.checked().map(summary),
    }
}

/// Returns the summary that tells how many files taplo checked
// linttoml[impl check.passed+2]
fn summary(checked: u64) -> Summary {
    if checked == 1 {
        Summary::new("checked 1 file")
    } else {
        Summary::new(format!("checked {checked} files"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // linttoml[verify check.passed+2]
    #[test]
    fn passed_of_a_run_that_counted_one_file_says_so() {
        let observation = Observation::builder().checked(1).succeeded(true).build();

        let outcome = passed(&observation);

        assert!(
            matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "checked 1 file"),
            "expected the count of the files, got {outcome:?}"
        );
    }

    // linttoml[verify check.passed+2]
    #[test]
    fn passed_of_a_run_without_a_count_says_nothing() {
        let observation = Observation::builder().succeeded(true).build();

        let outcome = passed(&observation);

        assert!(
            matches!(&outcome, Outcome::Passed { summary: None }),
            "expected a pass without a summary, got {outcome:?}"
        );
    }
}
