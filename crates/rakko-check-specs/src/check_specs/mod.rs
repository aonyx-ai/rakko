//! The action that checks the specifications of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps tracey as a subprocess: tracey reads the specifications and the code
//! that answers them, and the action asks it the questions that gate a project
//! and translates the answers into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, SkipReason, Summary, action_name,
};

pub use self::error::CheckSpecsError;
use crate::comparison::Comparison;
use crate::problem::TraceyProblem;
use crate::tracey::{Coverage, Tracey};

/// The reason of a run in a project that configures no tracey
const NO_CONFIGURATION: &str = "the project holds no .config/tracey/config.styx";

/// The variable that names the base branch of a pull request
///
/// A code host sets it for a job that checks a pull request, and nothing sets
/// it in the checkout of a contributor.
const BASE_REF: &str = "GITHUB_BASE_REF";

/// The action that checks the specifications of a project
///
/// The action wraps [tracey]. A specification states what a crate does as a
/// list of requirements, each with an identifier, and the code that implements
/// or verifies a requirement names that identifier in a comment. Tracey holds
/// the two together, and the tracey that runs is the one that [mise] installed
/// for the project, at the version that the project pinned.
///
/// A run asks three questions. Does every reference point at a requirement
/// that exists, at the version it carries now? Does a change to the text of a
/// requirement carry the version bump that such a change needs? And how much
/// of the specification does the code answer for? The first two gate the run,
/// and the third travels in the summary of a run that passed, because a
/// specification may land before the code that answers it.
///
/// A run only reports, and it takes no argument. It changes nothing about the
/// project, and nothing about the repository of the project: the comparison
/// that a pull request needs is built in a copy.
///
/// The action applies to a project that configures tracey, and it skips
/// visibly otherwise.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_check_specs::CheckSpecs;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckSpecs)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [tracey]: https://tracey.bearcove.eu/
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckSpecs;

impl Action for CheckSpecs {
    // checkspecs[impl args.none]
    type Args = ();

    // checkspecs[impl name]
    fn name(&self) -> Name {
        action_name!("check-specs")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // checkspecs[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves tracey, and asks it the questions
/// that gate a project. An error that this function returns stops the run, and
/// the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, a question that tracey left unanswered, or the comparison that a pull
/// request needs.
async fn drive(context: &Context) -> Result<Outcome, CheckSpecsError> {
    // checkspecs[impl skip.missing]
    if !Tracey::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_CONFIGURATION),
        });
    }

    // checkspecs[impl tool.tracey]
    // checkspecs[impl tool.missing]
    let tracey = Tracey::resolve(context.root().clone())
        .await
        .map_err(|source| CheckSpecsError::UnresolvedTool { source })?;

    // checkspecs[impl daemon.stop]
    // checkspecs[impl daemon.absent]
    tracey.stop().await?;

    // checkspecs[impl check.read]
    let problems = tracey.validate().await?;
    let mut findings = findings(&problems)?;

    // checkspecs[impl version.base]
    // checkspecs[impl version.checkout]
    let pull_request = std::env::var_os(BASE_REF).is_some();
    let comparison = Comparison::prepare(context.root(), pull_request).await?;
    let directory = comparison
        .as_ref()
        .map_or(context.root().get(), Comparison::directory);

    // checkspecs[impl version.staged]
    if let Some(details) = tracey.versions(directory).await? {
        findings.push(
            Finding::builder()
                .message(details)
                .location(Location::Project)
                .build(),
        );
    }

    if !findings.is_empty() {
        return Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        });
    }

    // checkspecs[impl coverage.open]
    // checkspecs[impl coverage.summary]
    Ok(Outcome::Passed {
        summary: Some(summary(tracey.coverage().await?)),
    })
}

/// Returns the finding that reports one problem of the project
///
/// Every message comes from tracey, so a reader of a finding reads what the
/// tool itself would have told them.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the path of the problem is not one
/// that a finding can name.
///
/// [foreign]: CheckSpecsError::ForeignPath
// checkspecs[impl check.diagnostic]
fn finding(problem: &TraceyProblem) -> Result<Finding, CheckSpecsError> {
    let path =
        problem
            .path()
            .as_str()
            .try_into()
            .map_err(|source| CheckSpecsError::ForeignPath {
                path: problem.path().clone(),
                source,
            })?;

    Ok(Finding::builder()
        .message(problem.message().clone())
        .location(Location::Position {
            path,
            position: Position::builder()
                .line(*problem.line())
                .column(*problem.column())
                .build(),
        })
        .build())
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the path of a problem is not one that
/// a finding can name.
///
/// [foreign]: CheckSpecsError::ForeignPath
fn findings(problems: &[TraceyProblem]) -> Result<Vec<Finding>, CheckSpecsError> {
    problems.iter().map(finding).collect()
}

/// Returns the summary that tells how much of the specifications is answered
///
/// A pass needs the counts, so that a reader can question a pass over a
/// project whose specifications nothing answers yet.
// checkspecs[impl coverage.summary]
fn summary(coverage: Coverage) -> Summary {
    let requirements = if coverage.total == 1 {
        "1 requirement".to_owned()
    } else {
        format!("{} requirements", coverage.total)
    };

    Summary::new(format!(
        "checked {requirements}, {} implemented and {} verified",
        coverage.covered, coverage.verified
    ))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // checkspecs[verify coverage.summary]
    #[test]
    fn summary_of_one_requirement_says_so() {
        let summary = summary(Coverage {
            total: 1,
            covered: 1,
            verified: 1,
        });

        assert_eq!(
            summary.get(),
            "checked 1 requirement, 1 implemented and 1 verified"
        );
    }

    // checkspecs[verify coverage.summary]
    #[test]
    fn summary_reports_the_counts_of_the_project() {
        let summary = summary(Coverage {
            total: 15,
            covered: 14,
            verified: 12,
        });

        assert_eq!(
            summary.get(),
            "checked 15 requirements, 14 implemented and 12 verified"
        );
    }
}
