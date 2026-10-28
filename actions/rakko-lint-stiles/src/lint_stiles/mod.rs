//! The action that applies the stiles rules of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps stiles as a subprocess: stiles reads its own configuration, loads the
//! lints that the configuration names, discovers the files, and applies the
//! rules, and the action translates what stiles reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Access, Action, Context, Finding, Location, Name, Outcome, ProjectRoot, SkipReason, Summary,
    action_name,
};

pub use self::error::LintStilesError;
use crate::problem::StilesProblem;
use crate::stiles::Stiles;

/// The reason of a run whose stiles lists no rule that the project runs
///
/// Stiles ships no rule of its own, so a project without a configuration of
/// stiles, or with one that names no lint, runs no rule. A check of such a
/// project passes whatever the code holds, which says nothing about the code.
const NO_RULES: &str = "stiles lists no rule that the project runs";

/// The reason of a run whose stiles found no file to check
///
/// Stiles checks the files of the languages that it has a grammar for. The
/// ignore files and the configuration of the project can exclude them all as
/// well.
const NOTHING_TO_CHECK: &str = "stiles found no file to check";

/// The action that applies the stiles rules of a project
///
/// The action wraps [stiles]: stiles reads the configuration of the project
/// in `.config/stiles.toml`, loads the lints that it names, discovers the Rust
/// and TypeScript files, and applies the rules that the configuration turns
/// on, so a run agrees with a contributor that runs stiles bare. The stiles
/// that runs is the one that [mise] installed for the project, at the version
/// that the project pinned, and the action installs nothing.
///
/// A run only reports, and it takes no argument. Stiles repairs nothing that a
/// rule finds. Every diagnostic that a rule weighs as an error or as a warning
/// becomes a finding at the position that stiles reported, and the message
/// holds the severity, the rule, and the words of stiles. A warning fails a
/// run like an error, because a project that wants a rule to stay quiet turns
/// that rule off. A note and a help fail no run, as in stiles itself.
///
/// The action applies to a project that runs stiles rules, and it skips
/// visibly otherwise. Before it checks the project, a run asks stiles to list
/// the rules that the configuration turns on, and it skips when that list is
/// empty, because stiles checks such a project with no rule at all. It also
/// skips when stiles finds no file to check. A run stops with an error when
/// mise reports no stiles, when stiles stops before it has checked the whole
/// project, and when stiles writes a report that the action cannot read.
///
/// A run declares a read of the whole project. Stiles analyzes the types of
/// the code it checks, so it reads the manifests and the sources that the
/// files of the project refer to, and not only the files that it checks. It
/// builds a lint that the configuration names from source, in a directory
/// that cargo locks itself, so that build does not count as a write. A caller
/// that orders runs by their declarations runs it after the actions that
/// write.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_lint_stiles::LintStiles;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintStiles)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [stiles]: https://github.com/aonyx-ai/stiles
#[derive(Copy, Clone, Debug, Default)]
pub struct LintStiles;

impl Action for LintStiles {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("lint-stiles")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
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
/// The run resolves stiles, asks it for the rules that the project runs, and
/// checks the project with them. An error that this function returns stops
/// the run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, a stiles run, or the reading of a report.
async fn drive(context: &Context) -> Result<Outcome, LintStilesError> {
    let stiles = Stiles::resolve(context.root().clone())
        .await
        .map_err(|source| LintStilesError::UnresolvedTool { source })?;

    let rules = stiles.enabled_rules().await?;

    if rules.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_RULES),
        });
    }

    let observation = stiles.observe().await?;

    if !observation.examined() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NOTHING_TO_CHECK),
        });
    }

    let failing: Vec<&StilesProblem> = observation
        .problems()
        .iter()
        .filter(|problem| problem.severity().fails())
        .collect();

    if failing.is_empty() {
        return Ok(Outcome::Passed {
            summary: Some(summary(rules.len())),
        });
    }

    Ok(Outcome::Failed {
        findings: findings(&failing, context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one diagnostic of stiles
///
/// The finding sits at the position where stiles said the diagnostic starts.
/// The message comes from stiles, so a reader of a finding reads what the tool
/// itself would have told them, including the severity and the rule.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of the problem.
///
/// [foreign]: LintStilesError::ForeignPath
fn finding(problem: &StilesProblem, root: &ProjectRoot) -> Result<Finding, LintStilesError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| LintStilesError::ForeignPath {
            path: problem.path().clone(),
        })?;

    Ok(Finding::builder()
        .message(problem.message())
        .location(Location::Position {
            path,
            position: problem.position(),
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
/// [foreign]: LintStilesError::ForeignPath
fn findings(
    problems: &[&StilesProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, LintStilesError> {
    problems
        .iter()
        .map(|problem| finding(problem, root))
        .collect()
}

/// Returns the summary that tells how many rules the run applied
///
/// A pass names the number of rules, so that a reader can question a pass
/// that applied fewer rules than they expect.
fn summary(rules: usize) -> Summary {
    if rules == 1 {
        Summary::new("applied 1 rule")
    } else {
        Summary::new(format!("applied {rules} rules"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{Files, Position};
    use rakko_test_utils::path;

    use super::*;
    use crate::problem::Severity;

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns a problem of the given severity in a file of the project
    fn problem(severity: Severity) -> StilesProblem {
        StilesProblem::new(
            path("./src/lib.rs"),
            Position::builder().line(7).column(5).build(),
            severity,
            "lint.todo-comment".to_owned(),
            "a TODO comment marks unfinished work".to_owned(),
        )
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn access_is_read() {
        let access = LintStiles.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn files_read_the_whole_project() {
        let files = LintStiles.files(&context(), &());

        assert_eq!(files.reads(), &Files::Project);
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = StilesProblem::new(
            path("/elsewhere/lib.rs"),
            Position::builder().line(1).column(1).build(),
            Severity::Error,
            "lint.todo-comment".to_owned(),
            "a TODO comment marks unfinished work".to_owned(),
        );

        let error = finding(&problem, &root()).unwrap_err();

        assert!(matches!(error, LintStilesError::ForeignPath { .. }));
    }

    #[test]
    fn finding_of_a_problem_carries_the_message_of_stiles() {
        let finding = finding(&problem(Severity::Warning), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "[warning] lint.todo-comment: a TODO comment marks unfinished work"
        );
    }

    #[test]
    fn finding_of_a_problem_sits_at_the_position_of_stiles() {
        let finding = finding(&problem(Severity::Error), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "src/lib.rs".parse().unwrap(),
                position: Position::builder().line(7).column(5).build(),
            }
        );
    }

    #[test]
    fn summary_of_one_rule_says_rule() {
        let summary = summary(1);

        assert_eq!(summary.get(), "applied 1 rule");
    }

    #[test]
    fn summary_of_several_rules_counts_them() {
        let summary = summary(23);

        assert_eq!(summary.get(), "applied 23 rules");
    }
}
