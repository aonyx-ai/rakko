/// The arguments that a run of the command reads
mod args;
/// The errors that stop a run of the command
mod error;
/// One action of a run, bound to the values of the run
mod step;
/// What the outcome of one action means for the commit
mod verdict;

use std::fmt;

use rakko_action::{
    Access, ArgsValues, ArgumentValue, Context, ErasedAction, Name, action_name, argument_name,
};
use rakko_cli::clawless::CommandResult;
use rakko_cli::clawless::context::Context as ClawlessContext;
use rakko_cli::{Command, Report};

pub use self::args::PreCommitArgs;
pub use self::error::PreCommitError;
use self::step::Step;
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
/// to the command in one list. Each action declares whether a run reads or
/// writes the project, and the command orders the run from these
/// declarations.
pub struct PreCommit {
    /// The actions that guard a commit, in the order of the list that the
    /// harness gave
    actions: Vec<Box<dyn ErasedAction>>,
}

impl PreCommit {
    /// Creates the command from the actions that it runs before a commit
    ///
    /// The list can hold the actions in any order. A run asks each action how
    /// it uses the project, and drives every action that writes before the
    /// first action that reads. An action that declares nothing writes. Among
    /// the actions that write, and among the actions that read, the run keeps
    /// the order of the list. This order matters when two actions that write
    /// touch the same files. Two formatters can do this, and so can a
    /// generator that reads the files that a formatter writes.
    ///
    /// The list is independent of the mount, so a harness that wants a command
    /// for one of these actions mounts that action as well.
    pub fn new(actions: impl IntoIterator<Item = Box<dyn ErasedAction>>) -> Self {
        Self {
            actions: actions.into_iter().collect(),
        }
    }
}

/// Shows the names of the actions, because an erased action shows nothing
impl fmt::Debug for PreCommit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreCommit")
            .field("actions", &names(&self.actions))
            .finish()
    }
}

impl Command for PreCommit {
    type Args = PreCommitArgs;

    fn name(&self) -> Name {
        action_name!("pre-commit")
    }

    /// Runs the actions that guard a commit, one after another
    ///
    /// The run binds the values of the run to every action first, and asks
    /// each bound action how it uses the project. The actions that write run
    /// first, so that what the other actions read is what the commit will
    /// contain. They can write the same files, so no two of them run at once.
    /// The actions that read run one after another as well, until a scheduler
    /// can overlap them.
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
        let total = self.actions.len();
        let values = repairs(args);

        // The declaration of an action depends on its arguments, so every
        // action is bound before the run is ordered.
        let (writes, reads): (Vec<Step<'_>>, Vec<Step<'_>>) = self
            .actions
            .iter()
            .map(|action| Step::bind(action.as_ref(), &values))
            .partition(|step| match step.access(project) {
                Access::Write => true,
                Access::Read => false,
            });

        // The actions that write run to their end before the first action that
        // reads starts, so that what it reads is what the commit will contain.
        let mut problems = drive(writes, project, clawless).await?;
        problems += drive(reads, project, clawless).await?;

        if problems > 0 {
            return Err(PreCommitError::FailedActions { problems, total }.into());
        }

        Ok(())
    }
}

/// Returns the values that every action of a run receives
///
/// The command carries one flag, so a run that was asked to repair passes the
/// flag on as the `fix` argument, and every other run passes nothing. Every
/// action receives these values. An action that takes no `fix` argument
/// ignores the value, and an action that takes it declares a write only when
/// a run gives it.
fn repairs(args: &PreCommitArgs) -> ArgsValues {
    if args.fix() {
        ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
    } else {
        ArgsValues::empty()
    }
}

/// Runs a list of steps in order and returns how many of them did not pass
///
/// A step whose values did not convert stops with the error of the
/// conversion, as a problem of the run. The run reports an action as soon as
/// that action finishes, so a reader learns what one action found while the
/// next one runs, and no two reports interleave.
///
/// An action that found problems or stopped does not end the run. A run that
/// stopped at the first problem would hide the ones after it, and whoever
/// started it would learn them one run at a time.
///
/// # Errors
///
/// Returns an error when the report of an action does not reach the reader.
async fn drive(
    steps: Vec<Step<'_>>,
    project: &Context,
    clawless: &ClawlessContext,
) -> Result<usize, PreCommitError> {
    let mut problems = 0;

    for step in steps {
        let name = step.name();
        let outcome = step.run(project).await;
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

    /// How a recorder declares that a run uses the project
    enum Declaration {
        /// Every run reads
        Read,

        /// Every run writes
        Write,

        /// A run writes only when it receives `fix`, as a formatter does
        WriteOnFix,
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

    /// An action that writes into a log when it runs, and declares and answers
    /// as the test asks
    ///
    /// The action returns control to the driver of the run once before it
    /// answers. A run that drives two actions at once therefore starts the next
    /// action while this one waits.
    struct Recorder {
        /// The name that the action reports and writes into the log
        name: Name,

        /// How the action declares that a run uses the project
        declaration: Declaration,

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

        fn access(&self, _context: &Context, args: &Self::Args) -> Access {
            match (&self.declaration, &args.fix) {
                (Declaration::Read, _) | (Declaration::WriteOnFix, None) => Access::Read,
                (Declaration::Write, _) | (Declaration::WriteOnFix, Some(_)) => Access::Write,
            }
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

    /// An action that declares nothing about how a run uses the project, and
    /// otherwise runs as the recorder that it wraps
    struct Undeclared {
        /// The recorder that names the action and runs it
        recorder: Recorder,
    }

    impl Action for Undeclared {
        type Args = Received;

        fn name(&self) -> Name {
            Action::name(&self.recorder)
        }

        async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
            Action::run(&self.recorder, context, args).await
        }
    }

    /// An action whose arguments never convert from the values of a run
    struct Unbuildable;

    impl Action for Unbuildable {
        type Args = Unreadable;

        fn name(&self) -> Name {
            action_name!("unbuildable")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// The arguments of an action that no value converts into
    struct Unreadable;

    impl Args for Unreadable {
        fn schema() -> ArgsSchema {
            ArgsSchema::empty()
        }

        fn from_values(_values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Err(ReadArgsError::MissingValue {
                name: argument_name!("fix"),
            })
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

    /// Returns an action with the given name that reads and found a problem,
    /// and that writes into the given log
    fn failing(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        recorder(name, Declaration::Read, Answer::Fail, log)
    }

    /// Returns the arguments of a run that asks for a repair
    fn fixing() -> PreCommitArgs {
        PreCommitArgs::from_values(&ArgsValues::new([(
            argument_name!("fix"),
            ArgumentValue::new("true"),
        )]))
        .expect("the test gives the fix argument a value that it reads")
    }

    /// Returns an action with the given name that writes only when a run gives
    /// it `fix`, that passes, and that writes into the given log
    fn formatter(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        recorder(name, Declaration::WriteOnFix, Answer::Pass, log)
    }

    /// Returns the context of the project that a run maintains
    fn project() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns an action with the given name that reads and passes, and that
    /// writes into the given log
    fn reader(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        recorder(name, Declaration::Read, Answer::Pass, log)
    }

    /// Returns a recorder with the given name, declaration, and answer, which
    /// writes into the given log
    fn recorder(
        name: Name,
        declaration: Declaration,
        answer: Answer,
        log: &Arc<Mutex<Vec<String>>>,
    ) -> Box<dyn ErasedAction> {
        Box::new(Recorder {
            name,
            declaration,
            answer,
            log: Arc::clone(log),
        })
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
                Event::Message(_) | Event::Detail(_) | Event::Process(_) | Event::Prompt(_) => {}
            }
        }

        reports
    }

    /// Returns an action with the given name that declares nothing and
    /// passes, and that writes into the given log
    fn undeclared(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        Box::new(Undeclared {
            recorder: Recorder {
                name,
                declaration: Declaration::Read,
                answer: Answer::Pass,
                log: Arc::clone(log),
            },
        })
    }

    /// Returns an action with the given name that writes and passes, and that
    /// writes into the given log
    fn writer(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        recorder(name, Declaration::Write, Answer::Pass, log)
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

    #[test]
    fn name_is_the_name_of_the_hook() {
        let command = PreCommit::new([]);

        assert_eq!(command.name().get(), "pre-commit");
    }

    #[test]
    fn repairs_of_a_run_that_asks_for_a_repair_carry_the_fix_argument() {
        let args = fixing();

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
    fn run_drives_a_writer_that_the_list_names_after_a_reader_first() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
        ]);

        complete(&command, &PreCommitArgs::default()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "lint-yaml"]
        );
    }

    #[test]
    fn run_drives_an_action_that_declares_nothing_among_the_writers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            undeclared(action_name!("generate-sdk"), &log),
        ]);

        complete(&command, &PreCommitArgs::default()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["generate-sdk", "lint-yaml"]
        );
    }

    #[test]
    fn run_keeps_the_order_of_the_list_among_the_writers_and_among_the_readers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-toml"), &log),
            writer(action_name!("format-json"), &log),
        ]);

        complete(&command, &PreCommitArgs::default()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "format-json", "lint-yaml", "lint-toml"]
        );
    }

    #[test]
    fn run_reports_an_action_whose_values_do_not_convert_among_the_writers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            Box::new(Unbuildable) as Box<dyn ErasedAction>,
        ]);

        let reports = reports(&command);

        assert_eq!(
            reports,
            [
                "unbuildable: no value for the required argument 'fix'",
                "lint-yaml: passed"
            ]
        );
    }

    #[test]
    fn run_reports_each_action_in_the_order_of_the_run() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-toml"), &log),
        ]);

        let reports = reports(&command);

        assert_eq!(
            reports,
            [
                "format-yaml: passed",
                "lint-yaml: passed",
                "lint-toml: passed"
            ]
        );
    }

    #[test]
    fn run_that_asks_for_a_repair_drives_a_formatter_among_the_writers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            formatter(action_name!("format-yaml"), &log),
        ]);

        complete(&command, &fixing()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml fix=true", "lint-yaml fix=true"]
        );
    }

    #[test]
    fn run_that_asks_for_a_repair_gives_fix_to_every_action() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-yaml"), &log),
        ]);

        complete(&command, &fixing()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml fix=true", "lint-yaml fix=true"]
        );
    }

    #[test]
    fn run_that_asks_for_a_report_drives_a_formatter_among_the_readers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            formatter(action_name!("format-yaml"), &log),
            writer(action_name!("generate-sdk"), &log),
        ]);

        complete(&command, &PreCommitArgs::default()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["generate-sdk", "format-yaml"]
        );
    }

    #[test]
    fn run_waits_for_an_action_before_it_drives_the_next() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            writer(action_name!("format-yaml"), &log),
            writer(action_name!("format-json"), &log),
            reader(action_name!("lint-yaml"), &log),
        ]);
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

    #[test]
    fn run_with_actions_that_found_problems_counts_them() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            writer(action_name!("format-yaml"), &log),
            failing(action_name!("lint-yaml"), &log),
            failing(action_name!("lint-toml"), &log),
        ]);

        let error = complete(&command, &PreCommitArgs::default()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "2 of 3 actions found problems or stopped"
        );
    }

    #[test]
    fn run_with_an_action_that_found_problems_drives_the_actions_after_it() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            writer(action_name!("format-yaml"), &log),
            failing(action_name!("lint-yaml"), &log),
            reader(action_name!("lint-toml"), &log),
        ]);

        let _result = complete(&command, &PreCommitArgs::default());

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "lint-yaml", "lint-toml"]
        );
    }

    #[test]
    fn run_with_an_action_whose_values_do_not_convert_counts_it() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            Box::new(Unbuildable) as Box<dyn ErasedAction>,
            reader(action_name!("lint-yaml"), &log),
        ]);

        let error = complete(&command, &PreCommitArgs::default()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "1 of 2 actions found problems or stopped"
        );
    }

    #[test]
    fn run_with_clean_actions_succeeds() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-yaml"), &log),
        ]);

        let result = complete(&command, &PreCommitArgs::default());

        assert!(result.is_ok());
    }

    #[test]
    fn run_without_a_reader_fails_at_the_report_of_the_first_action() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
        ]);
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
