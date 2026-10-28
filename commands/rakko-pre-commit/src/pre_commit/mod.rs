/// The errors that stop a run of the command
mod error;
/// What the outcome of one action means for the commit
mod verdict;

use std::fmt;

use rakko_action::{Access, Context, ErasedAction, Name, action_name};
use rakko_cli::clawless::CommandResult;
use rakko_cli::clawless::context::Context as ClawlessContext;
use rakko_cli::{BoundMember, Group, Report};

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
/// to the command in one list. Each action declares whether a run reads or
/// writes the project, and the command orders the run from these
/// declarations.
///
/// The command is a [`Group`], and the actions in its list are its members.
/// The command line derives the flags of the command from the arguments that
/// the members declare, so the command defines no flag of its own. A flag such
/// as `--fix` reaches every member that declares its argument, and no other
/// member.
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

impl Group for PreCommit {
    fn name(&self) -> Name {
        action_name!("pre-commit")
    }

    /// Returns the actions that guard a commit, in the order of the list that
    /// the harness gave
    fn members(&self) -> &[Box<dyn ErasedAction>] {
        &self.actions
    }

    /// Runs the actions that guard a commit, one after another
    ///
    /// The command line binds every action to the values of the run before
    /// the run starts. Each action receives the values of the arguments that
    /// it declares. The run asks each bound action how it uses the project.
    /// The actions that write run first, so that what the other actions read
    /// is what the commit will contain. They can write the same files, so no
    /// two of them run at once. The actions that read run one after another as
    /// well, until a scheduler can overlap them.
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
        members: Vec<BoundMember<'_>>,
    ) -> CommandResult {
        let total = members.len();

        let (writes, reads): (Vec<BoundMember<'_>>, Vec<BoundMember<'_>>) = members
            .into_iter()
            .partition(|member| match member.action().access(project) {
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

/// Runs a list of bound actions in order and returns how many of them did not
/// pass
///
/// The run reports an action as soon as that action finishes, so a reader
/// learns what one action found while the next one runs, and no two reports
/// interleave.
///
/// An action that found problems or stopped does not end the run. A run that
/// stopped at the first problem would hide the ones after it, and whoever
/// started it would learn them one run at a time.
///
/// # Errors
///
/// Returns an error when the report of an action does not reach the reader.
async fn drive(
    members: Vec<BoundMember<'_>>,
    project: &Context,
    clawless: &ClawlessContext,
) -> Result<usize, PreCommitError> {
    let mut problems = 0;

    for member in members {
        let name = member.name().clone();
        let outcome = member.action().run(project).await;
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

    use rakko_action::{
        Action, Args, ArgsSchema, ArgsValues, Argument, ArgumentShape, ArgumentValue, Outcome,
        ReadArgsError, argument_name,
    };
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
    }

    /// The arguments of a member that declares none, but that keeps a value of
    /// `fix` if a run gives it one anyway
    ///
    /// The command line gives such a member no value. A log entry with a value
    /// therefore shows a run that gave the member a value that it does not
    /// declare.
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

    /// The arguments of a member that declares `fix`, as a formatter does
    struct Repair {
        /// The value of `fix`, if the run gave one
        fix: Option<ArgumentValue>,
    }

    impl Args for Repair {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([Argument::builder()
                .name(argument_name!("fix"))
                .shape(ArgumentShape::Boolean)
                .documentation("Rewrite the files that the tool can format")
                .build()])
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

    impl Recorder {
        /// Writes the name of the action and the value of `fix` that its run
        /// received into the log, and returns the answer of the action
        async fn record(&self, fix: Option<&ArgumentValue>) -> Outcome {
            let entry = match fix {
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

    impl Action for Recorder {
        type Args = Received;

        fn name(&self) -> Name {
            self.name.clone()
        }

        fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
            match self.declaration {
                Declaration::Read => Access::Read,
                Declaration::Write => Access::Write,
            }
        }

        async fn run(&self, _context: &Context, args: &Self::Args) -> Outcome {
            self.record(args.fix.as_ref()).await
        }
    }

    /// An action that declares `fix` and writes only when a run gives it, as a
    /// formatter does, and otherwise runs as the recorder that it wraps
    struct Formatter {
        /// The recorder that names the action and runs it
        recorder: Recorder,
    }

    impl Action for Formatter {
        type Args = Repair;

        fn name(&self) -> Name {
            Action::name(&self.recorder)
        }

        fn access(&self, _context: &Context, args: &Self::Args) -> Access {
            match args.fix {
                Some(_) => Access::Write,
                None => Access::Read,
            }
        }

        async fn run(&self, _context: &Context, args: &Self::Args) -> Outcome {
            self.recorder.record(args.fix.as_ref()).await
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

    /// Binds every member of the command to the values of a run, as the
    /// command line binds them
    ///
    /// Each member receives the values of the arguments that it declares, and
    /// no other value. The command line does this itself before a run, and its
    /// own tests show it. A run of the command receives the members already
    /// bound, so these tests bind them here.
    fn bind<'a>(command: &'a PreCommit, values: &ArgsValues) -> Vec<BoundMember<'a>> {
        command
            .members()
            .iter()
            .map(|member| {
                let schema = member.arguments();
                let declared = ArgsValues::new(schema.arguments().iter().filter_map(|argument| {
                    values
                        .get(argument.name())
                        .map(|value| (argument.name().clone(), value.clone()))
                }));

                BoundMember::bind(member.as_ref(), &declared)
                    .expect("every member of the test reads the values that it declares")
            })
            .collect()
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

    /// Drives a run of the command with the given values to its end and
    /// returns its result
    ///
    /// The receiver of the output lives until the run ends, because a report
    /// that nothing reads fails the run.
    ///
    /// # Errors
    ///
    /// Returns the error of the run when the command failed.
    fn complete(command: &PreCommit, values: &ArgsValues) -> CommandResult {
        let (clawless, _receiver) = clawless();

        block_on(command.run(&project(), &clawless, bind(command, values)))
    }

    /// Returns an action with the given name that reads and found a problem,
    /// and that writes into the given log
    fn failing(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        recorder(name, Declaration::Read, Answer::Fail, log)
    }

    /// Returns the values of a run that asks for a repair
    fn fixing() -> ArgsValues {
        ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
    }

    /// Returns an action with the given name that declares `fix`, writes only
    /// when a run gives it, passes, and writes into the given log
    fn formatter(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        Box::new(Formatter {
            recorder: Recorder {
                name,
                declaration: Declaration::Read,
                answer: Answer::Pass,
                log: Arc::clone(log),
            },
        })
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

    /// Drives a run of the command without values to its end and returns the
    /// text of each report that it wrote, in the order in which it wrote them
    fn reports(command: &PreCommit) -> Vec<String> {
        let (clawless, mut receiver) = clawless();
        let _result =
            block_on(command.run(&project(), &clawless, bind(command, &ArgsValues::empty())));

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
    fn members_are_the_actions_in_the_order_of_the_list() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            formatter(action_name!("format-yaml"), &log),
        ]);

        let members: Vec<Name> = command
            .members()
            .iter()
            .map(|member| member.name())
            .collect();

        assert_eq!(
            members,
            [action_name!("lint-yaml"), action_name!("format-yaml")]
        );
    }

    #[test]
    fn name_is_the_name_of_the_hook() {
        let command = PreCommit::new([]);

        assert_eq!(command.name().get(), "pre-commit");
    }

    #[test]
    fn run_drives_a_writer_that_the_list_names_after_a_reader_first() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
        ]);

        complete(&command, &ArgsValues::empty()).expect("every action of the test passes");

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

        complete(&command, &ArgsValues::empty()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["generate-sdk", "lint-yaml"]
        );
    }

    /// The test binds `fix` only to the formatter, which declares it, as the
    /// command line does. The run adds no value, so the writer, which declares
    /// nothing, runs without it.
    #[test]
    fn run_drives_each_member_with_the_values_that_it_was_bound_to() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            formatter(action_name!("format-yaml"), &log),
            writer(action_name!("generate-sdk"), &log),
        ]);

        complete(&command, &fixing()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml fix=true", "generate-sdk"]
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

        complete(&command, &ArgsValues::empty()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "format-json", "lint-yaml", "lint-toml"]
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
            ["format-yaml fix=true", "lint-yaml"]
        );
    }

    #[test]
    fn run_that_asks_for_a_report_drives_a_formatter_among_the_readers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            formatter(action_name!("format-yaml"), &log),
            writer(action_name!("generate-sdk"), &log),
        ]);

        complete(&command, &ArgsValues::empty()).expect("every action of the test passes");

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
        let members = bind(&command, &ArgsValues::empty());
        let mut run = pin!(command.run(&project, &clawless, members));

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

        let error = complete(&command, &ArgsValues::empty()).unwrap_err();

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

        let _result = complete(&command, &ArgsValues::empty());

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "lint-yaml", "lint-toml"]
        );
    }

    #[test]
    fn run_with_clean_actions_succeeds() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let command = PreCommit::new([
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-yaml"), &log),
        ]);

        let result = complete(&command, &ArgsValues::empty());

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
            block_on(command.run(&project(), &clawless, bind(&command, &ArgsValues::empty())))
                .unwrap_err();

        assert_eq!(
            error.to_string(),
            "failed to report the outcome of the format-yaml action"
        );
    }
}
