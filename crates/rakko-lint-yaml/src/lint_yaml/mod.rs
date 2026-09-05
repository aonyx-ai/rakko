//! The action that lints the YAML files of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps yamllint as a subprocess: yamllint discovers the files, reads its
//! own configuration, and applies its rules, and the action translates what
//! yamllint reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason, Summary,
    action_name,
};

pub use self::error::LintYamlError;
use crate::observation::Observation;
use crate::problem::YamllintProblem;
use crate::yamllint::Yamllint;

/// The reason of a run that found no YAML file
const NO_YAML_FILES: &str =
    "the project holds no file with the .yaml or .yml extension, and no .yamllint";

/// The reason of a run whose yamllint examines nothing
///
/// The look of the action found a file, and yamllint then collected none. The
/// configuration of the project explains the difference.
const NOTHING_TO_EXAMINE: &str = "yamllint found no YAML file to examine";

/// The action that lints the YAML files of a project
///
/// The action wraps [yamllint]: yamllint discovers the YAML files of the
/// project, reads its own configuration, and applies the rules that the
/// project turned on, so a run agrees with an editor and with a contributor
/// that runs yamllint bare. The yamllint that runs is the one that [mise]
/// installed for the project, at the version that the project pinned, and the
/// action installs nothing.
///
/// A run only reports, and it takes no argument. Every rule that a file broke
/// becomes a finding at the position that yamllint named, and the message of
/// the finding is the sentence that yamllint wrote for a reader. A problem
/// that yamllint calls a warning becomes a finding like one that it calls an
/// error, because a project that wants a rule to stay quiet turns that rule
/// off.
///
/// The action applies to a project that holds YAML files, and it skips
/// visibly otherwise. A run stops with an error when mise reports no
/// yamllint, when yamllint refuses the configuration of the project, and when
/// yamllint writes a report that the action cannot read.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_lint_yaml::LintYaml;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintYaml)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [yamllint]: https://github.com/adrienverge/yamllint
#[derive(Copy, Clone, Debug, Default)]
pub struct LintYaml;

impl Action for LintYaml {
    // lintyaml[impl args.none]
    type Args = ();

    // lintyaml[impl name]
    fn name(&self) -> Name {
        action_name!("lint-yaml")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // lintyaml[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves yamllint, asks which files yamllint
/// examines, and lints them. An error that this function returns stops the
/// run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, the listing of the files, the yamllint run, or the reading of the
/// report.
async fn drive(context: &Context) -> Result<Outcome, LintYamlError> {
    // lintyaml[impl skip.hidden]
    // lintyaml[impl skip.links]
    // lintyaml[impl skip.missing]
    if !Yamllint::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_YAML_FILES),
        });
    }

    // lintyaml[impl tool.missing]
    // lintyaml[impl tool.yamllint]
    let yamllint = Yamllint::resolve(context.root().clone())
        .await
        .map_err(|source| LintYamlError::UnresolvedTool { source })?;

    // lintyaml[impl check.configuration]
    // lintyaml[impl run.listing]
    let examined = yamllint.list().await?;

    // lintyaml[impl skip.unexamined]
    if examined.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NOTHING_TO_EXAMINE),
        });
    }

    // lintyaml[impl check.read]
    let observation = yamllint.observe().await?;

    if let Some(outcome) = guard(&observation, examined.len())? {
        return Ok(outcome);
    }

    // lintyaml[impl check.level]
    // lintyaml[impl check.problem]
    Ok(Outcome::Failed {
        findings: findings(observation.problems(), context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one rule that a file broke
///
/// The finding sits at the position that yamllint named. The message comes
/// from yamllint, so a reader of a finding reads what the tool itself would
/// have told them, including the level that the project gave the rule.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of the problem.
///
/// [foreign]: LintYamlError::ForeignPath
// lintyaml[impl check.level]
// lintyaml[impl check.problem]
fn finding(problem: &YamllintProblem, root: &ProjectRoot) -> Result<Finding, LintYamlError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| LintYamlError::ForeignPath {
            path: problem.path().clone(),
        })?;

    let position = Position::builder()
        .line(problem.line())
        .column(problem.column())
        .build();

    Ok(Finding::builder()
        .message(problem.message())
        .location(Location::Position { path, position })
        .build())
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of a problem.
///
/// [foreign]: LintYamlError::ForeignPath
fn findings(
    problems: &[YamllintProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, LintYamlError> {
    problems
        .iter()
        .map(|problem| finding(problem, root))
        .collect()
}

/// Returns what a run reports when it cannot answer from its findings
///
/// Two conditions end a run before its findings matter. A yamllint that
/// stopped before it examined every file leaves the action with an answer
/// about a part of the project, and an outcome built on it would hide the
/// rest behind that part. A run that reported no rule found a clean project,
/// and it says how many files it examined for that answer.
///
/// Returns `None` when the caller reports what the run found.
///
/// # Errors
///
/// Returns [`IncompleteExamination`][incomplete] when yamllint stopped before
/// it examined every file.
///
/// [incomplete]: LintYamlError::IncompleteExamination
fn guard(observation: &Observation, examined: usize) -> Result<Option<Outcome>, LintYamlError> {
    // lintyaml[impl check.incomplete]
    if !observation.finished() {
        return Err(LintYamlError::IncompleteExamination {
            details: observation.stderr().clone(),
        });
    }

    if !observation.problems().is_empty() {
        return Ok(None);
    }

    // lintyaml[impl check.passed]
    Ok(Some(Outcome::Passed {
        summary: Some(summary(examined)),
    }))
}

/// Returns the summary of a run that found no problem
///
/// The summary names how many files yamllint examined, so that a reader can
/// question a pass that examined less than they expect. The configuration of
/// a project can exclude a file that the reader believes is checked, and the
/// count is where that shows.
// lintyaml[impl check.passed]
fn summary(examined: usize) -> Summary {
    if examined == 1 {
        Summary::new("checked 1 file")
    } else {
        Summary::new(format!("checked {examined} files"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;
    use crate::problem::ProblemLevel;

    /// Returns a problem of the given level in a file of the project
    fn problem(level: ProblemLevel) -> YamllintProblem {
        YamllintProblem::new(
            PathBuf::from("./notes.yaml"),
            4,
            9,
            level,
            "truthy value should be one of [false, true] (truthy)".to_owned(),
        )
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    // lintyaml[verify check.problem]
    #[test]
    fn finding_of_a_problem_carries_the_message_of_yamllint() {
        let finding = finding(&problem(ProblemLevel::Error), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "[error] truthy value should be one of [false, true] (truthy)"
        );
    }

    // lintyaml[verify check.problem]
    #[test]
    fn finding_of_a_problem_sits_at_the_position_of_yamllint() {
        let finding = finding(&problem(ProblemLevel::Error), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "notes.yaml".parse().unwrap(),
                position: Position::builder().line(4).column(9).build(),
            }
        );
    }

    // lintyaml[verify check.level]
    #[test]
    fn finding_of_a_warning_names_the_warning_level() {
        let finding = finding(&problem(ProblemLevel::Warning), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "[warning] truthy value should be one of [false, true] (truthy)"
        );
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = YamllintProblem::new(
            PathBuf::from("/elsewhere/notes.yaml"),
            1,
            1,
            ProblemLevel::Error,
            "trailing spaces (trailing-spaces)".to_owned(),
        );

        let error = finding(&problem, &root()).unwrap_err();

        assert!(matches!(error, LintYamlError::ForeignPath { .. }));
    }

    // lintyaml[verify check.passed]
    #[test]
    fn guard_of_a_clean_run_passes() {
        let outcome = guard(&Observation::builder().build(), 3).unwrap();

        assert!(
            matches!(outcome, Some(Outcome::Passed { .. })),
            "expected the run to pass, got {outcome:?}"
        );
    }

    // lintyaml[verify check.passed]
    #[test]
    fn guard_of_a_clean_run_says_how_many_files_yamllint_examined() {
        let outcome = guard(&Observation::builder().build(), 3).unwrap();

        let Some(Outcome::Passed { summary: Some(it) }) = outcome else {
            panic!("expected a summary, got {outcome:?}");
        };
        assert_eq!(it.get(), "checked 3 files");
    }

    // lintyaml[verify check.incomplete]
    #[test]
    fn guard_of_a_run_that_stopped_early_stops() {
        let observation = Observation::builder().finished(false).build();

        let error = guard(&observation, 3).unwrap_err();

        assert!(matches!(error, LintYamlError::IncompleteExamination { .. }));
    }

    // lintyaml[verify check.level]
    #[test]
    fn guard_of_a_run_that_reported_a_warning_reports_nothing() {
        let observation = Observation::builder()
            .problems(vec![problem(ProblemLevel::Warning)])
            .build();

        let outcome = guard(&observation, 3).unwrap();

        assert!(outcome.is_none(), "expected no outcome, got {outcome:?}");
    }

    // lintyaml[verify check.passed]
    #[test]
    fn summary_of_a_single_file_names_it_in_the_singular() {
        let summary = summary(1);

        assert_eq!(summary.get(), "checked 1 file");
    }
}
