//! The action that checks the Renovate configuration of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps the validator of Renovate as a subprocess: the validator finds the
//! configurations, reads them, and reports what Renovate would refuse, and the
//! action translates what the validator reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Name, Outcome, ProjectRoot, SkipReason, Summary, action_name,
};

pub use self::error::CheckRenovateConfigError;
use crate::problem::RenovateProblem;
use crate::validator::Validator;

/// The action that checks the Renovate configuration of a project
///
/// The action wraps `renovate-config-validator`, the validator that [Renovate]
/// ships: the validator finds the configurations of the project, reads each
/// of them, and reports every option that Renovate would refuse, so a run
/// agrees with a contributor that runs the validator bare. The validator that
/// runs is the one that [mise] installed for the project, at the version that
/// the project pinned, and the action installs nothing.
///
/// A run only reports, and it takes no argument. A run asks for strict
/// validation, so a configuration that needs a migration fails the run like an
/// error and a warning do. Each error and each warning becomes a finding at
/// the file that the validator named, with the topic and the message of the
/// validator, and a configuration that needs a migration becomes one finding
/// that names the options that the migration changes.
///
/// The action applies to a project that holds a Renovate configuration, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// validator, when the validator stops before it has validated the project,
/// and when the validator writes a log that the action cannot read.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_check_renovate_config::CheckRenovateConfig;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckRenovateConfig)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [renovate]: https://docs.renovatebot.com
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckRenovateConfig;

impl Action for CheckRenovateConfig {
    // checkrenovateconfig[impl args.none]
    type Args = ();

    // checkrenovateconfig[impl name]
    fn name(&self) -> Name {
        action_name!("check-renovate-config")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // checkrenovateconfig[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run resolves the validator and validates the project with it. An error
/// that this function returns stops the run, and the caller reports it in the
/// outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, the run of the validator, or the reading of its log.
async fn drive(context: &Context) -> Result<Outcome, CheckRenovateConfigError> {
    // checkrenovateconfig[impl tool.missing]
    // checkrenovateconfig[impl tool.validator]
    let validator = Validator::resolve(context.root().clone())
        .await
        .map_err(|source| CheckRenovateConfigError::UnresolvedTool { source })?;

    // checkrenovateconfig[impl check.read]
    let observation = validator.observe().await?;

    // checkrenovateconfig[impl skip.unconfigured]
    if let Some(words) = observation.unconfigured() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(words.clone()),
        });
    }

    // checkrenovateconfig[impl check.passed]
    // checkrenovateconfig[impl check.summary]
    if observation.problems().is_empty() {
        return Ok(Outcome::Passed {
            summary: Some(summary(observation.validated())),
        });
    }

    Ok(Outcome::Failed {
        findings: findings(observation.problems(), context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one problem of a configuration
///
/// The finding sits at the position that the validator named, or at the whole
/// file where it named none. The message comes from the validator, so a reader
/// of a finding reads what the tool itself would have told them.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// file of the problem.
///
/// [foreign]: CheckRenovateConfigError::ForeignPath
// checkrenovateconfig[impl check.error]
// checkrenovateconfig[impl check.migration]
// checkrenovateconfig[impl check.unparsable]
// checkrenovateconfig[impl check.warning]
fn finding(
    problem: &RenovateProblem,
    root: &ProjectRoot,
) -> Result<Finding, CheckRenovateConfigError> {
    let location = problem
        .location(root)
        .ok_or_else(|| CheckRenovateConfigError::ForeignPath {
            path: problem.path().clone(),
        })?;

    Ok(Finding::builder()
        .message(problem.message())
        .location(location)
        .build())
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// file of a problem.
///
/// [foreign]: CheckRenovateConfigError::ForeignPath
fn findings(
    problems: &[RenovateProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, CheckRenovateConfigError> {
    problems
        .iter()
        .map(|problem| finding(problem, root))
        .collect()
}

/// Returns the summary of a run that validated the given number of
/// configurations
// checkrenovateconfig[impl check.summary]
fn summary(validated: u32) -> Summary {
    if validated == 1 {
        Summary::new("validated 1 configuration")
    } else {
        Summary::new(format!("validated {validated} configurations"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use rakko_action::{Location, Position};
    use rakko_test_utils::path;

    use super::*;

    /// Returns an error of the validator about the given path
    fn problem(path: PathBuf) -> RenovateProblem {
        RenovateProblem::new(
            path,
            None,
            "Configuration Error".to_owned(),
            "Invalid configuration option: foo".to_owned(),
        )
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let error = finding(&problem(path("/elsewhere/config.js")), &root()).unwrap_err();

        assert!(matches!(
            error,
            CheckRenovateConfigError::ForeignPath { .. }
        ));
    }

    // checkrenovateconfig[verify check.unparsable]
    #[test]
    fn finding_of_a_problem_at_a_position_sits_at_the_position() {
        let position = Position::builder().line(2).column(1).build();
        let problem = RenovateProblem::new(
            path("renovate.json"),
            Some(position),
            "File could not be parsed".to_owned(),
            "JSON5: invalid end of input at 2:1".to_owned(),
        );

        let finding = finding(&problem, &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "renovate.json".parse().unwrap(),
                position,
            }
        );
    }

    // checkrenovateconfig[verify check.error]
    #[test]
    fn finding_of_a_problem_carries_the_message_of_the_validator() {
        let finding = finding(&problem(path("renovate.json")), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "Configuration Error: Invalid configuration option: foo"
        );
    }

    // checkrenovateconfig[verify check.error]
    #[test]
    fn findings_report_every_problem_of_the_observation() {
        let problems = vec![problem(path("renovate.json")), problem(path(".renovaterc"))];

        let findings = findings(&problems, &root()).unwrap();

        assert_eq!(findings.len(), 2);
    }

    // checkrenovateconfig[verify check.summary]
    #[test]
    fn summary_of_one_configuration_names_one_configuration() {
        let summary = summary(1);

        assert_eq!(summary.get(), "validated 1 configuration");
    }

    // checkrenovateconfig[verify check.summary]
    #[test]
    fn summary_of_several_configurations_names_their_number() {
        let summary = summary(3);

        assert_eq!(summary.get(), "validated 3 configurations");
    }
}
