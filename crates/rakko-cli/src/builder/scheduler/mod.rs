/// The error that stops a run of a group
mod error;

use clawless::context::Context as ClawlessContext;
use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use rakko_action::{Access, Context, Name, Outcome};

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
/// so no two of them run at the same time, and they run in the order of the
/// list. When the last writer finished, all the readers start at the same
/// time, with no limit on their number. They share the task of the run, so
/// only one of them runs code at a time. They overlap while they wait, such as
/// for a tool. So a reader that blocks the thread delays the other readers
/// until it returns.
///
/// Every member reports when it finishes. So the reports of the readers
/// follow the order in which the readers finish, and not the order of the
/// list.
///
/// # Errors
///
/// Returns [`RunGroupError::UnknownAccess`] before any member runs when a
/// member declares a kind of access that the scheduler does not know.
///
/// Returns [`RunGroupError::FailedMembers`] after every member ran when a
/// member found problems or stopped. Returns
/// [`RunGroupError::UnreportedOutcome`] when nothing reads the output
/// anymore.
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

    let mut problems = drive_writers(writes, project, clawless).await?;
    problems += drive_readers(reads, project, clawless).await?;

    if problems > 0 {
        return Err(RunGroupError::FailedMembers { problems, total });
    }

    Ok(())
}

/// Runs the members that write one at a time, in the order of the list, and
/// returns how many of them did not pass
///
/// The run reports a member as soon as that member finishes, so whoever runs
/// the group learns what one member found while the next one runs.
///
/// A member that found problems or stopped does not end the run. A run that
/// stopped at the first problem would hide the problems after it, and whoever
/// started it would learn them one run at a time. A member passes when a run
/// of that action alone exits clean: a member that skipped or that repaired
/// everything that it found passes as well.
///
/// # Errors
///
/// Returns [`RunGroupError::UnreportedOutcome`] when nothing reads the output
/// anymore.
async fn drive_writers(
    members: Vec<BoundMember<'_>>,
    project: &Context,
    clawless: &ClawlessContext,
) -> Result<usize, RunGroupError> {
    let mut problems = 0;

    for member in members {
        let outcome = member.action().run(project).await;

        if exit_code(&outcome) != EXIT_CLEAN {
            problems += 1;
        }

        report(member.name(), outcome, clawless).await?;
    }

    Ok(problems)
}

/// Starts all the members that read at the same time and returns how many of
/// them did not pass
///
/// The run reports a member as soon as that member finishes, so the report of
/// a fast member does not wait for a slow member. A member that found problems
/// or stopped does not end the run, and the other members run to their end. A
/// member passes as [`drive_writers`] describes.
///
/// # Errors
///
/// Returns [`RunGroupError::UnreportedOutcome`] when nothing reads the output
/// anymore. The members that still run then stop, because nothing would read
/// what they find.
async fn drive_readers(
    members: Vec<BoundMember<'_>>,
    project: &Context,
    clawless: &ClawlessContext,
) -> Result<usize, RunGroupError> {
    let mut running = members
        .iter()
        .map(|member| async move { (member.name(), member.action().run(project).await) })
        .collect::<FuturesUnordered<_>>();

    let mut problems = 0;

    while let Some((name, outcome)) = running.next().await {
        if exit_code(&outcome) != EXIT_CLEAN {
            problems += 1;
        }

        report(name, outcome, clawless).await?;
    }

    Ok(problems)
}

/// Reports the outcome of one member under the name of the member
///
/// Each report is one event, so two reports never mix, even when the members
/// finish at the same time.
///
/// # Errors
///
/// Returns [`RunGroupError::UnreportedOutcome`] when nothing reads the output
/// anymore.
async fn report(
    name: &Name,
    outcome: Outcome,
    clawless: &ClawlessContext,
) -> Result<(), RunGroupError> {
    clawless
        .output()
        .artifact(Report::new(name.clone(), outcome))
        .await
        .map_err(|source| RunGroupError::UnreportedOutcome {
            member: name.clone(),
            source,
        })
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::future::{pending, poll_fn};
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

    /// The number of times that a test polls a run that never ends
    ///
    /// Each action of these tests answers on its second poll, unless it never
    /// answers. So this count reaches every member that a run starts, while a
    /// member that never answers keeps the run from its end.
    const POLLS: usize = 16;

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

        /// The action never answers, as a slow tool that still runs
        Wait,
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
                .expect("no action panics while it holds the log")
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
                Answer::Wait => pending().await,
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

    /// Polls a run of the actions without values a fixed number of times and
    /// returns the text of each report that it wrote by then, in the order in
    /// which it wrote them
    ///
    /// A run with a member that never answers never ends. The test then sees
    /// what the run did while that member still ran.
    fn advance(actions: &[Box<dyn ErasedAction>]) -> Vec<String> {
        let project = project();
        let (clawless, receiver) = clawless();

        {
            let mut future = pin!(run(
                bind(actions, &ArgsValues::empty()),
                &project,
                &clawless
            ));
            let mut task_context = TaskContext::from_waker(Waker::noop());

            for _ in 0..POLLS {
                let _state = future.as_mut().poll(&mut task_context);
            }
        }

        received(clawless, receiver)
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
    /// Each future that this function drives makes progress every time it is
    /// polled, so the loop needs no waker that wakes it. A run with a member
    /// that never answers never ends, so [`advance`] polls such a run instead.
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

    /// Returns the text of each report that a run wrote, in the order in which
    /// it wrote them
    ///
    /// The run has ended or was dropped, so nothing writes through the context
    /// anymore.
    fn received(clawless: ClawlessContext, mut receiver: EventReceiver) -> Vec<String> {
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
        let (clawless, receiver) = clawless();
        let _result = block_on(run(
            bind(actions, &ArgsValues::empty()),
            &project(),
            &clawless,
        ));

        received(clawless, receiver)
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

    /// Returns an action with the given name and declaration that never
    /// answers, and that writes into the given log
    fn waiting(
        name: Name,
        declaration: Declaration,
        log: &Arc<Mutex<Vec<String>>>,
    ) -> Box<dyn ErasedAction> {
        recorder(name, declaration, Answer::Wait, log)
    }

    /// Returns an action with the given name that writes and passes, and that
    /// writes into the given log
    fn writer(name: Name, log: &Arc<Mutex<Vec<String>>>) -> Box<dyn ErasedAction> {
        recorder(name, Declaration::Write, Answer::Pass, log)
    }

    /// Returns control to the driver of a future once, and then resolves
    ///
    /// The future asks to be polled again before it returns control, as a
    /// future does once the event that it waits for has happened. A run that
    /// drives several actions at once polls only the actions that asked for
    /// it.
    async fn yield_once() {
        let mut yielded = false;

        poll_fn(|task_context| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                task_context.waker().wake_by_ref();
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
    fn run_while_a_slower_reader_still_runs_reports_a_faster_reader() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            waiting(action_name!("test-rust"), Declaration::Read, &log),
            reader(action_name!("lint-yaml"), &log),
        ];

        let reports = advance(&actions);

        assert_eq!(reports, ["lint-yaml: passed"]);
    }

    #[test]
    fn run_while_a_writer_still_runs_drives_no_other_writer() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            waiting(action_name!("format-yaml"), Declaration::Write, &log),
            writer(action_name!("format-json"), &log),
            reader(action_name!("lint-yaml"), &log),
        ];

        let _reports = advance(&actions);

        assert_eq!(*log.lock().expect("the run was dropped"), ["format-yaml"]);
    }

    #[test]
    fn run_while_a_writer_still_runs_starts_no_reader() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
            waiting(action_name!("format-json"), Declaration::Write, &log),
        ];

        let _reports = advance(&actions);

        assert_eq!(
            *log.lock().expect("the run was dropped"),
            ["format-yaml", "format-json"]
        );
    }

    #[test]
    fn run_while_the_first_reader_still_runs_starts_every_reader() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            waiting(action_name!("test-rust"), Declaration::Read, &log),
            reader(action_name!("lint-yaml"), &log),
            reader(action_name!("lint-toml"), &log),
        ];

        let _reports = advance(&actions);
        let mut started = log.lock().expect("the run was dropped").clone();
        started.sort();

        assert_eq!(started, ["lint-toml", "lint-yaml", "test-rust"]);
    }

    #[test]
    fn run_with_actions_that_found_problems_counts_them() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            recorder(
                action_name!("format-yaml"),
                Declaration::Write,
                Answer::Fail,
                &log,
            ),
            writer(action_name!("format-json"), &log),
            failing(action_name!("lint-yaml"), &log),
            failing(action_name!("lint-toml"), &log),
        ];

        let result = complete(&actions, &ArgsValues::empty());

        assert_eq!(
            result.map_err(|error| error.to_string()),
            Err("3 of 4 actions found problems or stopped".to_owned())
        );
    }

    #[test]
    fn run_with_an_action_that_found_problems_drives_the_actions_after_it() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            recorder(
                action_name!("format-yaml"),
                Declaration::Write,
                Answer::Fail,
                &log,
            ),
            writer(action_name!("format-json"), &log),
            reader(action_name!("lint-yaml"), &log),
        ];

        let _result = complete(&actions, &ArgsValues::empty());

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "format-json", "lint-yaml"]
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
    fn run_with_several_writers_drives_them_in_the_order_of_the_list() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            writer(action_name!("format-yaml"), &log),
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-json"), &log),
        ];

        complete(&actions, &ArgsValues::empty()).expect("every action of the test passes");

        assert_eq!(
            *log.lock().expect("the run has ended"),
            ["format-yaml", "format-json", "lint-yaml"]
        );
    }

    #[test]
    fn run_with_writers_and_a_reader_reports_the_writers_first_in_the_order_of_the_list() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            reader(action_name!("lint-yaml"), &log),
            writer(action_name!("format-yaml"), &log),
            writer(action_name!("format-json"), &log),
        ];

        let reports = reports(&actions);

        assert_eq!(
            reports,
            [
                "format-yaml: passed",
                "format-json: passed",
                "lint-yaml: passed"
            ]
        );
    }

    #[test]
    fn run_without_an_output_fails_at_the_report_of_a_reader_while_a_slower_reader_still_runs() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let actions = [
            waiting(action_name!("test-rust"), Declaration::Read, &log),
            reader(action_name!("lint-yaml"), &log),
        ];
        let project = project();
        let (clawless, receiver) = clawless();
        drop(receiver);
        let mut future = pin!(run(
            bind(&actions, &ArgsValues::empty()),
            &project,
            &clawless
        ));
        let mut task_context = TaskContext::from_waker(Waker::noop());

        let result = (0..POLLS).find_map(|_| match future.as_mut().poll(&mut task_context) {
            Poll::Ready(result) => Some(result.map_err(|error| error.to_string())),
            Poll::Pending => None,
        });

        assert_eq!(
            result,
            Some(Err(
                "failed to report the outcome of the lint-yaml action".to_owned()
            ))
        );
    }

    #[test]
    fn run_without_an_output_fails_at_the_report_of_the_first_writer() {
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
