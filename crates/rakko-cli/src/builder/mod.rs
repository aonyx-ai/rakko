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
use registry::Registry;

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
        let mut command = Command::new(NAME)
            .about(ABOUT)
            .arg_required_else_help(true)
            .subcommand_required(true)
            .arg(project_root());

        for action in self.registry.actions() {
            command = command.subcommand(Command::new(action.name().get().to_owned()));
        }

        OutputFlags::augment_command(command)
    }
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
// cli[impl root.named]
// cli[impl run.action]
fn dispatch(matches: ArgMatches, action: Box<dyn ErasedAction>) -> Result<u8, Box<dyn Error>> {
    let code = Arc::new(AtomicU8::new(EXIT_UNANSWERED));
    let reported = Arc::clone(&code);
    let named = matches.get_one::<PathBuf>(PROJECT_ROOT).cloned();

    CommandRunner::run(matches, move |_matches, context| async move {
        let root = root::resolve(named, context.current_working_directory().get())?;
        let project = Context::builder().root(root).build();

        let name = action.name();
        let outcome = action.run(&project, &ArgsValues::empty()).await;

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
/// that project red.
// cli[impl exit.clean]
// cli[impl exit.findings]
// cli[impl exit.unanswered]
fn exit_code(outcome: &Outcome) -> u8 {
    match outcome {
        Outcome::Passed | Outcome::Skipped { .. } => EXIT_CLEAN,
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

    use rakko_action::{Action, Name, SkipReason};

    use super::*;

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

    // cli[verify builder.create]
    #[test]
    fn builder_returns_a_builder_of_a_command_line() {
        let builder = builder();

        let name = builder.command().get_name().to_owned();

        assert_eq!(name, NAME);
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
        });

        assert_ne!(code, 0);
    }

    // cli[verify exit.unanswered]
    #[test]
    fn dispatch_reports_another_code_for_an_action_that_stopped() {
        let findings = code_for(|| Outcome::Failed {
            findings: Vec::new(),
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

    // cli[verify mount.list]
    #[test]
    fn resolve_returns_the_action_that_the_run_names() {
        let builder = builder().mount([probe("format-toml"), probe("lint-rust")]);

        let Ok((_matches, action)) = builder.resolve(["rakko", "lint-rust"]) else {
            panic!("expected the run to resolve an action");
        };

        assert_eq!(action.name().get(), "lint-rust");
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
