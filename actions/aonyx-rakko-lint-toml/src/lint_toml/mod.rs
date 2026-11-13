//! The action that validates the TOML files of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps taplo as a subprocess. Taplo discovers the files, reads its own
//! configuration, and validates them. The action translates what taplo
//! reported into an outcome.

mod error;

use aonyx_rakko_taplo::{Observation, Operation, ProblemDetail, Taplo, TaploProblem};
use rakko::{
    Access, Action, Context, Documented, Finding, Location, Name, Outcome, Position, ProjectRoot,
    Summary, ToolRequirement, action_name,
};

pub use self::error::LintTomlError;

/// The message of a finding about a file that is not formatted
///
/// A validation never reports one. The message exists because the problems of
/// taplo carry every level that any of its operations can name. A finding that
/// says what taplo meant is better than one that guesses.
const UNFORMATTED: &str = "the file is not properly formatted";

/// Validates the TOML files of a project
///
/// The action wraps [taplo]. Taplo discovers the TOML files of the project,
/// reads its own configuration, and validates what its configuration selects.
/// So a run agrees with an editor and with a contributor that runs taplo bare.
/// The taplo that runs is the version that the action declares, which [mise]
/// installed, whatever version the project pins. The action installs nothing.
///
/// A run only reports, and it takes no argument. A file that taplo cannot
/// parse, and a file whose content its schema refuses, become findings at the
/// position that taplo reports. A file that taplo cannot open becomes a finding
/// that names the file and carries the reason of taplo. This is because taplo
/// never read a character of it.
///
/// A project that holds no TOML file passes, having found nothing to report. A
/// run stops with an error when mise reports no taplo at the declared version
/// or when taplo rejects a configuration file of the project. It also stops
/// when taplo writes a report that the action does not recognize.
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
/// use aonyx_rakko_lint_toml::LintToml;
///
/// let actions = Bundle::default().with(LintToml);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [taplo]: https://taplo.tamasfe.dev
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct LintToml;

impl Action for LintToml {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("lint-toml")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Taplo::requirement()]
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
/// The run examines the project, resolves taplo, and validates. An error
/// that this function returns stops the run, and the caller reports it in
/// the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, the taplo run, or the reading of the report.
async fn drive(context: &Context) -> Result<Outcome, LintTomlError> {
    let taplo = Taplo::resolve(context)
        .await
        .map_err(|source| LintTomlError::UnresolvedTool { source })?;

    let observation = taplo.observe(Operation::Lint).await?;

    if let Some(details) = observation.rejected_configuration() {
        return Err(LintTomlError::RejectedConfiguration {
            details: details.clone(),
        });
    }

    if observation.problems().is_empty() {
        return Ok(passed(&observation));
    }

    Ok(Outcome::Failed {
        findings: findings(observation.problems(), context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one problem of the project
///
/// A problem that taplo placed in a file gets a finding at that position.
/// A problem that taplo could only attach to a file gets a finding at the
/// file level. Every message comes from taplo, so a reader reads what the
/// tool itself would have said.
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
/// pass that examined nothing. Taplo can lose the line that carries the count.
/// A pass without it stays a pass, because taplo ended the run with success. No
/// line that a report loses can turn a run that found problems into one that
/// found none.
fn passed(observation: &Observation) -> Outcome {
    Outcome::Passed {
        summary: observation.checked().map(summary),
    }
}

/// Returns the summary that tells how many files taplo checked
fn summary(checked: u64) -> Summary {
    if checked == 1 {
        Summary::new("checked 1 file")
    } else {
        Summary::new(format!("checked {checked} files"))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::FilePath;
    use rakko_test_utils::path;

    use super::*;

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    #[test]
    fn access_is_read() {
        let access = LintToml.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn findings_of_a_diagnostic_place_the_finding_at_its_position() {
        let problem = TaploProblem::builder()
            .path(path("/home/otter/project/a.toml"))
            .detail(ProblemDetail::Diagnostic {
                line: 2,
                column: 5,
                message: "unexpected token".to_owned(),
            })
            .build();
        let observation = Observation::builder().problems(vec![problem]).build();
        let root = ProjectRoot::new(path("/home/otter/project"));

        let findings = findings(observation.problems(), &root).unwrap();

        assert_eq!(
            findings,
            vec![
                Finding::builder()
                    .message("unexpected token")
                    .location(Location::Position {
                        path: FilePath::try_from(path("a.toml")).unwrap(),
                        position: Position::builder().line(2).column(5).build(),
                    })
                    .build()
            ]
        );
    }

    #[test]
    fn lint_toml_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<LintToml>();
    }

    #[test]
    fn passed_of_a_run_that_counted_one_file_says_so() {
        let observation = Observation::builder().checked(1).build();

        let outcome = passed(&observation);

        assert!(
            matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "checked 1 file"),
            "expected the count of the files, got {outcome:?}"
        );
    }

    #[test]
    fn passed_of_a_run_without_a_count_says_nothing() {
        let observation = Observation::builder().build();

        let outcome = passed(&observation);

        assert!(
            matches!(&outcome, Outcome::Passed { summary: None }),
            "expected a pass without a summary, got {outcome:?}"
        );
    }
}
