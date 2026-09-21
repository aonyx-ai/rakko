//! The action that lints the TypeScript of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps oxlint as a subprocess: oxlint discovers the files, reads its own
//! configuration, and applies its rules, and the action translates what oxlint
//! reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason, Summary,
    action_name,
};

pub use self::error::LintTypeScriptError;
use crate::observation::Observation;
use crate::oxlint::Oxlint;
use crate::problem::OxlintProblem;

/// The reason of a run whose oxlint examined nothing
///
/// Oxlint counts the files that it examined, so this is what a project without
/// TypeScript reports. A project whose configuration ignores every one of its
/// files reports the same way.
const NOTHING_TO_EXAMINE: &str = "oxlint found no file to examine";

/// The action that lints the TypeScript of a project
///
/// The action wraps [oxlint]: oxlint discovers the files of the project, reads
/// its own configuration, and applies the rules that the project turned on, so
/// a run agrees with an editor and with a contributor that runs oxlint bare.
/// The oxlint that runs is the one that [mise] installed for the project, at
/// the version that the project pinned, and the action installs nothing.
///
/// A run only reports, and it takes no argument. Every rule that a file broke
/// becomes a finding at the place that oxlint marked, and the message of the
/// finding is the sentence that oxlint wrote for a reader. A diagnostic that
/// oxlint weighs as a warning becomes a finding like one that it weighs as an
/// error, because a project that wants a rule to stay quiet turns that rule
/// off.
///
/// The action applies to a project that holds files which oxlint lints, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// oxlint, when oxlint writes no report at all, and when oxlint writes a report
/// that the action cannot read.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_lint_typescript::LintTypeScript;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintTypeScript)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [oxlint]: https://oxc.rs/docs/guide/usage/linter.html
#[derive(Copy, Clone, Debug, Default)]
pub struct LintTypeScript;

impl Action for LintTypeScript {
    // linttypescript[impl args.none]
    type Args = ();

    // linttypescript[impl name]
    fn name(&self) -> Name {
        action_name!("lint-typescript")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // linttypescript[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run resolves oxlint, lints the project with it, and turns the report
/// into an outcome. An error that this function returns stops the run, and the
/// caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, the oxlint run, or the reading of the report.
async fn drive(context: &Context) -> Result<Outcome, LintTypeScriptError> {
    // linttypescript[impl tool.missing]
    // linttypescript[impl tool.oxlint]
    let oxlint = Oxlint::resolve(context.root().clone())
        .await
        .map_err(|source| LintTypeScriptError::UnresolvedTool { source })?;

    // linttypescript[impl check.read]
    let observation = oxlint.observe().await?;

    if let Some(outcome) = guard(&observation) {
        return Ok(outcome);
    }

    // linttypescript[impl check.diagnostic]
    // linttypescript[impl check.severity]
    Ok(Outcome::Failed {
        findings: findings(observation.problems(), context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one rule that a file broke
///
/// The finding sits at the place that oxlint marked. The message comes from
/// oxlint, so a reader of a finding reads what the tool itself would have told
/// them, including the severity that the project gave the rule.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of the diagnostic.
///
/// [foreign]: LintTypeScriptError::ForeignPath
// linttypescript[impl check.diagnostic]
// linttypescript[impl check.severity]
fn finding(problem: &OxlintProblem, root: &ProjectRoot) -> Result<Finding, LintTypeScriptError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| LintTypeScriptError::ForeignPath {
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

/// Returns the findings that report the given diagnostics
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of a diagnostic.
///
/// [foreign]: LintTypeScriptError::ForeignPath
fn findings(
    problems: &[OxlintProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, LintTypeScriptError> {
    problems
        .iter()
        .map(|problem| finding(problem, root))
        .collect()
}

/// Returns what a run reports when it cannot answer from its findings
///
/// Two answers of oxlint end a run before its findings matter. A run that
/// examined no file found no TypeScript to lint, which is not a clean project
/// but an action that does not apply. A run that examined files and reported no
/// rule found a clean project, and it says how many files it examined for that
/// answer.
///
/// Returns `None` when the caller reports what the run found.
fn guard(observation: &Observation) -> Option<Outcome> {
    // linttypescript[impl skip.unexamined]
    if observation.examined() == 0 {
        return Some(Outcome::Skipped {
            reason: SkipReason::new(NOTHING_TO_EXAMINE),
        });
    }

    if !observation.problems().is_empty() {
        return None;
    }

    // linttypescript[impl check.passed]
    Some(Outcome::Passed {
        summary: Some(summary(observation.examined())),
    })
}

/// Returns the summary of a run that found no problem
///
/// The summary names how many files oxlint examined, so that a reader can
/// question a pass that examined less than they expect. The configuration of a
/// project can ignore a file that the reader believes is checked, and the count
/// is where that shows.
// linttypescript[impl check.passed]
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

    use rakko_test_utils::path;

    use super::*;
    use crate::problem::Severity;

    /// Returns a diagnostic of the given severity in a file of the project
    fn problem(severity: Severity) -> OxlintProblem {
        OxlintProblem::builder()
            .path(path("src/index.ts"))
            .line(2)
            .column(3)
            .severity(severity)
            .code("eslint(no-debugger)")
            .description("`debugger` statement is not allowed")
            .help("Remove the debugger statement")
            .build()
    }

    /// The root that the diagnostics of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    // linttypescript[verify check.diagnostic]
    #[test]
    fn finding_of_a_diagnostic_carries_the_message_of_oxlint() {
        let finding = finding(&problem(Severity::Error), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "[error] eslint(no-debugger): `debugger` statement is not allowed \
             help: Remove the debugger statement"
        );
    }

    // linttypescript[verify check.diagnostic]
    #[test]
    fn finding_of_a_diagnostic_sits_at_the_position_of_oxlint() {
        let finding = finding(&problem(Severity::Error), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "src/index.ts".parse().unwrap(),
                position: Position::builder().line(2).column(3).build(),
            }
        );
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = OxlintProblem::builder()
            .path(path("/elsewhere/index.ts"))
            .line(1)
            .column(1)
            .severity(Severity::Warning)
            .code("eslint(no-var)")
            .description("Unexpected var, use let or const instead")
            .build();

        let error = finding(&problem, &root()).unwrap_err();

        assert!(matches!(error, LintTypeScriptError::ForeignPath { .. }));
    }

    // linttypescript[verify check.severity]
    #[test]
    fn finding_of_a_warning_names_the_warning_severity() {
        let finding = finding(&problem(Severity::Warning), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "[warning] eslint(no-debugger): `debugger` statement is not allowed \
             help: Remove the debugger statement"
        );
    }

    // linttypescript[verify check.passed]
    #[test]
    fn guard_of_a_clean_run_passes() {
        let outcome = guard(&Observation::builder().examined(3).build());

        assert!(
            matches!(outcome, Some(Outcome::Passed { .. })),
            "expected the run to pass, got {outcome:?}"
        );
    }

    // linttypescript[verify check.passed]
    #[test]
    fn guard_of_a_clean_run_says_how_many_files_oxlint_examined() {
        let outcome = guard(&Observation::builder().examined(3).build());

        let Some(Outcome::Passed { summary: Some(it) }) = outcome else {
            panic!("expected a summary, got {outcome:?}");
        };
        assert_eq!(it.get(), "checked 3 files");
    }

    // linttypescript[verify skip.unexamined]
    #[test]
    fn guard_of_a_run_that_examined_nothing_names_the_tool() {
        let outcome = guard(&Observation::builder().build());

        let Some(Outcome::Skipped { reason }) = outcome else {
            panic!("expected the run to skip, got {outcome:?}");
        };
        assert!(
            reason.get().contains("oxlint"),
            "expected the reason to name the tool, got {reason:?}"
        );
    }

    // linttypescript[verify skip.unexamined]
    #[test]
    fn guard_of_a_run_that_examined_nothing_skips() {
        let outcome = guard(&Observation::builder().build());

        assert!(
            matches!(outcome, Some(Outcome::Skipped { .. })),
            "expected the run to skip, got {outcome:?}"
        );
    }

    // linttypescript[verify check.severity]
    #[test]
    fn guard_of_a_run_that_reported_a_warning_reports_nothing() {
        let observation = Observation::builder()
            .problems(vec![problem(Severity::Warning)])
            .examined(3)
            .build();

        let outcome = guard(&observation);

        assert!(outcome.is_none(), "expected no outcome, got {outcome:?}");
    }

    // linttypescript[verify check.passed]
    #[test]
    fn summary_of_a_single_file_names_it_in_the_singular() {
        let summary = summary(1);

        assert_eq!(summary.get(), "checked 1 file");
    }
}
