//! The action that lints the Markdown files of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps markdownlint as a subprocess: markdownlint discovers the files,
//! reads its own configuration, and applies its rules, and the action
//! translates what markdownlint reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason,
    action_name,
};

pub use self::error::LintMarkdownError;
use crate::markdownlint::Markdownlint;
use crate::observation::Observation;
use crate::problem::MarkdownlintProblem;

/// The reason of a run that found no Markdown file
const NO_MARKDOWN_FILES: &str = "the project holds no file with the .md or .markdown extension";

/// The reason of a run whose markdownlint examined nothing
///
/// The look of the action found a file, and markdownlint then collected none.
/// The ignore file of the project explains the difference.
const NOTHING_TO_EXAMINE: &str = "markdownlint found no Markdown file to examine";

/// The action that lints the Markdown files of a project
///
/// The action wraps [markdownlint]: markdownlint discovers the Markdown files
/// of the project, reads its own configuration, and applies the rules that
/// the project turned on, so a run agrees with an editor and with a
/// contributor that runs markdownlint bare. The markdownlint that runs is the
/// one that [mise] installed for the project, at the version that the project
/// pinned, and the action installs nothing.
///
/// A run only reports, and it takes no argument. Every rule that a file broke
/// becomes a finding on the line that markdownlint named, at the column when
/// the rule points at one, and the message of the finding is the sentence
/// that markdownlint would have written for a reader.
///
/// The action applies to a project that holds Markdown files, and it skips
/// visibly otherwise. A run stops with an error when mise reports no
/// markdownlint, and when markdownlint writes a report that the action cannot
/// read.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_lint_markdown::LintMarkdown;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintMarkdown)];
/// ```
///
/// [markdownlint]: https://github.com/DavidAnson/markdownlint
/// [mise]: https://mise.jdx.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct LintMarkdown;

impl Action for LintMarkdown {
    // lintmarkdown[impl args.none]
    type Args = ();

    // lintmarkdown[impl name]
    fn name(&self) -> Name {
        action_name!("lint-markdown")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // lintmarkdown[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves markdownlint, and lints. An error
/// that this function returns stops the run, and the caller reports it in the
/// outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, the markdownlint run, or the reading of the report.
async fn drive(context: &Context) -> Result<Outcome, LintMarkdownError> {
    // lintmarkdown[impl skip.hidden]
    // lintmarkdown[impl skip.links]
    // lintmarkdown[impl skip.missing]
    if !Markdownlint::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MARKDOWN_FILES),
        });
    }

    // lintmarkdown[impl tool.markdownlint]
    // lintmarkdown[impl tool.missing]
    let markdownlint = Markdownlint::resolve(context.root().clone())
        .await
        .map_err(|source| LintMarkdownError::UnresolvedTool { source })?;

    // lintmarkdown[impl check.read]
    let observation = markdownlint.observe().await?;

    if let Some(outcome) = guard(&observation)? {
        return Ok(outcome);
    }

    // lintmarkdown[impl check.column]
    // lintmarkdown[impl check.violation]
    Ok(Outcome::Failed {
        findings: findings(observation.problems(), context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one rule that a file broke
///
/// The finding sits on the line that markdownlint named, and at the column
/// when the rule points at one. The message comes from markdownlint, so a
/// reader of a finding reads what the tool itself would have told them.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of the problem.
///
/// [foreign]: LintMarkdownError::ForeignPath
// lintmarkdown[impl check.column]
// lintmarkdown[impl check.violation]
fn finding(
    problem: &MarkdownlintProblem,
    root: &ProjectRoot,
) -> Result<Finding, LintMarkdownError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| LintMarkdownError::ForeignPath {
            path: problem.path().clone(),
        })?;

    let position = Position::builder()
        .line(problem.line())
        .maybe_column(problem.column())
        .build();

    Ok(Finding::builder()
        .message(problem.message().clone())
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
/// [foreign]: LintMarkdownError::ForeignPath
fn findings(
    problems: &[MarkdownlintProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, LintMarkdownError> {
    problems
        .iter()
        .map(|problem| finding(problem, root))
        .collect()
}

/// Returns what a run reports when it cannot answer from its findings
///
/// Three conditions end a run before its findings matter. A markdownlint that
/// examined no file leaves the action with nothing to answer for, which the
/// ignore file of the project explains, so the run skips. A run that reported
/// no rule and ended with success found a clean project. A run that reported
/// no rule and ended without success failed for a reason that the action
/// cannot name, and an answer built on it would hide every problem behind a
/// green result.
///
/// Returns `None` when the caller reports what the run found.
///
/// # Errors
///
/// Returns [`UnrecognizedReport`][unrecognized] when the run ended without
/// success and reported no rule.
///
/// [unrecognized]: LintMarkdownError::UnrecognizedReport
fn guard(observation: &Observation) -> Result<Option<Outcome>, LintMarkdownError> {
    // lintmarkdown[impl skip.unexamined]
    if !observation.examined() {
        return Ok(Some(Outcome::Skipped {
            reason: SkipReason::new(NOTHING_TO_EXAMINE),
        }));
    }

    if !observation.problems().is_empty() {
        return Ok(None);
    }

    // lintmarkdown[impl check.unrecognized]
    if !observation.succeeded() {
        return Err(LintMarkdownError::UnrecognizedReport {
            stderr: observation.stderr().clone(),
        });
    }

    // lintmarkdown[impl check.passed]
    Ok(Some(Outcome::Passed { summary: None }))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    /// Returns a problem at the given position of a file of the project
    fn problem(line: u32, column: Option<u32>) -> MarkdownlintProblem {
        MarkdownlintProblem::new(
            PathBuf::from("notes.md"),
            line,
            column,
            "MD013/line-length Line length [Expected: 80; Actual: 92]".to_owned(),
        )
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    // lintmarkdown[verify check.violation]
    #[test]
    fn finding_of_a_problem_carries_the_message_of_markdownlint() {
        let finding = finding(&problem(4, Some(81)), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "MD013/line-length Line length [Expected: 80; Actual: 92]"
        );
    }

    // lintmarkdown[verify check.column]
    #[test]
    fn finding_of_a_problem_with_a_column_sits_at_that_column() {
        let finding = finding(&problem(4, Some(81)), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "notes.md".parse().unwrap(),
                position: Position::builder().line(4).column(81).build(),
            }
        );
    }

    // lintmarkdown[verify check.column]
    #[test]
    fn finding_of_a_problem_without_a_column_sits_on_its_line() {
        let finding = finding(&problem(4, None), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "notes.md".parse().unwrap(),
                position: Position::builder().line(4).build(),
            }
        );
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = MarkdownlintProblem::new(
            PathBuf::from("/elsewhere/notes.md"),
            1,
            None,
            "MD041/first-line-heading".to_owned(),
        );

        let error = finding(&problem, &root()).unwrap_err();

        assert!(matches!(error, LintMarkdownError::ForeignPath { .. }));
    }

    // lintmarkdown[verify check.passed]
    #[test]
    fn guard_of_a_clean_run_passes() {
        let outcome = guard(&Observation::builder().build()).unwrap();

        assert!(
            matches!(outcome, Some(Outcome::Passed { summary: None })),
            "expected the run to pass, got {outcome:?}"
        );
    }

    // lintmarkdown[verify check.unrecognized]
    #[test]
    fn guard_of_a_failed_run_without_a_problem_stops() {
        let observation = Observation::builder().succeeded(false).build();

        let error = guard(&observation).unwrap_err();

        assert!(matches!(
            error,
            LintMarkdownError::UnrecognizedReport { .. }
        ));
    }

    #[test]
    fn guard_of_a_run_that_reported_a_problem_reports_nothing() {
        let observation = Observation::builder()
            .problems(vec![problem(4, None)])
            .succeeded(false)
            .build();

        let outcome = guard(&observation).unwrap();

        assert!(outcome.is_none(), "expected no outcome, got {outcome:?}");
    }

    // lintmarkdown[verify skip.unexamined]
    #[test]
    fn guard_of_a_run_that_examined_nothing_skips() {
        let observation = Observation::builder().examined(false).build();

        let outcome = guard(&observation).unwrap();

        assert!(
            matches!(outcome, Some(Outcome::Skipped { .. })),
            "expected the run to skip, got {outcome:?}"
        );
    }
}
