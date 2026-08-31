/// The projection of the arguments of an action into flags
mod arguments;
/// The actions that a harness mounted
mod registry;

use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use clap::error::ErrorKind;
use clap::{Arg, ArgMatches, Command, value_parser};
use clawless::output::OutputFlags;
use clawless::runner::CommandRunner;
use rakko_action::{ArgsValues, Context, ErasedAction, Outcome};

use self::registry::Registry;
use crate::report::Report;
use crate::root;

/// The description that the command line shows above its help
const ABOUT: &str = "Runs the maintenance actions of this project";

/// The code that a run gives back when the project is clean
const EXIT_CLEAN: u8 = 0;

/// The code that a run gives back when its action found problems
const EXIT_FINDINGS: u8 = 1;

/// The code that a run gives back when it could not get an answer
///
/// The parser gives this code to a command line that it cannot read. A run
/// that never reached an action and a run whose action stopped are the same
/// event for whoever reads the result, so one code covers both.
const EXIT_UNANSWERED: u8 = 2;

/// The name of the argument with which a user names the project root
///
/// The name is also the long flag, so a reader of a workflow file sees
/// `--project-root` and finds this argument in the help.
const PROJECT_ROOT: &str = "project-root";

/// The name that the command line carries
///
/// A reader sees the name of the harness binary instead of this name, because
/// clap takes the name of a usage line from the command that started the
/// process. This name is the fallback for the places that have nothing else.
const NAME: &str = "rakko";

/// Creates the builder of the command line of a harness
///
/// A harness calls this function in its `main`, mounts what the project uses,
/// and runs the result.
///
/// # Examples
///
/// ```no_run
/// rakko_cli::builder().run();
/// ```
// cli[impl builder.create]
#[must_use]
pub fn builder() -> Builder {
    Builder {
        registry: Registry::default(),
    }
}

/// The command line of a harness
///
/// A harness is the small binary that a project runs to maintain itself. This
/// type is the whole surface that the harness touches: the harness creates it
/// with [`builder`], mounts the actions that the project uses, and then calls
/// [`run`].
///
/// The command line of every project has the same shape, because one type
/// builds all of them. A harness cannot name a flag of its own.
///
/// [`run`]: Builder::run
#[derive(Debug)]
pub struct Builder {
    /// The actions that the harness mounted
    registry: Registry,
}

impl Builder {
    /// Mounts actions in the command line
    ///
    /// Each action becomes one command, and the command tree stays flat. A
    /// harness passes a list, and a list is an ordinary value: the harness can
    /// mount the list that a bundle exports, join two lists, or leave one
    /// action out. Reading the harness therefore reports what the project
    /// runs.
    ///
    /// # Panics
    ///
    /// Panics when two mounted actions carry one name, and reports that name.
    /// A run cannot choose between them, and only a change of the harness
    /// corrects it, so the failure happens here instead of at a run.
    ///
    /// Panics when an action declares an argument whose name the command line
    /// carries itself, and reports the action and the argument.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rakko_action::{Action, Context, ErasedAction, Name, Outcome, action_name};
    /// # struct LineCount;
    /// # impl Action for LineCount {
    /// #     type Args = ();
    /// #     fn name(&self) -> Name { action_name!("line-count") }
    /// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    /// #         Outcome::Passed
    /// #     }
    /// # }
    /// let command_line = rakko_cli::builder().mount([Box::new(LineCount) as Box<dyn ErasedAction>]);
    /// ```
    // cli[impl mount.list]
    // cli[impl mount.collision]
    #[must_use]
    pub fn mount(mut self, actions: impl IntoIterator<Item = Box<dyn ErasedAction>>) -> Self {
        let actions: Vec<Box<dyn ErasedAction>> = actions.into_iter().collect();

        arguments::refuse_reserved(&actions);

        self.registry.add(actions);

        self
    }

    /// Runs the command line against the arguments of the process
    ///
    /// A request for help, and a request that the command line cannot read,
    /// end the process in this method. The user gets the message of the parser
    /// and the process gets an exit code.
    ///
    /// The method ends the process itself, so that a harness stays a `main`
    /// that names what the project mounts and returns nothing.
    // cli[impl builder.run]
    pub fn run(self) {
        let (matches, action) = match self.resolve(std::env::args_os()) {
            Ok(resolved) => resolved,
            Err(error) => error.exit(),
        };

        match dispatch(matches, action) {
            Ok(code) => std::process::exit(i32::from(code)),
            Err(error) => clap::Error::raw(
                ErrorKind::Io,
                format!("failed to run the action: {error}\n"),
            )
            .exit(),
        }
    }

    /// Parses the arguments and takes the action that the run names
    ///
    /// The method takes the arguments as a parameter, so that a test drives
    /// the command line without the arguments of the test process.
    ///
    /// # Errors
    ///
    /// Returns the error of the parser when the arguments do not describe a
    /// run, and when the user asked for help. Returns an error for a run that
    /// names no action of the registry, which the command tree already
    /// prevents.
    fn resolve<I, T>(
        mut self,
        arguments: I,
    ) -> Result<(ArgMatches, Box<dyn ErasedAction>), clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Clone + Into<OsString>,
    {
        let matches = self.command().try_get_matches_from(arguments)?;

        let action = matches
            .subcommand()
            .and_then(|(name, _)| self.registry.take(name));

        match action {
            Some(action) => Ok((matches, action)),
            None => Err(clap::Error::raw(
                ErrorKind::InvalidSubcommand,
                "the run names no action\n",
            )),
        }
    }

    /// Returns the command line that the builder describes
    ///
    /// The parser behind the command line is an implementation detail. It
    /// appears in no signature that a harness or an action can see.
    // cli[impl command.action]
    // cli[impl command.help]
    // cli[impl command.output]
    // cli[impl mount.flat]
    fn command(&self) -> Command {
        let mut command = shell();

        for action in self.registry.actions() {
            command = command.subcommand(
                Command::new(action.name().get().to_owned())
                    .args(arguments::render(&action.arguments())),
            );
        }

        command
    }
}

/// Returns the command line without the command of any action
///
/// The flags of the command line are global, so the parser gives every one of
/// them to the command of every action. The builder adds the command of an
/// action to this command, and the check that a mount runs reads the flags
/// from it.
fn shell() -> Command {
    OutputFlags::augment_command(
        Command::new(NAME)
            .about(ABOUT)
            .arg_required_else_help(true)
            .subcommand_required(true)
            .arg(project_root()),
    )
}

/// Returns the argument with which a user names the project root
///
/// The argument is global, so it reads the same before and after the name of
/// an action. A user reaches for it when the search cannot answer, which is
/// why the help describes what it replaces.
// cli[impl root.named]
fn project_root() -> Arg {
    Arg::new(PROJECT_ROOT)
        .long(PROJECT_ROOT)
        .value_name("PATH")
        .value_parser(value_parser!(PathBuf))
        .global(true)
        .help("Take this directory as the root of the project instead of searching for one")
}

/// Runs one action and returns the code of the run
///
/// The command line builds the context of a command, and this function turns
/// that context into the context of an action. A user who named the project
/// root gets that root, and every other run searches for the directory that
/// marks the project.
///
/// What the action returned reaches the reader as one report, and the flags
/// of the run decide whether that report renders as text or as JSON. The report travels
/// as the result of the command and not as a message, so the flags that reduce
/// the output do not suppress what the run found.
///
/// # Errors
///
/// Returns the error of the command line when it cannot build the context of
/// a command, when it cannot start the runtime that drives the action, and when
/// the report cannot reach the reader. Returns an error for a run that names no
/// root and stands in no project, because an action that receives a guessed
/// root reports paths that mean nothing.
// cli[impl argument.values]
// cli[impl root.named]
// cli[impl run.action]
fn dispatch(matches: ArgMatches, action: Box<dyn ErasedAction>) -> Result<u8, Box<dyn Error>> {
    let code = Arc::new(AtomicU8::new(EXIT_UNANSWERED));
    let reported = Arc::clone(&code);
    let named = matches.get_one::<PathBuf>(PROJECT_ROOT).cloned();
    let values = matches
        .subcommand()
        .map_or_else(ArgsValues::empty, |(_, command)| {
            arguments::collect(&action.arguments(), command)
        });

    CommandRunner::run(matches, move |_matches, context| async move {
        let root = root::resolve(named, context.current_working_directory().get())?;
        let project = Context::builder().root(root).build();

        let name = action.name();
        let outcome = action.run(&project, &values).await;

        reported.store(exit_code(&outcome), Ordering::SeqCst);

        context
            .output()
            .artifact(Report::new(name, outcome))
            .await?;

        Ok(())
    })?;

    Ok(code.load(Ordering::SeqCst))
}

/// Returns the code that a run gives back for the given outcome
///
/// A run that does not apply is clean, because a skip is an answer: a bundle
/// that mounts an action for a stack that a project does not use must not turn
/// that project red. A run that repaired the project is clean as well: the
/// caller asked for the repair, and the repairs travel in the report, so a
/// step that runs a fix in CI must not go red over its success. A run that
/// repaired some of what it found still has a problem, so it takes the code
/// for a problem.
// cli[impl exit.clean]
// cli[impl exit.changed+2]
// cli[impl exit.findings]
// cli[impl exit.unanswered]
fn exit_code(outcome: &Outcome) -> u8 {
    match outcome {
        Outcome::Passed | Outcome::Changed { .. } | Outcome::Skipped { .. } => EXIT_CLEAN,
        Outcome::Failed { .. } => EXIT_FINDINGS,
        Outcome::Errored { .. } => EXIT_UNANSWERED,
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;

    use clap::builder::{Str, ValueRange};
    use rakko_action::{
        Action, Args, ArgsSchema, Argument, ArgumentShape, ArgumentValue, Name, ReadArgsError,
        SkipReason, argument_name,
    };

    use super::*;

    /// The documentation of the boolean argument that the reader reads
    const FIX: &str = "Rewrite the files that the tool can format";

    /// The name of the action that reads an argument of every shape
    const READER: &str = "reader";

    /// The arguments that the reader reads
    ///
    /// The set holds one argument of every shape, and it keeps the values that
    /// it was built from, so that a test reads what the command line
    /// collected.
    #[derive(Clone, Eq, PartialEq, Debug)]
    struct Arguments {
        /// The values that the run gave to the action
        values: ArgsValues,
    }

    impl Args for Arguments {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([
                argument("fix", ArgumentShape::Boolean, FIX),
                argument(
                    "jobs",
                    ArgumentShape::Integer,
                    "Examine this many files at once",
                ),
                argument(
                    "report",
                    ArgumentShape::Path,
                    "Write the report to this file",
                ),
                argument(
                    "extension",
                    ArgumentShape::Text,
                    "Examine only the files with this extension",
                ),
            ])
        }

        fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Ok(Self {
                values: values.clone(),
            })
        }
    }

    /// An action that declares an argument that the command line carries
    struct Colliding;

    impl Action for Colliding {
        type Args = Reserved;

        fn name(&self) -> Name {
            "colliding"
                .parse()
                .expect("the test names an action correctly")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed
        }
    }

    /// An action that records that it ran and reports what a test asked for
    struct Probe {
        /// The name that identifies this action
        name: Name,
        /// Whether a run of this action happened
        ran: Arc<AtomicBool>,
        /// What a run of this action reports
        ///
        /// An outcome cannot be cloned, and a run borrows the action, so the
        /// action holds a function that builds a fresh outcome per run.
        outcome: fn() -> Outcome,
    }

    impl Probe {
        /// Creates an action with the given name, and the flag that it sets
        fn new(name: &str) -> (Self, Arc<AtomicBool>) {
            Self::reporting(name, || Outcome::Passed)
        }

        /// Creates an action that reports what the given function builds
        fn reporting(name: &str, outcome: fn() -> Outcome) -> (Self, Arc<AtomicBool>) {
            let ran = Arc::new(AtomicBool::new(false));
            let probe = Self {
                name: name.parse().expect("the test names an action correctly"),
                ran: Arc::clone(&ran),
                outcome,
            };

            (probe, ran)
        }
    }

    impl Action for Probe {
        type Args = ();

        fn name(&self) -> Name {
            self.name.clone()
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            self.ran.store(true, Ordering::SeqCst);

            (self.outcome)()
        }
    }

    /// An action that records the values that its run received
    struct Reader {
        /// Where a run stores the values that it received
        seen: Arc<Mutex<Option<ArgsValues>>>,
    }

    impl Action for Reader {
        type Args = Arguments;

        fn name(&self) -> Name {
            READER.parse().expect("the test names an action correctly")
        }

        async fn run(&self, _context: &Context, args: &Self::Args) -> Outcome {
            let mut seen = self.seen.lock().expect("the test holds the lock alone");
            *seen = Some(args.values.clone());

            Outcome::Passed
        }
    }

    /// An action that records the project root that its run received
    struct Recorder {
        /// Where a run stores the root that it received
        seen: Arc<Mutex<Option<PathBuf>>>,
    }

    impl Action for Recorder {
        type Args = ();

        fn name(&self) -> Name {
            "recorder"
                .parse()
                .expect("the test names an action correctly")
        }

        async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
            let mut seen = self.seen.lock().expect("the test holds the lock alone");
            *seen = Some(context.root().get().to_path_buf());

            Outcome::Passed
        }
    }

    /// The arguments of an action that declares a name of the command line
    struct Reserved;

    impl Args for Reserved {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([argument(
                "json",
                ArgumentShape::Boolean,
                "Report what the action found as JSON",
            )])
        }

        fn from_values(_values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Ok(Self)
        }
    }

    /// Returns the description of an argument
    fn argument(name: &str, shape: ArgumentShape, documentation: &str) -> Argument {
        Argument::builder()
            .name(name.parse().expect("the test names an argument correctly"))
            .shape(shape)
            .documentation(documentation)
            .build()
    }

    /// Returns the code that a run of an action reporting `outcome` gives back
    ///
    /// The run names its project root, so that the test drives the action
    /// without a directory tree that marks a project.
    fn code_for(outcome: fn() -> Outcome) -> u8 {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (probe, _ran) = Probe::reporting("probe", outcome);

        dispatch(naming(directory.path()), Box::new(probe))
            .expect("expected the command line to drive the action")
    }

    /// Returns the flag that the command of the reader carries for an argument
    ///
    /// The command is built first, because the parser fills in how many values
    /// a flag takes while it builds.
    fn flag_of(name: &str) -> Arg {
        let mut command = builder().mount([reader()]).command();
        command.build();

        command
            .get_subcommands()
            .find(|action| action.get_name() == READER)
            .expect("the test mounts the action that reads arguments")
            .get_arguments()
            .find(|flag| flag.get_id().as_str() == name)
            .expect("the test names an argument of that action")
            .clone()
    }

    /// Returns the matches of a run that names the given directory as its root
    fn naming(root: &Path) -> ArgMatches {
        OutputFlags::augment_command(Command::new(NAME).arg(project_root())).get_matches_from([
            "rakko".as_ref(),
            "--project-root".as_ref(),
            root.as_os_str(),
        ])
    }

    /// Returns the kind of the error that a run of the given arguments gives
    fn error_kind(arguments: &[&str]) -> ErrorKind {
        let Err(error) = builder().resolve(arguments) else {
            panic!("expected the run to report an error");
        };

        error.kind()
    }

    /// Returns one erased action with the given name
    fn probe(name: &str) -> Box<dyn ErasedAction> {
        let (probe, _ran) = Probe::new(name);

        Box::new(probe)
    }

    /// Returns one erased action that reads an argument of every shape
    fn reader() -> Box<dyn ErasedAction> {
        Box::new(Reader {
            seen: Arc::new(Mutex::new(None)),
        })
    }

    /// Returns the values that a run of the given arguments gives to the action
    ///
    /// The run names its project root, so that the test drives the action
    /// without a directory tree that marks a project.
    fn values_of(arguments: &[&str]) -> ArgsValues {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let seen = Arc::new(Mutex::new(None));
        let reader = Reader {
            seen: Arc::clone(&seen),
        };

        let mut invocation: Vec<OsString> = vec![
            "rakko".into(),
            "--project-root".into(),
            directory.path().into(),
        ];
        invocation.extend(arguments.iter().map(OsString::from));

        let (matches, action) = builder()
            .mount([Box::new(reader) as Box<dyn ErasedAction>])
            .resolve(invocation)
            .expect("the test names a run that the command line reads");

        dispatch(matches, action).expect("expected the command line to drive the action");

        seen.lock()
            .expect("the test holds the lock alone")
            .clone()
            .expect("expected the action to have run")
    }

    // cli[verify builder.create]
    #[test]
    fn builder_returns_a_builder_of_a_command_line() {
        let builder = builder();

        let name = builder.command().get_name().to_owned();

        assert_eq!(name, NAME);
    }

    // cli[verify argument.switch]
    #[test]
    fn command_gives_a_boolean_argument_no_value() {
        let flag = flag_of("fix");

        let values = flag.get_num_args();

        assert_eq!(values, Some(ValueRange::EMPTY));
    }

    // cli[verify argument.flag]
    #[test]
    fn command_gives_an_argument_a_long_flag() {
        let flag = flag_of("fix");

        let long = flag.get_long();

        assert_eq!(long, Some("fix"));
    }

    // cli[verify argument.value]
    #[test]
    fn command_gives_an_argument_that_holds_a_number_a_value() {
        let flag = flag_of("jobs");

        let values = flag.get_num_args();

        assert_eq!(values, Some(ValueRange::SINGLE));
    }

    // cli[verify argument.value]
    #[test]
    fn command_names_the_value_of_an_argument_after_the_argument() {
        let flag = flag_of("extension");

        let value: Vec<&str> = flag
            .get_value_names()
            .unwrap_or_default()
            .iter()
            .map(Str::as_str)
            .collect();

        assert_eq!(value, ["EXTENSION"]);
    }

    // cli[verify argument.integer]
    #[test]
    fn command_refuses_a_value_that_is_not_a_number() {
        let builder = builder().mount([reader()]);

        let Err(error) = builder.resolve(["rakko", READER, "--jobs", "many"]) else {
            panic!("expected the run to report an error");
        };

        assert_eq!(error.kind(), ErrorKind::ValueValidation);
    }

    // cli[verify argument.help]
    #[test]
    fn command_shows_the_documentation_of_an_argument() {
        let flag = flag_of("fix");

        let help = flag.get_help().map(ToString::to_string);

        assert_eq!(help, Some(FIX.to_owned()));
    }

    // cli[verify command.action]
    #[test]
    fn command_refuses_a_run_that_names_no_action() {
        let kind = error_kind(&["rakko", "--json"]);

        assert_eq!(kind, ErrorKind::MissingSubcommand);
    }

    // cli[verify command.output]
    #[test]
    fn command_carries_the_flags_that_control_the_output() {
        let command = builder().command();

        let flags: Vec<&str> = command
            .get_arguments()
            .map(|argument| argument.get_id().as_str())
            .collect();

        assert!(
            ["quiet", "verbose", "json"]
                .iter()
                .all(|flag| flags.contains(flag))
        );
    }

    // cli[verify root.named]
    #[test]
    fn dispatch_gives_the_action_the_project_root_that_the_user_names() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let seen = Arc::new(Mutex::new(None));
        let recorder = Recorder {
            seen: Arc::clone(&seen),
        };

        dispatch(naming(directory.path()), Box::new(recorder))
            .expect("expected the command line to drive the action");

        let root = seen.lock().expect("the test holds the lock alone").clone();
        let named = directory
            .path()
            .canonicalize()
            .expect("the test names a directory that exists");
        assert_eq!(root, Some(named));
    }

    // cli[verify exit.findings]
    #[test]
    fn dispatch_reports_a_nonzero_code_for_an_action_that_found_problems() {
        let code = code_for(|| Outcome::Failed {
            findings: Vec::new(),
            repairs: Vec::new(),
        });

        assert_ne!(code, 0);
    }

    // cli[verify exit.unanswered]
    #[test]
    fn dispatch_reports_another_code_for_an_action_that_stopped() {
        let findings = code_for(|| Outcome::Failed {
            findings: Vec::new(),
            repairs: Vec::new(),
        });
        let stopped = code_for(|| Outcome::Errored {
            source: Box::new(std::io::Error::other("boom")),
        });

        assert_ne!(findings, stopped);
    }

    // cli[verify exit.unanswered]
    #[test]
    fn dispatch_reports_a_nonzero_code_for_an_action_that_stopped() {
        let code = code_for(|| Outcome::Errored {
            source: Box::new(std::io::Error::other("boom")),
        });

        assert_ne!(code, 0);
    }

    // cli[verify exit.clean]
    #[test]
    fn dispatch_reports_zero_for_a_passed_action() {
        let code = code_for(|| Outcome::Passed);

        assert_eq!(code, 0);
    }

    // cli[verify exit.clean]
    #[test]
    fn dispatch_reports_zero_for_an_action_that_does_not_apply() {
        let code = code_for(|| Outcome::Skipped {
            reason: SkipReason::new("this project has no TOML file"),
        });

        assert_eq!(code, 0);
    }

    // cli[verify exit.changed+2]
    #[test]
    fn dispatch_reports_zero_for_an_action_that_repaired_the_project() {
        let code = code_for(|| Outcome::Changed {
            repairs: Vec::new(),
        });

        assert_eq!(code, 0);
    }

    // cli[verify run.action]
    #[test]
    fn dispatch_runs_the_action_that_it_gets() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (probe, ran) = Probe::new("probe");

        dispatch(naming(directory.path()), Box::new(probe))
            .expect("expected the command line to drive the action");

        assert!(ran.load(Ordering::SeqCst));
    }

    // cli[verify mount.list]
    #[test]
    fn mount_gives_each_action_a_command() {
        let command = builder()
            .mount([probe("format-toml"), probe("lint-rust")])
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, ["format-toml", "lint-rust"]);
    }

    // cli[verify mount.flat]
    #[test]
    fn mount_builds_a_flat_command_tree() {
        let command = builder()
            .mount([probe("format-toml"), probe("lint-rust")])
            .command();

        let nested = command
            .get_subcommands()
            .any(|action| action.get_subcommands().next().is_some());

        assert!(!nested);
    }

    // cli[verify mount.collision]
    #[test]
    #[should_panic(expected = "format-toml")]
    fn mount_refuses_two_actions_with_one_name() {
        let _builder = builder().mount([probe("format-toml"), probe("format-toml")]);
    }

    // cli[verify mount.reserved]
    #[test]
    #[should_panic(expected = "json")]
    fn mount_refuses_an_action_whose_argument_the_command_line_carries() {
        let _builder = builder().mount([Box::new(Colliding) as Box<dyn ErasedAction>]);
    }

    // cli[verify mount.list]
    #[test]
    fn resolve_returns_the_action_that_the_run_names() {
        let builder = builder().mount([probe("format-toml"), probe("lint-rust")]);

        let Ok((_matches, action)) = builder.resolve(["rakko", "lint-rust"]) else {
            panic!("expected the run to resolve an action");
        };

        assert_eq!(action.name().get(), "lint-rust");
    }

    // cli[verify argument.absent]
    #[test]
    fn run_gives_the_action_no_value_for_a_boolean_that_the_user_left_out() {
        let values = values_of(&[READER]);

        let value = values.get(&argument_name!("fix"));

        assert_eq!(value, None);
    }

    // cli[verify argument.absent]
    #[test]
    fn run_gives_the_action_no_value_for_a_flag_that_the_user_left_out() {
        let values = values_of(&[READER]);

        let value = values.get(&argument_name!("extension"));

        assert_eq!(value, None);
    }

    // cli[verify argument.values]
    #[test]
    fn run_gives_the_action_the_path_that_the_user_wrote() {
        let values = values_of(&[READER, "--report", "target/report.json"]);

        let value = values.get(&argument_name!("report"));

        assert_eq!(value, Some(&ArgumentValue::new("target/report.json")));
    }

    // cli[verify argument.values]
    #[test]
    fn run_gives_the_action_the_text_that_the_user_wrote() {
        let values = values_of(&[READER, "--jobs", "007"]);

        let value = values.get(&argument_name!("jobs"));

        assert_eq!(value, Some(&ArgumentValue::new("007")));
    }

    // cli[verify argument.values]
    #[test]
    fn run_gives_the_action_true_for_a_boolean_that_the_user_gave() {
        let values = values_of(&[READER, "--fix"]);

        let value = values.get(&argument_name!("fix"));

        assert_eq!(value, Some(&ArgumentValue::new("true")));
    }

    // cli[verify command.help]
    #[test]
    fn resolve_shows_the_help_for_a_run_without_arguments() {
        let Err(error) = builder().resolve(["rakko"]) else {
            panic!("expected the run to report an error");
        };

        let help = error.render().to_string();

        assert!(help.contains(ABOUT));
    }

    // cli[verify builder.run]
    #[test]
    fn resolve_reports_a_run_that_it_cannot_read() {
        let kind = error_kind(&["rakko", "--unknown"]);

        assert_eq!(kind, ErrorKind::UnknownArgument);
    }
}
