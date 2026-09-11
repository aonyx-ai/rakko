/// The arguments that a run of the command reads
mod args;
/// The errors that stop a run of the command
mod error;
/// What the outcome of one action means for the commit
mod verdict;

use std::fmt;

use rakko_action::{
    ArgsValues, ArgumentValue, Context, ErasedAction, Name, action_name, argument_name,
};
use rakko_cli::clawless::CommandResult;
use rakko_cli::clawless::context::Context as ClawlessContext;
use rakko_cli::{Command, Report};

pub use self::args::PreCommitArgs;
pub use self::error::PreCommitError;
use self::verdict::Verdict;

/// The command that runs the actions that guard a commit
///
/// A commit of a project has to pass a list of actions, and no action
/// describes that list. The list repairs the tree first and then examines what
/// it repaired, and whoever reads it wants what each action said, where an
/// action answers with one outcome. The command therefore drives the actions
/// itself, and the hook that Git runs before a commit calls it.
///
/// The command names no action of its own. Which activities guard a commit is
/// a decision of the project, so the harness names the actions and gives them
/// to the command in two lists: the actions that write to the tree, and the
/// actions that only read it.
pub struct PreCommit {
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
    /// that writes before the first action that reads. The lists are
    /// independent of the mount, so a harness that wants a command for one of
    /// these actions mounts that action as well.
    pub fn new(
        actions_that_write: impl IntoIterator<Item = Box<dyn ErasedAction>>,
        actions_that_read: impl IntoIterator<Item = Box<dyn ErasedAction>>,
    ) -> Self {
        Self {
            actions_that_write: actions_that_write.into_iter().collect(),
            actions_that_read: actions_that_read.into_iter().collect(),
        }
    }
}

/// Shows the names of the actions, because an erased action shows nothing
impl fmt::Debug for PreCommit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreCommit")
            .field("actions_that_write", &names(&self.actions_that_write))
            .field("actions_that_read", &names(&self.actions_that_read))
            .finish()
    }
}

impl Command for PreCommit {
    type Args = PreCommitArgs;

    // precommit[impl name]
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
    // precommit[impl lists.order]
    // precommit[impl lists.fix]
    // precommit[impl result.failed]
    // precommit[impl result.succeeded]
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
// precommit[impl lists.fix]
// precommit[impl args.absent]
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
// precommit[impl lists.order]
// precommit[impl lists.sequential]
// precommit[impl report.each]
// precommit[impl report.unreported]
// precommit[impl result.continue]
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

/// Returns the names of the actions, in the order of the list
fn names(actions: &[Box<dyn ErasedAction>]) -> Vec<Name> {
    actions.iter().map(|action| action.name()).collect()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::future::poll_fn;
    use std::path::Path;
    use std::pin::pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context as TaskContext, Poll, Waker};

    use rakko_action::{Action, Args, ArgsSchema, Outcome, ReadArgsError};
    use rakko_cli::clawless::event::{Event, EventReceiver, event_channel};
    use rakko_cli::clawless::prelude::Output;

    use super::*;

    /// What a recorder answers when it runs
    enum Answer {
        /// The action passes
        Pass,

        /// The action found a problem
        Fail,
    }

    /// The arguments of a recorder, which keep the value of `fix` that a run
    /// gave
    struct Received {
        /// The value of `fix`, if the run gave one
        fix: Option<ArgumentValue>,
    }

    impl Args for Received {
        fn schema() -> ArgsSchema {
            ArgsSchema::empty()
        }

        fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Ok(Self {
                fix: values.get(&argument_name!("fix")).cloned(),
            })
        }
    }

    /// An action that writes into a log when it runs, and answers as the test
    /// asks
    ///
    /// The action returns control to the driver of the run once before it
    /// answers. A run that drives two actions at once therefore starts the next
    /// action while this one waits.
    struct Recorder {
        /// The name that the action reports and writes into the log
        name: Name,

        /// What the action answers
        answer: Answer,

        /// The log that every action of a test writes into, in the order of
        /// their runs
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Action for Recorder {
        type Args = Received;

        fn name(&self) -> Name {
            self.name.clone()
        }

        async fn run(&self, _context: &Context, args: &Self::Args) -> Outcome {
            let entry = match &args.fix {
                Some(value) => format!("{} fix={}", self.name, value.get()),
                None => self.name.to_string(),
            };

            self.log
                .lock()
                .expect("the test runs one action at a time")
                .push(entry);

            yield_once().await;

            match self.answer {
                Answer::Pass => Outcome::Passed { summary: None },
                Answer::Fail => Outcome::Failed {
                    findings: Vec::new(),
                    repairs: Vec::new(),
                },
            }
        }
    }

    /// Drives a future to its end on the current thread and returns its output
    ///
    /// Each future of these tests makes progress every time it is polled, so
    /// the loop needs no waker that wakes it.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut task_context = TaskContext::from_waker(Waker::noop());

        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut task_context) {
                return output;
            }
        }
    }

    /// Returns the context of Clawless that a run writes through, and the
    /// receiver of what the run writes
    fn clawless() -> (ClawlessContext, EventReceiver) {
        let (sender, receiver) = event_channel();
        let context = ClawlessContext::builder()
            .current_working_directory(Path::new("/tmp/project"))
            .output(Output::new(sender))
            .build()
            .expect("the test names a working directory");

        (context, receiver)
    }

    /// Drives a run of the command to its end and returns its result
    ///
    /// The receiver of the output lives until the run ends, because a report
    /// that nothing reads fails the run.
    ///
    /// # Errors
    ///
    /// Returns the error of the run when the command failed.
    fn complete(command: &PreCommit, args: &PreCommitArgs) -> CommandResult {
        let (clawless, _receiver) = clawless();

        block_on(command.run(&project(), &clawless, args))
    }

    /// Returns an action with the given name that found a problem, and that
    /// writes into the given log
    fn failing(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        Box::new(Recorder {
            name,
            answer: Answer::Fail,
            log: Arc::clone(log),
        })
    }

    /// Returns an action with the given name that passes, and that writes into
    /// the given log
    fn passing(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        Box::new(Recorder {
            name,
            answer: Answer::Pass,
            log: Arc::clone(log),
        })
    }

    /// Returns the context of the project that a run maintains
    fn project() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Drives a run of the command to its end and returns the text of each
    /// report that it wrote, in the order in which it wrote them
    fn reports(command: &PreCommit) -> Vec<String> {
        let (clawless, mut receiver) = clawless();
        let _result = block_on(command.run(&project(), &clawless, &PreCommitArgs::default()));

        // The context holds the sender of the output. Without the sender, the
        // receiver ends after the last event that the run wrote.
        drop(clawless);

        let mut reports = Vec::new();
        while let Some(event) = block_on(receiver.recv()) {
            match event {
                Event::Artifact(artifact) => reports.push(artifact.to_string()),
                Event::Message(_) | Event::Detail(_) => {}
            }
        }

        reports
    }

    /// Returns control to the driver of a future once, and then resolves
    async fn yield_once() {
        let mut yielded = false;

        poll_fn(|_| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                Poll::Pending
            }
        })
        .await;
    }

    // precommit[verify name]
    #[test]
    fn name_is_the_name_of_the_hook() {
        let command = PreCommit::new([], []);

        assert_eq!(command.name().get(), "pre-commit");
    }

    // precommit[verify lists.fix]
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

    // precommit[verify args.absent]
    #[test]
    fn repairs_of_a_run_that_asks_for_a_report_are_empty() {
        let args = PreCommitArgs::default();

        let values = repairs(&args);

        assert_eq!(values, ArgsValues::empty());
    }

    // precommit[verify lists.order]
    #[test]
    fn run_drives_the_actions_that_write_first_and_each_list_in_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [
                passing(action_name!("format-yaml"), &log),
                passing(action_name!("format-json"), &log),
            ],
            [
                passing(action_name!("lint-yaml"), &log),
                passing(action_name!("check-specs"), &log),
            ],
        );

        complete(&command, &PreCommitArgs::default()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "format-json", "lint-yaml", "check-specs"]
        );
    }

    // precommit[verify report.each]
    #[test]
    fn run_reports_each_action_in_the_order_of_the_run() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [passing(action_name!("format-yaml"), &log)],
            [
                passing(action_name!("lint-yaml"), &log),
                passing(action_name!("check-specs"), &log),
            ],
        );

        let reports = reports(&command);

        assert_eq!(
            reports,
            [
                "format-yaml: passed",
                "lint-yaml: passed",
                "check-specs: passed"
            ]
        );
    }

    // precommit[verify lists.fix]
    #[test]
    fn run_that_asks_for_a_repair_gives_fix_to_the_actions_that_write_alone() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [passing(action_name!("format-yaml"), &log)],
            [passing(action_name!("lint-yaml"), &log)],
        );
        let args = PreCommitArgs::from_values(&ArgsValues::new([(
            argument_name!("fix"),
            ArgumentValue::new("true"),
        )]))
        .expect("the test gives the fix argument a value that it reads");

        complete(&command, &args).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml fix=true", "lint-yaml"]
        );
    }

    // precommit[verify lists.sequential]
    #[test]
    fn run_waits_for_an_action_before_it_drives_the_next() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [
                passing(action_name!("format-yaml"), &log),
                passing(action_name!("format-json"), &log),
            ],
            [passing(action_name!("lint-yaml"), &log)],
        );
        let project = project();
        let (clawless, _receiver) = clawless();
        let args = PreCommitArgs::default();
        let mut run = pin!(command.run(&project, &clawless, &args));

        let _state = run
            .as_mut()
            .poll(&mut TaskContext::from_waker(Waker::noop()));

        assert_eq!(
            *log.lock().expect("the first action waits"),
            ["format-yaml"]
        );
    }

    // precommit[verify result.failed]
    #[test]
    fn run_with_actions_that_found_problems_counts_them() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [passing(action_name!("format-yaml"), &log)],
            [
                failing(action_name!("lint-yaml"), &log),
                failing(action_name!("check-specs"), &log),
            ],
        );

        let error = complete(&command, &PreCommitArgs::default()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "2 of 3 actions found problems or stopped"
        );
    }

    // precommit[verify result.continue]
    #[test]
    fn run_with_an_action_that_found_problems_drives_the_actions_after_it() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [
                failing(action_name!("format-yaml"), &log),
                passing(action_name!("format-json"), &log),
            ],
            [passing(action_name!("lint-yaml"), &log)],
        );

        let _result = complete(&command, &PreCommitArgs::default());

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "format-json", "lint-yaml"]
        );
    }

    // precommit[verify result.succeeded]
    #[test]
    fn run_with_clean_actions_succeeds() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [passing(action_name!("format-yaml"), &log)],
            [passing(action_name!("lint-yaml"), &log)],
        );

        let result = complete(&command, &PreCommitArgs::default());

        assert!(result.is_ok());
    }

    // precommit[verify report.unreported]
    #[test]
    fn run_without_a_reader_fails_at_the_report_of_the_first_action() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new(
            [passing(action_name!("format-yaml"), &log)],
            [passing(action_name!("lint-yaml"), &log)],
        );
        let (clawless, receiver) = clawless();
        drop(receiver);

        let error =
            block_on(command.run(&project(), &clawless, &PreCommitArgs::default())).unwrap_err();

        assert_eq!(
            error.to_string(),
            "failed to report the outcome of the format-yaml action"
        );
    }
}
