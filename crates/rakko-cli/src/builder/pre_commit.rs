use std::fmt;

use clap::Command;
use rakko_action::{ArgsSchema, ArgsValues, Argument, ArgumentShape, ErasedAction, argument_name};

use super::arguments;

/// The name of the command that runs the actions that guard a commit
pub(super) const NAME: &str = "pre-commit";

/// The description that the command shows in the help
const ABOUT: &str = "Runs the actions that guard a commit, in the order that the harness names";

/// The documentation of the flag that asks the actions to repair what they can
const FIX_DOCUMENTATION: &str = "Rewrite what the actions of the list can repair";

/// One action of the list that guards a commit
///
/// A step names the action and says what a run of the `pre-commit` command
/// gives it. The command carries a `fix` flag, and a step either asks for
/// that flag or asks for nothing. The harness decides which, so the command
/// line learns nothing about the arguments that an action reads.
///
/// # Examples
///
/// ```
/// # use rakko_action::{Action, Context, Name, Outcome, action_name};
/// # struct FormatToml;
/// # impl Action for FormatToml {
/// #     type Args = ();
/// #     fn name(&self) -> Name { action_name!("format-toml") }
/// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
/// #         Outcome::Passed { summary: None }
/// #     }
/// # }
/// # struct LintRust;
/// # impl Action for LintRust {
/// #     type Args = ();
/// #     fn name(&self) -> Name { action_name!("lint-rust") }
/// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
/// #         Outcome::Passed { summary: None }
/// #     }
/// # }
/// use rakko_cli::Step;
///
/// let steps = [
///     Step::with_fix(Box::new(FormatToml)),
///     Step::new(Box::new(LintRust)),
/// ];
/// ```
pub struct Step {
    /// The action that the step runs
    action: Box<dyn ErasedAction>,
    /// Whether the action gets the `fix` flag of the command
    fix: bool,
}

impl Step {
    /// Creates a step whose action gets nothing from the command
    #[must_use]
    pub fn new(action: Box<dyn ErasedAction>) -> Self {
        Self { action, fix: false }
    }

    /// Creates a step whose action gets the `fix` flag of the command
    ///
    /// The action reads the flag as its own `fix` argument, so a run of the
    /// command with the flag lets the action repair what it can, and a run
    /// without the flag lets it report.
    #[must_use]
    pub fn with_fix(action: Box<dyn ErasedAction>) -> Self {
        Self { action, fix: true }
    }

    /// Returns the values that a run gives the action, from the values of the
    /// command
    ///
    /// The command carries the `fix` flag alone, so a step that asks for the
    /// flag gets the values of the command as they are, and every other step
    /// gets no value.
    // cli[impl precommit.fix]
    pub(super) fn values(&self, command: &ArgsValues) -> ArgsValues {
        if self.fix {
            command.clone()
        } else {
            ArgsValues::empty()
        }
    }

    /// Takes the action out of the step
    pub(super) fn into_action(self) -> Box<dyn ErasedAction> {
        self.action
    }
}

impl fmt::Debug for Step {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Step")
            .field("action", &self.action.name())
            .field("fix", &self.fix)
            .finish()
    }
}

/// Returns the description of the arguments that the command carries
///
/// The command reads one argument of its own. The description takes the same
/// shape as the arguments of an action, so the flag that renders it and the
/// values that a run collects come from the machinery that every action
/// shares.
// cli[impl precommit.fix]
pub(super) fn schema() -> ArgsSchema {
    ArgsSchema::new([Argument::builder()
        .name(argument_name!("fix"))
        .shape(ArgumentShape::Boolean)
        .documentation(FIX_DOCUMENTATION)
        .build()])
}

/// Returns the command that runs the actions that guard a commit
// cli[impl precommit.list]
// cli[impl precommit.fix]
pub(super) fn command() -> Command {
    Command::new(NAME)
        .about(ABOUT)
        .args(arguments::render(&schema()))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{Action, ArgumentValue, Context, Name, Outcome, action_name};

    use super::*;

    /// An action that reads no arguments and passes
    struct Probe;

    impl Action for Probe {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("probe")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// Returns the values of a command that the user ran with the fix flag
    fn fixing() -> ArgsValues {
        ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
    }

    // cli[verify precommit.fix]
    #[test]
    fn values_of_a_step_that_asks_for_nothing_are_empty() {
        let step = Step::new(Box::new(Probe));

        let values = step.values(&fixing());

        assert_eq!(values, ArgsValues::empty());
    }

    // cli[verify precommit.fix]
    #[test]
    fn values_of_a_step_that_asks_for_the_fix_flag_carry_it() {
        let step = Step::with_fix(Box::new(Probe));

        let values = step.values(&fixing());

        assert_eq!(values, fixing());
    }

    // cli[verify precommit.fix]
    #[test]
    fn values_of_a_step_that_asks_for_the_fix_flag_are_empty_without_it() {
        let step = Step::with_fix(Box::new(Probe));

        let values = step.values(&ArgsValues::empty());

        assert_eq!(values, ArgsValues::empty());
    }
}
