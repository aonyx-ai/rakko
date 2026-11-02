/// The error that stops a run of a group
mod error;

use clawless::context::Context as ClawlessContext;
use rakko_action::{Access, Context};

pub(crate) use self::error::RunGroupError;
use super::members::BoundMember;
use super::{EXIT_CLEAN, exit_code};
use crate::report::Report;

/// Runs the members of a group and reports each member when it finishes
///
/// The members arrive bound to the values of the run, in the order of the
/// list of the group. The scheduler asks every member how its run uses the
/// project before any member runs. So a kind of access that the scheduler does
/// not know stops the run before any member changes the project.
///
/// The members that write run first, so that the members that read examine
/// the project as the writers left it. The writers can write the same files,
/// so no two of them run at the same time. The readers then run one at a time
/// as well. Within each kind, the members run in the order of the list.
///
/// # Errors
///
/// Returns [`RunGroupError::UnknownAccess`] before any member runs when a
/// member declares a kind of access that the scheduler does not know.
///
/// Returns [`RunGroupError::FailedMembers`] after every member ran when a
/// member found problems or stopped. Returns
/// [`RunGroupError::UnreportedOutcome`] when a report does not reach the
/// reader.
pub(super) async fn run(
    members: Vec<BoundMember<'_>>,
    project: &Context,
    clawless: &ClawlessContext,
) -> Result<(), RunGroupError> {
    let total = members.len();

    let mut writes = Vec::new();
    let mut reads = Vec::new();

    for member in members {
        match member.action().access(project) {
            Access::Write => writes.push(member),
            Access::Read => reads.push(member),
            _ => {
                return Err(RunGroupError::UnknownAccess {
                    member: member.name().clone(),
                });
            }
        }
    }

    let mut problems = drive(writes, project, clawless).await?;
    problems += drive(reads, project, clawless).await?;

    if problems > 0 {
        return Err(RunGroupError::FailedMembers { problems, total });
    }

    Ok(())
}

/// Runs a list of bound members in order and returns how many of them did not
/// pass
///
/// The run reports a member as soon as that member finishes, so a reader
/// learns what one member found while the next one runs, and no two reports
/// mix.
///
/// A member that found problems or stopped does not end the run. A run that
/// stopped at the first problem would hide the problems after it, and whoever
/// started it would learn them one run at a time. A member passes when a run
/// of that action alone exits clean: a member that skipped or that repaired
/// everything that it found passes as well.
///
/// # Errors
///
/// Returns [`RunGroupError::UnreportedOutcome`] when the report of a member
/// does not reach the reader.
async fn drive(
    members: Vec<BoundMember<'_>>,
    project: &Context,
    clawless: &ClawlessContext,
) -> Result<usize, RunGroupError> {
    let mut problems = 0;

    for member in members {
        let name = member.name().clone();
        let outcome = member.action().run(project).await;

        if exit_code(&outcome) != EXIT_CLEAN {
            problems += 1;
        }

        clawless
            .output()
            .artifact(Report::new(name.clone(), outcome))
            .await
            .map_err(|source| RunGroupError::UnreportedOutcome {
                member: name,
                source,
            })?;
    }

    Ok(problems)
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

    use clawless::event::{Event, EventReceiver, event_channel};
    use clawless::output::Output;
    use rakko_action::{
        Access, Action, Args, ArgsSchema, ArgsValues, Argument, ArgumentShape, ArgumentValue,
        ErasedAction, Name, Outcome, ReadArgsError, SkipReason, action_name, argument_name,
    };

    use super::*;

    /// What a recorder answers when it runs
    enum Answer {
        /// The action passes
        Pass,

        /// The action found a problem
        Fail,

        /// The action repaired everything that it found
        Repair,

        /// The action has nothing to examine
        Skip,

        /// The action stopped before it had an answer
        Stop,
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
                Answer::Repair => Outcome::Changed {
                    repairs: Vec::new(),
                },
                Answer::Skip => Outcome::Skipped {
                    reason: SkipReason::new("the project has no YAML file"),
                },
                Answer::Stop => Outcome::Errored {
                    source: Box::new(std::io::Error::other("the tool is missing")),
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
        /// The recorder that gives the action its name and runs it
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
        /// The recorder that gives the action its name and runs it
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

    /// Binds every action to the values of a run, as the command line binds
    /// the members of a group
    ///
    /// Each action receives the values of the arguments that it declares, and
    /// no other value. The command line does this itself before a run, and its
    /// own tests show it. The scheduler receives the members already bound, so
    /// these tests bind them here.
    fn bind<'a>(actions: &'a [Box<dyn ErasedAction>], values: &ArgsValues) -> Vec<BoundMember<'a>> {
        actions
            .iter()
            .map(|action| {
                let schema = action.arguments();
                let declared = ArgsValues::new(schema.arguments().iter().filter_map(|argument| {
                    values
                        .get(argument.name())
                        .map(|value| (argument.name().clone(), value.clone()))
                }));

                BoundMember::bind(action.as_ref(), &declared)
                    .expect("every action of the test reads the values that it declares")
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
            .expect("the test gives a working directory");

        (context, receiver)
    }

    /// Drives a run of the actions with the given values to its end and
    /// returns its result
    ///
    /// The receiver of the output lives until the run ends, because a report
    /// that nothing reads fails the run.
    ///
    /// # Errors
    ///
    /// Returns the error of the run when it failed.
    fn complete(
        actions: &[Box<dyn ErasedAction>],
        values: &ArgsValues,
    ) -> Result<(), RunGroupError> {
        let (clawless, _receiver) = clawless();

        block_on(run(bind(actions, values), &project(), &clawless))
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

    /// Drives a run of the actions without values to its end and returns the
    /// text of each report that it wrote, in the order in which it wrote them
    fn reports(actions: &[Box<dyn ErasedAction>]) -> Vec<String> {
        let (clawless, mut receiver) = clawless();
        let _result = block_on(run(
            bind(actions, &ArgsValues::empty()),
            &project(),
            &clawless,
        ));

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
    fn run_drives_a_writer_that_follows_a_reader_in_the_list_first() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
        ];

        complete(&actions, &ArgsValues::empty()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "lint-yaml"]
        );
    }

    #[test]
    fn run_drives_an_action_that_declares_nothing_among_the_writers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            reader(action_name!("lint-yaml"), &log),
            undeclared(action_name!("generate-sdk"), &log),
        ];

        complete(&actions, &ArgsValues::empty()).expect("every action of the test passes");

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
        let actions = [
            formatter(action_name!("format-yaml"), &log),
            writer(action_name!("generate-sdk"), &log),
        ];

        complete(&actions, &fixing()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml fix=true", "generate-sdk"]
        );
    }

    #[test]
    fn run_keeps_the_order_of_the_list_among_the_writers_and_among_the_readers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-toml"), &log),
            writer(action_name!("format-json"), &log),
        ];

        complete(&actions, &ArgsValues::empty()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "format-json", "lint-yaml", "lint-toml"]
        );
    }

    #[test]
    fn run_reports_each_action_in_the_order_of_the_run() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-toml"), &log),
        ];

        let reports = reports(&actions);

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
        let actions = [
            reader(action_name!("lint-yaml"), &log),
            formatter(action_name!("format-yaml"), &log),
        ];

        complete(&actions, &fixing()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml fix=true", "lint-yaml"]
        );
    }

    #[test]
    fn run_that_asks_for_a_report_drives_a_formatter_among_the_readers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            formatter(action_name!("format-yaml"), &log),
            writer(action_name!("generate-sdk"), &log),
        ];

        complete(&actions, &ArgsValues::empty()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["generate-sdk", "format-yaml"]
        );
    }

    #[test]
    fn run_waits_for_an_action_before_it_drives_the_next() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            writer(action_name!("format-yaml"), &log),
            writer(action_name!("format-json"), &log),
            reader(action_name!("lint-yaml"), &log),
        ];
        let project = project();
        let (clawless, _receiver) = clawless();
        let members = bind(&actions, &ArgsValues::empty());
        let mut future = pin!(run(members, &project, &clawless));

        let _state = future
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
        let actions = [
            writer(action_name!("format-yaml"), &log),
            failing(action_name!("lint-yaml"), &log),
            failing(action_name!("lint-toml"), &log),
        ];

        let result = complete(&actions, &ArgsValues::empty());

        assert_eq!(
            result.map_err(|error| error.to_string()),
            Err("2 of 3 actions found problems or stopped".to_owned())
        );
    }

    #[test]
    fn run_with_an_action_that_found_problems_drives_the_actions_after_it() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            writer(action_name!("format-yaml"), &log),
            failing(action_name!("lint-yaml"), &log),
            reader(action_name!("lint-toml"), &log),
        ];

        let _result = complete(&actions, &ArgsValues::empty());

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "lint-yaml", "lint-toml"]
        );
    }

    #[test]
    fn run_with_an_action_that_repaired_the_project_succeeds() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            recorder(
                action_name!("format-yaml"),
                Declaration::Write,
                Answer::Repair,
                &log,
            ),
            reader(action_name!("lint-yaml"), &log),
        ];

        let result = complete(&actions, &ArgsValues::empty());

        assert!(result.is_ok());
    }

    #[test]
    fn run_with_an_action_that_skipped_succeeds() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            writer(action_name!("format-yaml"), &log),
            recorder(
                action_name!("lint-yaml"),
                Declaration::Read,
                Answer::Skip,
                &log,
            ),
        ];

        let result = complete(&actions, &ArgsValues::empty());

        assert!(result.is_ok());
    }

    #[test]
    fn run_with_an_action_that_stopped_counts_it() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            writer(action_name!("format-yaml"), &log),
            recorder(
                action_name!("lint-yaml"),
                Declaration::Read,
                Answer::Stop,
                &log,
            ),
        ];

        let result = complete(&actions, &ArgsValues::empty());

        assert_eq!(
            result.map_err(|error| error.to_string()),
            Err("1 of 2 actions found problems or stopped".to_owned())
        );
    }

    #[test]
    fn run_with_clean_actions_succeeds() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-yaml"), &log),
        ];

        let result = complete(&actions, &ArgsValues::empty());

        assert!(result.is_ok());
    }

    #[test]
    fn run_without_a_reader_fails_at_the_report_of_the_first_action() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
        ];
        let (clawless, receiver) = clawless();
        drop(receiver);

        let result = block_on(run(
            bind(&actions, &ArgsValues::empty()),
            &project(),
            &clawless,
        ));

        assert_eq!(
            result.map_err(|error| error.to_string()),
            Err("failed to report the outcome of the format-yaml action".to_owned())
        );
    }
}
