/// The arguments that a run of the command reads
mod args;
/// The errors that stop a run of the command
mod error;
/// What the outcome of one action means for the commit
mod verdict;

use rakko_action::{
    ArgsValues, ArgumentValue, Context, ErasedAction, Name, action_name, argument_name,
};
use rakko_cli::clawless::CommandResult;
use rakko_cli::clawless::context::Context as ClawlessContext;
use rakko_cli::{Command, Report};

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
/// The command names no action of its own. Which activities guard a commit is
/// a decision of the project, so the harness names the actions and gives them
/// to the command in two lists: the actions that write to the tree, and the
/// actions that only read it.
pub(crate) struct PreCommit {
    /// The actions that write to the tree, in the order in which they run
    actions_that_write: Vec<Box<dyn ErasedAction>>,

    /// The actions that only read the tree, in the order in which they run
    actions_that_read: Vec<Box<dyn ErasedAction>>,
}

impl PreCommit {
    /// Creates the command from the actions that it runs before a commit
    ///
    /// An action that writes to the tree belongs in the first list, whether
    /// it repairs a file or generates one from files that another action
    /// formats. An action that only reads the tree belongs in the second list.
    /// Only the actions that write receive the `fix` argument of a run.
    ///
    /// The command keeps the order of each list, and a run drives every action
    /// that writes before the first action that reads.
    pub(crate) fn new(
        actions_that_write: impl IntoIterator<Item = Box<dyn ErasedAction>>,
        actions_that_read: impl IntoIterator<Item = Box<dyn ErasedAction>>,
    ) -> Self {
        Self {
            actions_that_write: actions_that_write.into_iter().collect(),
            actions_that_read: actions_that_read.into_iter().collect(),
        }
    }
}

impl Command for PreCommit {
    type Args = PreCommitArgs;

    fn name(&self) -> Name {
        action_name!("pre-commit")
    }

    /// Runs the actions that guard a commit, one after another
    ///
    /// The actions that write run first, so that what the other actions read
    /// is what the commit will contain. They can write the same files, so no
    /// two of them run at once. The actions that read run one after another as
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
        let total = self.actions_that_write.len() + self.actions_that_read.len();

        // The actions that write run to their end before the first action that
        // reads starts, so that what it reads is what the commit will contain.
        let mut problems =
            drive(&self.actions_that_write, &repairs(args), project, clawless).await?;
        problems += drive(
            &self.actions_that_read,
            &ArgsValues::empty(),
            project,
            clawless,
        )
        .await?;

        if problems > 0 {
            return Err(PreCommitError::FailedActions { problems, total }.into());
        }

        Ok(())
    }
}

/// Returns the values that the actions which repair receive
///
/// The command carries one flag, and the actions that write take an argument
/// of that name, so a run that was asked to repair passes the flag on and
/// every other run passes nothing. Which actions receive it is the code above:
/// the actions that only read repair nothing, and they take no argument at
/// all.
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
    actions: &[Box<dyn ErasedAction>],
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

    use std::path::Path;
    use std::pin::pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context as TaskContext, Poll, Waker};

    use rakko_action::{Action, Args, Outcome};
    use rakko_cli::clawless::event::event_channel;
    use rakko_cli::clawless::prelude::Output;

    use super::*;

    /// An action that writes its name into a log when it runs, and passes
    struct Recorder {
        /// The name that the action reports and writes into the log
        name: Name,

        /// The log that every action of a test writes into, in the order of
        /// their runs
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Action for Recorder {
        type Args = ();

        fn name(&self) -> Name {
            self.name.clone()
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            self.log
                .lock()
                .expect("the test runs one action at a time")
                .push(self.name.to_string());

            Outcome::Passed { summary: None }
        }
    }

    /// Drives a run of the command to its end and returns its result
    ///
    /// The receiver of the output lives until the run ends, because a report
    /// that nothing reads fails the run.
    ///
    /// # Errors
    ///
    /// Returns the error of the run when the command failed.
    fn complete(command: &PreCommit) -> CommandResult {
        let project = Context::builder().root("/tmp/project").build();
        let (sender, _receiver) = event_channel();
        let clawless = ClawlessContext::builder()
            .current_working_directory(Path::new("/tmp/project"))
            .output(Output::new(sender))
            .build()
            .expect("the test names a working directory");
        let args = PreCommitArgs::default();
        let mut future = pin!(command.run(&project, &clawless, &args));
        let mut task_context = TaskContext::from_waker(Waker::noop());

        loop {
            if let Poll::Ready(result) = future.as_mut().poll(&mut task_context) {
                return result;
            }
        }
    }

    /// Returns an action with the given name that writes into the given log
    fn recorder(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        Box::new(Recorder {
            name,
            log: Arc::clone(log),
        })
    }

    #[test]
    fn name_is_the_name_of_the_hook() {
        let command = PreCommit::new([], []);

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

    #[test]
    fn run_drives_the_actions_that_write_first_and_each_list_in_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [
                recorder(action_name!("format-yaml"), &log),
                recorder(action_name!("format-json"), &log),
            ],
            [
                recorder(action_name!("lint-yaml"), &log),
                recorder(action_name!("check-specs"), &log),
            ],
        );

        complete(&command).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "format-json", "lint-yaml", "check-specs"]
        );
    }
}
