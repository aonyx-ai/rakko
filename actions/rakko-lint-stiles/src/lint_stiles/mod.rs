//! The action that applies the stiles rules of a project
//!
//! This module holds the action and the error that stops a run.
//! The action wraps stiles as a subprocess.
//! Stiles reads its own configuration and loads the lints that the
//! configuration names.
//! Stiles discovers the files and applies the rules.
//! The action translates what stiles reported into an outcome.

mod error;

use rakko::{
    Access, Action, Context, Documented, Finding, Location, Name, Outcome, ProjectRoot, SkipReason,
    Summary, ToolRequirement, action_name,
};

pub use self::error::LintStilesError;
use crate::observation::Examination;
use crate::problem::StilesProblem;
use crate::stiles::Stiles;

/// The message of a run whose stiles lists no rule that the project runs
///
/// Stiles ships no rule of its own, so a project without a configuration of
/// stiles, or with one that names no lint, runs no rule. A check of such a
/// project passes whatever the code holds, which says nothing about the code.
/// The project therefore fails, and the message names the file that fixes it.
const NO_RULES: &str = "stiles lists no rule that the project runs, so name the lints of the \
                        project in `.config/stiles.toml`";

/// The reason of a run whose stiles found no file to check
///
/// Stiles checks the files of the languages that it has a grammar for. The
/// ignore files and the configuration of the project can exclude them all as
/// well.
const NOTHING_TO_CHECK: &str = "stiles found no file to check";

/// Applies the stiles rules of a project
///
/// The action wraps [stiles].
/// Stiles reads the configuration of the project in `.config/stiles.toml`,
/// loads the lints that it names, and discovers the Rust and TypeScript files.
/// It applies the rules that the configuration turns on, so a run agrees with a
/// contributor that runs stiles bare.
/// The action declares the version of stiles that runs, because the lints that
/// a project loads are built against one version.
/// The stiles that runs is the one that [mise] installed at that version, and
/// the action installs nothing.
/// Stiles has builds only for Linux and for macOS on ARM. On every other
/// platform, the machinery skips a run of the action and gives the tool and
/// the platform as its reason.
///
/// A run only reports, and it takes no argument.
/// Stiles repairs nothing that a rule finds.
/// Every diagnostic that a rule weighs as an error or a warning becomes a
/// finding at the position stiles reported.
/// The message holds the severity, the rule, and the words of stiles.
/// A warning fails a run like an error, because a project that wants a rule to
/// stay quiet turns that rule off.
/// A note and a help fail no run, as in stiles itself.
///
/// Every project that mounts the action is meant to run stiles rules.
/// Before it checks the project, a run asks stiles to list the rules that the
/// configuration turns on.
/// It fails when that list is empty, because stiles checks such a project with
/// no rule at all, and a check with no rule passes whatever the code holds.
/// The finding belongs to the project and names `.config/stiles.toml`.
/// It skips when stiles finds no file to check.
/// A run stops with an error when mise reports no stiles at the declared
/// version.
/// It stops when stiles stops before checking the whole project.
/// It stops when stiles writes a report that the action cannot read.
///
/// A run declares a read of the whole project. Stiles analyzes the types of the
/// code it checks. So it reads the manifests and the sources that the files of
/// the project refer to, and not only the files that it checks. It builds a
/// lint that the configuration names from source. The build happens in a
/// directory that cargo locks itself, so it does not count as a write. A caller
/// that orders runs by their declarations runs it after the actions that write.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use rakko_lint_stiles::LintStiles;
///
/// let actions = Bundle::default().with(LintStiles);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [stiles]: https://github.com/aonyx-ai/stiles
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct LintStiles;

impl Action for LintStiles {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("lint-stiles")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Stiles::requirement()]
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
    let stiles = Stiles::resolve(context)
        .await
        .map_err(|source| LintStilesError::UnresolvedTool { source })?;

    let rules = stiles.enabled_rules().await?;

    if rules.is_empty() {
        return Ok(Outcome::Failed {
            findings: vec![no_rules()],
            repairs: Vec::new(),
        });
    }

    let observation = stiles.observe().await?;

    match observation.examined() {
        Examination::Skipped => {
            return Ok(Outcome::Skipped {
                reason: SkipReason::new(NOTHING_TO_CHECK),
            });
        }
        Examination::Examined => {}
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

/// Returns the finding of a project whose stiles lists no rule
///
/// The finding belongs to the project and to no file in it, because the
/// missing configuration is the problem and not a line of code.
fn no_rules() -> Finding {
    Finding::builder()
        .message(NO_RULES)
        .location(Location::Project)
        .build()
}

/// Returns the finding that reports one diagnostic of stiles
///
/// The finding sits at the position where stiles said the diagnostic starts.
/// The message comes from stiles. So a reader of a finding reads what the tool
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::Position;
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
        StilesProblem::builder()
            .path(path("./src/lib.rs"))
            .position(Position::builder().line(7).column(5).build())
            .severity(severity)
            .rule("lint.todo-comment".to_owned())
            .text("a TODO comment marks unfinished work".to_owned())
            .build()
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
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = StilesProblem::builder()
            .path(path("/elsewhere/lib.rs"))
            .position(Position::builder().line(1).column(1).build())
            .severity(Severity::Error)
            .rule("lint.todo-comment".to_owned())
            .text("a TODO comment marks unfinished work".to_owned())
            .build();

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
    fn lint_stiles_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<LintStiles>();
    }

    #[test]
    fn no_rules_belongs_to_the_project() {
        let finding = no_rules();

        assert_eq!(finding.location(), &Location::Project);
    }

    #[test]
    fn no_rules_names_the_configuration_of_stiles() {
        let finding = no_rules();

        assert!(finding.message().get().contains(".config/stiles.toml"));
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

    #[test]
    fn tools_declare_the_stiles_of_the_crate() {
        let tools = LintStiles.tools();

        assert_eq!(tools, vec![Stiles::requirement()]);
    }
}
