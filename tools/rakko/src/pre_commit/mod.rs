/// The arguments that a run of the command reads
mod args;
/// The errors that stop a run of the command
mod error;
/// What the outcome of one action means for the commit
mod verdict;

use rakko_action::{
    ArgsValues, ArgumentValue, Context, ErasedAction, Name, action_name, argument_name,
};
use rakko_check_specs::CheckSpecs;
use rakko_cli::clawless::CommandResult;
use rakko_cli::clawless::context::Context as ClawlessContext;
use rakko_cli::{Command, Report};
use rakko_format_json::FormatJson;
use rakko_format_markdown::FormatMarkdown;
use rakko_format_rust::FormatRust;
use rakko_format_toml::FormatToml;
use rakko_format_yaml::FormatYaml;
use rakko_lint_github_actions::LintGitHubActions;
use rakko_lint_markdown::LintMarkdown;
use rakko_lint_rust::LintRust;
use rakko_lint_yaml::LintYaml;
use rakko_test_rust::TestRust;
use rakko_test_rust_docs::TestRustDocs;

pub(crate) use self::args::PreCommitArgs;
pub(crate) use self::error::PreCommitError;
use self::verdict::Verdict;

/// The command that runs the actions that guard a commit
///
/// A commit of this repository has to pass a list of actions, and no action
/// describes that list. The list repairs the tree first and then examines what
/// it repaired, and whoever reads it wants what each action said, where an
/// action answers with one outcome. The command therefore drives the actions
/// itself, and the hook that Git runs before a commit calls it.
///
/// The command names its actions in its own code, because which activities
/// guard a commit is a decision of this repository. The list is shorter than
/// the list that the harness mounts: an action such as `check-latest-deps`,
/// which resolves the dependencies of the project again, belongs in a
/// scheduled job and not in front of every commit.
pub(crate) struct PreCommit;

impl Command for PreCommit {
    type Args = PreCommitArgs;

    fn name(&self) -> Name {
        action_name!("pre-commit")
    }

    /// Runs the actions that guard a commit, one after another
    ///
    /// The formatters run first, so that what the checks read is what the
    /// commit will contain. They rewrite the same files, so no two of them
    /// run at once. The checks only read, and they run one after another as
    /// well, until an action declares what it reads and writes and a
    /// scheduler can overlap them.
    ///
    /// # Errors
    ///
    /// Returns an error when an action found problems or stopped, so that the
    /// commit waits for whoever started it. Returns an error as well when a
    /// report does not reach the reader.
    async fn run(
        &self,
        project: &Context,
        clawless: &ClawlessContext,
        args: &Self::Args,
    ) -> CommandResult {
        let formatters = formatters();
        let checks = checks();
        let total = formatters.len() + checks.len();

        // The formatters run to their end before the first check starts, so
        // that what a check reads is what the commit will contain.
        let mut problems = drive(formatters, &repairs(args), project, clawless).await?;
        problems += drive(checks, &ArgsValues::empty(), project, clawless).await?;

        if problems > 0 {
            return Err(PreCommitError::FailedActions { problems, total }.into());
        }

        Ok(())
    }
}

/// Returns the actions that rewrite the tree, in the order that they run
///
/// The order is the one that this repository formatted in before the command
/// existed. The formatters overlap in the files that they write, and a run
/// that let two of them write one file would keep whichever wrote last.
fn formatters() -> Vec<Box<dyn ErasedAction>> {
    vec![
        Box::new(FormatJson),
        Box::new(FormatMarkdown),
        Box::new(FormatYaml),
        Box::new(FormatToml),
        Box::new(FormatRust),
    ]
}

/// Returns the actions that only read the tree, in the order that they run
///
/// Nothing writes while these run, so what each of them sees is what the
/// commit will contain.
fn checks() -> Vec<Box<dyn ErasedAction>> {
    vec![
        Box::new(CheckSpecs),
        Box::new(LintGitHubActions),
        Box::new(LintMarkdown),
        Box::new(LintRust),
        Box::new(LintYaml),
        Box::new(TestRust),
        Box::new(TestRustDocs),
    ]
}

/// Returns the values that the actions which repair receive
///
/// The command carries one flag, and the formatters read an argument of that
/// name, so a run that was asked to repair passes the flag on and every other
/// run passes nothing. Which actions receive it is the code above: the
/// checks repair nothing, and they read no argument at all.
fn repairs(args: &PreCommitArgs) -> ArgsValues {
    if args.fix() {
        ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
    } else {
        ArgsValues::empty()
    }
}

/// Runs a list of actions in order and returns how many of them did not pass
///
/// Every action receives the same values and the same context. The run
/// reports an action as soon as that action finishes, so a reader learns what
/// one action found while the next one runs, and no two reports interleave.
///
/// An action that found problems or stopped does not end the run. A run that
/// stopped at the first problem would hide the ones after it, and whoever
/// started it would learn them one run at a time.
///
/// # Errors
///
/// Returns an error when the report of an action does not reach the reader.
async fn drive(
    actions: Vec<Box<dyn ErasedAction>>,
    values: &ArgsValues,
    project: &Context,
    clawless: &ClawlessContext,
) -> Result<usize, PreCommitError> {
    let mut problems = 0;

    for action in actions {
        let name = action.name();
        let outcome = action.run(project, values).await;
        let verdict = Verdict::of(&outcome);

        clawless
            .output()
            .artifact(Report::new(name.clone(), outcome))
            .await
            .map_err(|source| PreCommitError::UnreportedOutcome {
                action: name,
                source,
            })?;

        match verdict {
            Verdict::Clean => {}
            Verdict::Problem => problems += 1,
        }
    }

    Ok(problems)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::Args;

    use super::*;

    /// Returns the names of the actions of a list, in their order
    fn names(actions: &[Box<dyn ErasedAction>]) -> Vec<String> {
        actions
            .iter()
            .map(|action| action.name().to_string())
            .collect()
    }

    #[test]
    fn checks_run_in_the_order_that_the_repository_checks_in() {
        let actions = checks();

        assert_eq!(
            names(&actions),
            [
                "check-specs",
                "lint-github-actions",
                "lint-markdown",
                "lint-rust",
                "lint-yaml",
                "test-rust",
                "test-rust-docs",
            ]
        );
    }

    #[test]
    fn formatters_run_in_the_order_that_the_repository_formats_in() {
        let actions = formatters();

        assert_eq!(
            names(&actions),
            [
                "format-json",
                "format-markdown",
                "format-yaml",
                "format-toml",
                "format-rust",
            ]
        );
    }

    #[test]
    fn name_is_the_name_of_the_hook() {
        let command = PreCommit;

        assert_eq!(command.name().get(), "pre-commit");
    }

    #[test]
    fn repairs_of_a_run_that_asks_for_a_repair_carry_the_fix_argument() {
        let args = PreCommitArgs::from_values(&ArgsValues::new([(
            argument_name!("fix"),
            ArgumentValue::new("true"),
        )]))
        .expect("the test gives the fix argument a value that it reads");

        let values = repairs(&args);

        assert_eq!(
            values.get(&argument_name!("fix")),
            Some(&ArgumentValue::new("true"))
        );
    }

    #[test]
    fn repairs_of_a_run_that_asks_for_a_report_are_empty() {
        let args = PreCommitArgs::default();

        let values = repairs(&args);

        assert_eq!(values, ArgsValues::empty());
    }
}
