//! The action that lints the Markdown files of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps markdownlint as a subprocess. Markdownlint discovers the files, reads
//! its configuration, and applies its rules. The action translates what
//! markdownlint reported into an outcome.

mod error;

use rakko::{
    Access, Action, Context, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason,
    ToolRequirement, action_name,
};

pub use self::error::LintMarkdownError;
use crate::markdownlint::Markdownlint;
use crate::observation::{Examination, Exit, Observation};
use crate::problem::MarkdownlintProblem;

/// The reason of a run whose markdownlint examined nothing
///
/// Markdownlint answers with its usage text or with a sentence of its own when
/// it resolves no file. This is what a project without Markdown files reports.
/// A project whose ignore file excludes every one of them reports the same way.
const NOTHING_TO_EXAMINE: &str = "markdownlint found no Markdown file to examine";

/// The action that lints the Markdown files of a project
///
/// The action wraps [markdownlint]. Markdownlint discovers the Markdown files
/// of the project, reads its configuration, and applies the rules that the
/// project turned on. So a run agrees with an editor and with a contributor
/// that runs markdownlint bare. The markdownlint that runs is the version that
/// the action declares, which [mise] installed, whatever version the project
/// pins. It runs on the Node that the project pins, when the action accepts
/// that version. The action installs nothing.
///
/// A run examines the files and directories whose name starts with a dot,
/// such as `.github`, which markdownlint skips by default. It leaves out every
/// directory with the name `.git`, and the ignore file of the project decides
/// what else a run leaves out.
///
/// A run only reports, and it takes no argument. Every rule that a file broke
/// becomes a finding on the line that markdownlint named, at the column when
/// the rule points at one. The message of the finding is the sentence that
/// markdownlint would have written for a reader.
///
/// The action applies to a project that holds Markdown files, and it skips
/// visibly otherwise. A run stops with an error when mise reports no
/// markdownlint at the declared version or no Node in the accepted range, and
/// when markdownlint writes a report that the action cannot read.
///
/// A run declares a read of the project. A caller that orders runs by their
/// declarations runs it after the actions that write.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use rakko_lint_markdown::LintMarkdown;
///
/// let actions = Bundle::default().with(LintMarkdown);
/// ```
///
/// [markdownlint]: https://github.com/DavidAnson/markdownlint
/// [mise]: https://mise.jdx.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct LintMarkdown;

impl Action for LintMarkdown {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("lint-markdown")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![
            Markdownlint::requirement(),
            Markdownlint::node_requirement(),
        ]
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
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
    let markdownlint = Markdownlint::resolve(context)
        .await
        .map_err(|source| LintMarkdownError::UnresolvedTool { source })?;

    let observation = markdownlint.observe().await?;

    if let Some(outcome) = guard(&observation)? {
        return Ok(outcome);
    }

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
/// examined no file leaves the action with nothing to answer for. The ignore
/// file of the project explains this, so the run skips. A run that reported
/// no rule and ended with success found a clean project. A run that reported
/// no rule and ended without success failed for a reason the action cannot
/// name. An answer built on it would hide every problem behind a green result.
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
    match observation.examined() {
        Examination::Skipped => {
            return Ok(Some(Outcome::Skipped {
                reason: SkipReason::new(NOTHING_TO_EXAMINE),
            }));
        }
        Examination::Examined => {}
    }

    if !observation.problems().is_empty() {
        return Ok(None);
    }

    match observation.succeeded() {
        Exit::Failed => Err(LintMarkdownError::UnrecognizedReport {
            stderr: observation.stderr().clone(),
        }),
        Exit::Succeeded => Ok(Some(Outcome::Passed { summary: None })),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::PathBuf;

    use rakko_test_utils::path;

    use super::*;

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns a problem at the given position of a file of the project
    fn problem(line: u32, column: Option<u32>) -> MarkdownlintProblem {
        MarkdownlintProblem::builder()
            .path(PathBuf::from("notes.md"))
            .line(line)
            .maybe_column(column)
            .message("MD013/line-length Line length [Expected: 80; Actual: 92]".to_owned())
            .build()
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn access_is_read() {
        let access = LintMarkdown.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = MarkdownlintProblem::builder()
            .path(path("/elsewhere/notes.md"))
            .line(1)
            .message("MD041/first-line-heading".to_owned())
            .build();

        let error = finding(&problem, &root()).unwrap_err();

        assert!(matches!(error, LintMarkdownError::ForeignPath { .. }));
    }

    #[test]
    fn finding_of_a_problem_carries_the_message_of_markdownlint() {
        let finding = finding(&problem(4, Some(81)), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "MD013/line-length Line length [Expected: 80; Actual: 92]"
        );
    }

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
    fn guard_of_a_clean_run_passes() {
        let outcome = guard(&Observation::builder().build()).unwrap();

        assert!(
            matches!(outcome, Some(Outcome::Passed { summary: None })),
            "expected the run to pass, got {outcome:?}"
        );
    }

    #[test]
    fn guard_of_a_failed_run_without_a_problem_stops() {
        let observation = Observation::builder().succeeded(Exit::Failed).build();

        let error = guard(&observation).unwrap_err();

        assert!(matches!(
            error,
            LintMarkdownError::UnrecognizedReport { .. }
        ));
    }

    #[test]
    fn guard_of_a_run_that_examined_nothing_skips() {
        let observation = Observation::builder()
            .examined(Examination::Skipped)
            .build();

        let outcome = guard(&observation).unwrap();

        assert!(
            matches!(outcome, Some(Outcome::Skipped { .. })),
            "expected the run to skip, got {outcome:?}"
        );
    }

    #[test]
    fn guard_of_a_run_that_reported_a_problem_reports_nothing() {
        let observation = Observation::builder()
            .problems(vec![problem(4, None)])
            .succeeded(Exit::Failed)
            .build();

        let outcome = guard(&observation).unwrap();

        assert!(outcome.is_none(), "expected no outcome, got {outcome:?}");
    }

    #[test]
    fn lint_markdown_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<LintMarkdown>();
    }

    #[test]
    fn tools_declare_the_markdownlint_and_its_node() {
        let tools = LintMarkdown.tools();

        assert_eq!(
            tools,
            vec![
                Markdownlint::requirement(),
                Markdownlint::node_requirement()
            ]
        );
    }
}
