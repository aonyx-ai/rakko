//! The action that audits the GitHub Actions workflows of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps zizmor as a subprocess: zizmor collects the files, reads its own
//! configuration, and applies its audits, and the action translates what
//! zizmor reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, ProjectRoot, SkipReason, action_name,
};

pub use self::error::LintGitHubActionsError;
use crate::problem::ZizmorProblem;
use crate::zizmor::Zizmor;

/// The reason of a run that found no workflow
const NO_WORKFLOWS: &str =
    "the .github/workflows directory of the project holds no .yaml or .yml file";

/// The reason of a run whose zizmor collected nothing
///
/// The look of the action found a workflow, and zizmor then collected none.
/// The configuration of the project, and the ignore rules of the version
/// control system, explain the difference.
const NOTHING_TO_AUDIT: &str = "zizmor found no file to audit";

/// The action that audits the GitHub Actions workflows of a project
///
/// The action wraps [zizmor]: zizmor collects the workflows of the project,
/// reads its own configuration, and applies the audits that it knows, so a run
/// agrees with an editor and with a contributor that runs zizmor bare. The
/// zizmor that runs is the one that [mise] installed for the project, at the
/// version that the project pinned, and the action installs nothing.
///
/// A run only reports, and it takes no argument. A finding of zizmor names one
/// or more places of a workflow, and each of those places becomes a finding of
/// the run at the range that zizmor named. The message holds the severity, the
/// audit, and the words that zizmor wrote about the place. A finding that
/// zizmor calls informational fails a run like one that it calls high, because
/// a project that wants an audit to stay quiet turns that audit off.
///
/// A run asks zizmor for the pedantic persona, which reports the code smells
/// of a workflow as well as the findings that zizmor is confident about.
/// Zizmor takes a persona on its command line alone, so a project cannot ask
/// for one in its configuration file. A run also asks zizmor to stop at a file
/// that it collected and cannot read, instead of dropping that file with a
/// warning that no outcome carries.
///
/// The action applies to a project that holds GitHub Actions workflows, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// zizmor, when zizmor stops before it has audited the project, and when
/// zizmor writes a report that the action cannot read.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_lint_github_actions::LintGitHubActions;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintGitHubActions)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [zizmor]: https://docs.zizmor.sh
#[derive(Copy, Clone, Debug, Default)]
pub struct LintGitHubActions;

impl Action for LintGitHubActions {
    // lintgithubactions[impl args.none]
    type Args = ();

    // lintgithubactions[impl name]
    fn name(&self) -> Name {
        action_name!("lint-github-actions")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // lintgithubactions[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves zizmor, and audits the project with
/// it. An error that this function returns stops the run, and the caller
/// reports it in the outcome.
///
/// A passing run carries no summary. Zizmor names no count in its report, and
/// it collects more than the workflows that the look of the action counted, so
/// a count from the action would speak about a different set of files than the
/// run examined.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, the zizmor run, or the reading of the report.
async fn drive(context: &Context) -> Result<Outcome, LintGitHubActionsError> {
    // lintgithubactions[impl skip.links]
    // lintgithubactions[impl skip.missing]
    if !Zizmor::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_WORKFLOWS),
        });
    }

    // lintgithubactions[impl tool.missing]
    // lintgithubactions[impl tool.zizmor]
    let zizmor = Zizmor::resolve(context.root().clone())
        .await
        .map_err(|source| LintGitHubActionsError::UnresolvedTool { source })?;

    // lintgithubactions[impl check.read]
    let observation = zizmor.observe().await?;

    // lintgithubactions[impl skip.uncollected]
    if !observation.collected() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NOTHING_TO_AUDIT),
        });
    }

    // lintgithubactions[impl check.passed]
    if observation.problems().is_empty() {
        return Ok(Outcome::Passed { summary: None });
    }

    // lintgithubactions[impl check.finding]
    // lintgithubactions[impl check.severity]
    Ok(Outcome::Failed {
        findings: findings(observation.problems(), context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one place of a workflow
///
/// The finding covers the range that zizmor named, so a reader and a code host
/// see the part of the file that zizmor underlines. The message comes from
/// zizmor, so a reader of a finding reads what the tool itself would have told
/// them, including the severity that the audit carries.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of the problem.
///
/// [foreign]: LintGitHubActionsError::ForeignPath
// lintgithubactions[impl check.finding]
// lintgithubactions[impl check.severity]
fn finding(problem: &ZizmorProblem, root: &ProjectRoot) -> Result<Finding, LintGitHubActionsError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| LintGitHubActionsError::ForeignPath {
            path: problem.path().clone(),
        })?;

    Ok(Finding::builder()
        .message(problem.message())
        .location(Location::Span {
            path,
            span: problem.span(),
        })
        .build())
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of a problem.
///
/// [foreign]: LintGitHubActionsError::ForeignPath
fn findings(
    problems: &[ZizmorProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, LintGitHubActionsError> {
    problems
        .iter()
        .map(|problem| finding(problem, root))
        .collect()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use rakko_action::{Position, Span};

    use super::*;
    use crate::problem::Severity;

    /// Returns a problem of the given severity in a workflow of the project
    fn problem(severity: Severity) -> ZizmorProblem {
        ZizmorProblem::new(
            PathBuf::from("./.github/workflows/ci.yml"),
            span(),
            severity,
            "template-injection".to_owned(),
            "code injection via template expansion".to_owned(),
            "may expand into attacker-controllable code".to_owned(),
        )
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns the range that the problems of a test cover
    fn span() -> Span {
        Span::builder()
            .start(Position::builder().line(8).column(24).build())
            .end(Position::builder().line(8).column(48).build())
            .build()
    }

    // lintgithubactions[verify check.finding]
    #[test]
    fn finding_of_a_problem_carries_the_message_of_zizmor() {
        let finding = finding(&problem(Severity::High), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "[high] template-injection: code injection via template expansion \
             (may expand into attacker-controllable code)"
        );
    }

    // lintgithubactions[verify check.finding]
    #[test]
    fn finding_of_a_problem_covers_the_range_of_zizmor() {
        let finding = finding(&problem(Severity::High), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Span {
                path: ".github/workflows/ci.yml".parse().unwrap(),
                span: span(),
            }
        );
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = ZizmorProblem::new(
            PathBuf::from("/elsewhere/ci.yml"),
            span(),
            Severity::High,
            "unpinned-uses".to_owned(),
            "unpinned action reference".to_owned(),
            "action is not pinned to a hash".to_owned(),
        );

        let error = finding(&problem, &root()).unwrap_err();

        assert!(matches!(error, LintGitHubActionsError::ForeignPath { .. }));
    }

    // lintgithubactions[verify check.severity]
    #[test]
    fn finding_of_an_informational_problem_names_the_informational_severity() {
        let finding = finding(&problem(Severity::Informational), &root()).unwrap();

        assert!(
            finding.message().get().starts_with("[informational] "),
            "expected the severity of zizmor, got {}",
            finding.message().get()
        );
    }

    // lintgithubactions[verify check.finding]
    #[test]
    fn findings_report_every_place_of_the_observation() {
        let problems = vec![problem(Severity::High), problem(Severity::Low)];

        let findings = findings(&problems, &root()).unwrap();

        assert_eq!(findings.len(), 2);
    }
}
