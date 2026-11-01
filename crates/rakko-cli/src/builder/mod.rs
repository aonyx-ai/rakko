/// The projection of the arguments of an action into flags
mod arguments;
/// The error of a run that the runner ended before it started
mod error;
/// The check, the flags, and the bind of the members of a group
mod members;
/// The actions that a harness mounted
mod registry;

use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use clap::error::ErrorKind;
use clap::{Arg, ArgMatches, Command, value_parser};
use clawless::CommandResult;
use clawless::context::Context as ClawlessContext;
use clawless::output::OutputFlags;
use clawless::runner::CommandRunner;
use rakko_action::{Access, ArgsSchema, ArgsValues, Context, ErasedAction, Outcome};

use self::error::RunEntryError;
use self::registry::{Mounted, Registry};
use crate::chain::Chain;
use crate::erased_command::ErasedCommand;
use crate::erased_group::ErasedGroup;
use crate::report::Report;
use crate::root::{self, ResolveProjectRootError};

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
/// with [`builder`], mounts the actions, the commands, and the groups that the
/// project uses, and then calls [`run`].
///
/// The command line of every project has the same shape, because one type
/// builds all of them. A harness cannot name a flag of its own.
///
/// [`run`]: Builder::run
#[derive(Debug)]
pub struct Builder {
    /// The actions, the commands, and the groups that the harness mounted
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
    /// Panics when a mounted action carries the name of another mounted action
    /// or of a mounted command or group, and reports that name. A run cannot
    /// choose between them, and only a change of the harness corrects it, so
    /// the failure happens here instead of at a run.
    ///
    /// Panics when an action declares an argument whose name the command line
    /// defines itself, and reports the action and the argument.
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
    /// #         Outcome::Passed { summary: None }
    /// #     }
    /// # }
    /// let command_line = rakko_cli::builder().mount([Box::new(LineCount) as Box<dyn ErasedAction>]);
    /// ```
    ///
    /// A bundle exports its actions as a [`Bundle`], and a harness mounts that
    /// list as it is. A second mount adds to the first, so a harness mounts a
    /// bundle beside the actions that it names itself:
    ///
    /// ```
    /// # use rakko_action::{Action, Context, ErasedAction, Name, Outcome, action_name};
    /// # mod rakko_baseline {
    /// #     use rakko_action::{Action, Bundle, Context, Name, Outcome, action_name};
    /// #     struct FormatJson;
    /// #     impl Action for FormatJson {
    /// #         type Args = ();
    /// #         fn name(&self) -> Name { action_name!("format-json") }
    /// #         async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    /// #             Outcome::Passed { summary: None }
    /// #         }
    /// #     }
    /// #     pub fn bundle() -> Bundle {
    /// #         Bundle::new(vec![Box::new(FormatJson)])
    /// #     }
    /// # }
    /// # struct CheckMsrv;
    /// # impl Action for CheckMsrv {
    /// #     type Args = ();
    /// #     fn name(&self) -> Name { action_name!("check-msrv") }
    /// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    /// #         Outcome::Passed { summary: None }
    /// #     }
    /// # }
    /// let command_line = rakko_cli::builder()
    ///     .mount(rakko_baseline::bundle())
    ///     .mount([Box::new(CheckMsrv) as Box<dyn ErasedAction>]);
    /// ```
    ///
    /// [`Bundle`]: rakko_action::Bundle
    #[must_use]
    pub fn mount(mut self, actions: impl IntoIterator<Item = Box<dyn ErasedAction>>) -> Self {
        self.add(actions.into_iter().map(Mounted::Action));

        self
    }

    /// Mounts commands in the command line
    ///
    /// A command is one that the harness wrote, for a maintenance activity of
    /// the project that does not fit an action. It takes its place in the
    /// same flat tree as the commands of the actions, and the help lists both
    /// as one list. A harness passes a list here as well, so a reader of the
    /// harness finds every activity of the project in one place.
    ///
    /// # Panics
    ///
    /// Panics when a mounted command carries the name of another mounted
    /// command or of a mounted action or group, and reports that name.
    ///
    /// Panics when a command declares an argument whose name the command line
    /// defines itself, and reports the command and the argument.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rakko_action::{Context, Name, action_name};
    /// # use rakko_cli::clawless::CommandResult;
    /// # use rakko_cli::clawless::context::Context as ClawlessContext;
    /// # use rakko_cli::{Command, ErasedCommand};
    /// # struct Serve;
    /// # impl Command for Serve {
    /// #     type Args = ();
    /// #     fn name(&self) -> Name { action_name!("serve") }
    /// #     async fn run(
    /// #         &self,
    /// #         _project: &Context,
    /// #         _clawless: &ClawlessContext,
    /// #         _args: &Self::Args,
    /// #     ) -> CommandResult {
    /// #         Ok(())
    /// #     }
    /// # }
    /// let command_line =
    ///     rakko_cli::builder().mount_commands([Box::new(Serve) as Box<dyn ErasedCommand>]);
    /// ```
    #[must_use]
    pub fn mount_commands(
        mut self,
        commands: impl IntoIterator<Item = Box<dyn ErasedCommand>>,
    ) -> Self {
        self.add(commands.into_iter().map(Mounted::Command));

        self
    }

    /// Mounts groups in the command line
    ///
    /// A group is a command that the harness wrote, which runs a list of
    /// actions, its members. The command line derives the flags of the group
    /// from the arguments of the members: each argument that a member declares
    /// becomes one flag, and an argument that several members declare stays
    /// one flag. A flag reaches every member that declares its argument, and
    /// no other member. The help of a flag shows one line for each member that
    /// declares its argument, in the order of the members, and each line
    /// starts with the name of the member.
    ///
    /// A group takes its place in the same flat tree as the actions and the
    /// commands. Its members need no mount of their own, so a harness that
    /// wants a command for one of them mounts that action as well.
    ///
    /// # Panics
    ///
    /// Panics when a group lists two members with the same name, and reports
    /// the group and the name.
    ///
    /// Panics when two members of a group declare one argument with two
    /// different shapes, and reports the group, both members, the argument,
    /// and both shapes.
    ///
    /// Panics when a member of a group declares an argument whose name the
    /// command line defines itself, and reports the group, the member, and
    /// the argument.
    ///
    /// Panics when a mounted group carries the name of another mounted group,
    /// command, or action, and reports that name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rakko_action::{Context, ErasedAction, Name, action_name};
    /// # use rakko_cli::clawless::CommandResult;
    /// # use rakko_cli::clawless::context::Context as ClawlessContext;
    /// # use rakko_cli::{BoundMember, ErasedGroup, Group};
    /// # struct CheckAll {
    /// #     actions: Vec<Box<dyn ErasedAction>>,
    /// # }
    /// # impl Group for CheckAll {
    /// #     fn name(&self) -> Name { action_name!("check-all") }
    /// #     fn members(&self) -> &[Box<dyn ErasedAction>] { &self.actions }
    /// #     async fn run(
    /// #         &self,
    /// #         _project: &Context,
    /// #         _clawless: &ClawlessContext,
    /// #         _members: Vec<BoundMember<'_>>,
    /// #     ) -> CommandResult {
    /// #         Ok(())
    /// #     }
    /// # }
    /// let check_all = CheckAll { actions: Vec::new() };
    ///
    /// let command_line =
    ///     rakko_cli::builder().mount_groups([Box::new(check_all) as Box<dyn ErasedGroup>]);
    /// ```
    #[must_use]
    pub fn mount_groups(mut self, groups: impl IntoIterator<Item = Box<dyn ErasedGroup>>) -> Self {
        self.add(groups.into_iter().map(|group| {
            self::members::refuse_repeated(&group.name(), group.members());
            let arguments = members::merge(&group.name(), group.members());

            Mounted::Group { group, arguments }
        }));

        self
    }

    /// Adds entries to the registry, after the checks that a mount runs
    ///
    /// Every mount ends here, so an action, a command, and a group pass the
    /// same checks and land in the same tree.
    fn add(&mut self, entries: impl IntoIterator<Item = Mounted>) {
        let entries: Vec<Mounted> = entries.into_iter().collect();

        arguments::refuse_reserved(&entries);

        self.registry.add(entries);
    }

    /// Runs the command line against the arguments of the process
    ///
    /// A request for help, and a request that the command line cannot read,
    /// end the process in this method. The user gets the message of the parser
    /// and the process gets an exit code. A run that names nothing ends here
    /// as well: the user gets the commands that the harness mounted, and the
    /// process gets the code of a clean run. A run that failed ends here too:
    /// the user gets the error and every cause of it, and the process gets the
    /// code of a run that could not answer, whether the failure stopped an
    /// action, a command, or a group.
    ///
    /// The method ends the process itself, so that a harness stays a `main`
    /// that names what the project mounts and returns nothing.
    pub fn run(self) {
        let (matches, mounted) = match self.resolve(std::env::args_os()) {
            Ok(Resolution::Entry(matches, mounted)) => (matches, mounted),
            Ok(Resolution::Commands(mut command)) => {
                std::process::exit(i32::from(show_commands(&mut command)));
            }
            Err(error) => error.exit(),
        };
        let entry = mounted.to_string();

        match dispatch(matches, mounted) {
            Ok(code) => std::process::exit(i32::from(code)),
            Err(error) => run_error(&entry, error.as_ref()).exit(),
        }
    }

    /// Parses the arguments and takes the action, the command, or the group
    /// that the run asks for
    ///
    /// A run that gives no argument names nothing, and it gets the command
    /// line back instead of an error, so that the caller lists the commands
    /// that the harness mounted.
    ///
    /// The method takes the arguments as a parameter, so that a test drives
    /// the command line without the arguments of the test process.
    ///
    /// # Errors
    ///
    /// Returns the error of the parser when the arguments do not describe a
    /// run, and when the user asked for help. Returns an error for a run that
    /// asks for no action, no command, and no group of the registry, which the
    /// command tree already prevents.
    fn resolve<I, T>(mut self, arguments: I) -> Result<Resolution, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Clone + Into<OsString>,
    {
        let mut command = self.command();

        let matches = match command.try_get_matches_from_mut(arguments) {
            Ok(matches) => matches,
            // The parser reports this kind for a run that gives no argument
            // at all, and for no other run, so it is the whole test for a run
            // that names nothing.
            Err(error) if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                return Ok(Resolution::Commands(Box::new(command)));
            }
            Err(error) => return Err(error),
        };

        let mounted = matches
            .subcommand()
            .and_then(|(name, _)| self.registry.take(name));

        match mounted {
            Some(mounted) => Ok(Resolution::Entry(matches, mounted)),
            None => Err(clap::Error::raw(
                ErrorKind::InvalidSubcommand,
                "the run asks for no action, no command, and no group\n",
            )),
        }
    }

    /// Returns the command line that the builder describes
    ///
    /// The parser behind the command line is an implementation detail. It
    /// appears in no signature that a harness or an action can see.
    ///
    /// The command of an action, a command that a harness wrote, and a group
    /// take the same shape here: a name, the flags of the arguments, and a
    /// place directly under the command line.
    fn command(&self) -> Command {
        let mut command = shell();

        for mounted in self.registry.entries() {
            command = command.subcommand(
                Command::new(mounted.name().get().to_owned()).args(arguments::render(mounted)),
            );
        }

        command
    }
}

/// What the arguments of a run describe
///
/// A run asks for one action, one written command, or one group, and the
/// command line drives it. A run that gives no argument asks for none of them.
/// It asked for nothing, so it gets the commands that the harness mounted, and
/// not a refusal.
///
/// The command line travels with that second answer. The parser holds the
/// only description of the tree that the builder assembled, and the help of
/// that tree is the list of the commands.
enum Resolution {
    /// The action, the written command, or the group that the run asks for,
    /// with the arguments that the parser read
    Entry(ArgMatches, Mounted),
    /// The command line of a run that names nothing
    Commands(Box<Command>),
}

/// Shows the commands that a harness mounted and returns the code of the run
///
/// The help of the command line lists the commands, so a user who wrote the
/// name of the harness and nothing else reads what the project runs. The run
/// drove no action and found no problem, so it is clean, and a workflow that
/// calls the harness without a command reads a code that says as much.
///
/// A reader that closed the stream loses the list. The parser drops its own
/// output in that case as well, and the run answers with its code either
/// way.
fn show_commands(command: &mut Command) -> u8 {
    let _ = command.print_help();

    EXIT_CLEAN
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
fn project_root() -> Arg {
    Arg::new(PROJECT_ROOT)
        .long(PROJECT_ROOT)
        .value_name("PATH")
        .value_parser(value_parser!(PathBuf))
        .global(true)
        .help("Take this directory as the root of the project instead of searching for one")
}

/// Runs one action, one command, or one group and returns the code of the run
///
/// The command line builds the context of a command, and this function turns
/// that context into the context of the project. A user who named the project
/// root gets that root, and every other run searches for the directory that
/// marks the project. The values of the flags reach the action or the command
/// as the text that the user wrote. Each member of a group receives the values
/// of the arguments that it declares, and no other value.
///
/// # Errors
///
/// Returns the error of the command line when it cannot build the context of
/// a command, and when it cannot start the runtime that drives the run.
/// Returns an error for a run that names no root and stands in no project,
/// because a run that receives a guessed root reports paths that mean nothing.
/// Returns an error for a group whose member cannot bind its values. Returns
/// the error of a command or a group that failed, and the error of a report
/// that cannot reach the reader.
fn dispatch(matches: ArgMatches, mounted: Mounted) -> Result<u8, Box<dyn Error>> {
    let named = matches.get_one::<PathBuf>(PROJECT_ROOT).cloned();

    match mounted {
        Mounted::Action(action) => {
            let values = values(&action.arguments(), &matches);

            drive_action(matches, named, values, action)
        }
        Mounted::Command(command) => {
            let values = values(&command.arguments(), &matches);

            drive_command(matches, named, values, command)
        }
        Mounted::Group { group, .. } => {
            let values = group
                .members()
                .iter()
                .map(|member| values(&member.arguments(), &matches))
                .collect();

            drive_group(matches, named, values, group)
        }
    }
}

/// Returns the values that a run holds for the arguments of one schema
///
/// The values come from the flags of the entry that the run asks for. A run
/// without an entry holds no values.
fn values(schema: &ArgsSchema, matches: &ArgMatches) -> ArgsValues {
    matches
        .subcommand()
        .map_or_else(ArgsValues::empty, |(_, command)| {
            arguments::collect(schema, command)
        })
}

/// Runs one action and returns the code of the run
///
/// The values of the run are bound to the arguments of the action first. Then
/// the bound action declares how the run uses the project, and then it runs.
/// Values that do not convert into the arguments end the run as an action that
/// stopped, with the error of the conversion. A kind of access that the
/// command line does not know ends the run in the same way, before the action
/// runs. Its error tells the user to update Rakko.
///
/// What the action returned reaches the reader as one report, and the flags
/// of the run decide whether that report renders as text or as JSON. The
/// report travels as the result of the command and not as a message, so the
/// flags that reduce the output do not suppress what the run found.
///
/// # Errors
///
/// Returns the errors that [`dispatch`] describes.
fn drive_action(
    matches: ArgMatches,
    named: Option<PathBuf>,
    values: ArgsValues,
    action: Box<dyn ErasedAction>,
) -> Result<u8, Box<dyn Error>> {
    drive(matches, move |context| async move {
        let project = locate(named, &context)?;

        let name = action.name();
        let outcome = match action.bind(&values) {
            Ok(bound) => match bound.access(&project) {
                Access::Read | Access::Write => bound.run(&project).await,
                _ => Outcome::Errored {
                    source: Box::new(RunEntryError::UnknownAccess {
                        action: name.clone(),
                    }),
                },
            },
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        };
        let code = exit_code(&outcome);

        context
            .output()
            .artifact(Report::new(name, outcome))
            .await?;

        Ok(code)
    })
}

/// Runs one command that the harness wrote and returns the code of the run
///
/// The command receives the context of the project and the context of
/// Clawless, and it writes what it has to say through the latter. A run that
/// ends in success is clean. A run that ends in an error carries that error
/// to the caller, which shows it and gives the process the code of a run
/// that could not answer, because the command line cannot tell a problem of
/// the project from a failure of the command.
///
/// # Errors
///
/// Returns the errors that [`dispatch`] describes.
fn drive_command(
    matches: ArgMatches,
    named: Option<PathBuf>,
    values: ArgsValues,
    command: Box<dyn ErasedCommand>,
) -> Result<u8, Box<dyn Error>> {
    drive(matches, move |context| async move {
        let project = locate(named, &context)?;

        command.run(&project, &context, &values).await?;

        Ok(EXIT_CLEAN)
    })
}

/// Runs one group that the harness wrote and returns the code of the run
///
/// The values hold one entry for each member of the group, in the order of
/// the members. Every member is bound to its values before the group runs, so
/// a member that cannot bind stops the run before any member runs. The group
/// then receives the bound members, and its result reaches the caller as the
/// result of a command does.
///
/// # Errors
///
/// Returns [`BindMembersError::UnboundMember`] for a member whose values do
/// not convert into its arguments. Returns the other errors that [`dispatch`]
/// describes.
///
/// [`BindMembersError::UnboundMember`]: members::BindMembersError::UnboundMember
fn drive_group(
    matches: ArgMatches,
    named: Option<PathBuf>,
    values: Vec<ArgsValues>,
    group: Box<dyn ErasedGroup>,
) -> Result<u8, Box<dyn Error>> {
    drive(matches, move |context| async move {
        let project = locate(named, &context)?;

        let members = members::bind(group.members(), &values)?;
        group.run(&project, &context, members).await?;

        Ok(EXIT_CLEAN)
    })
}

/// Runs one entry through the runner of Clawless and returns what it returned
///
/// The runner renders everything that the entry sends before it returns, and
/// it ends every run with an exit of its own. That exit holds an error only
/// as text, and it holds a code that the process cannot read back. The entry
/// therefore gives its result back beside the runner, so that the caller gets
/// the code that the entry chose and an error with every cause.
///
/// # Errors
///
/// Returns the error of the entry when it failed. Returns
/// [`RunEntryError::UnstartedRun`] when the runner ended the run before the
/// entry started.
fn drive<E, F>(matches: ArgMatches, entry: E) -> Result<u8, Box<dyn Error>>
where
    E: FnOnce(ClawlessContext) -> F,
    F: Future<Output = CommandResult<u8>> + Send + 'static,
{
    let result = Arc::new(Mutex::new(None));
    let returned = Arc::clone(&result);

    let exit = CommandRunner::run(matches, move |_matches, context| {
        let run = entry(context);

        async move {
            let outcome = run.await;
            returned
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .replace(outcome);

            Ok(())
        }
    });

    let outcome = result.lock().unwrap_or_else(PoisonError::into_inner).take();
    let Some(outcome) = outcome else {
        let reason = exit.text().unwrap_or_default();
        let reason = reason.strip_prefix("Error: ").unwrap_or(reason).trim_end();

        return Err(RunEntryError::UnstartedRun {
            reason: reason.to_owned(),
        }
        .into());
    };

    outcome.map_err(Into::into)
}

/// Returns the error of the command line that ends a run that failed
///
/// The message carries every cause of the error, because the error of a
/// command usually states only what failed, and the reason sits in its
/// source. The parser exits with the code of a run that could not answer for
/// every error of this kind, so the message and the code travel together.
fn run_error(entry: &str, error: &dyn Error) -> clap::Error {
    clap::Error::raw(
        ErrorKind::Io,
        format!("failed to run the {entry}: {}\n", Chain::new(error)),
    )
}

/// Returns the context of the project that a run maintains
///
/// A user who named the project root gets that root, and every other run
/// searches for the directory that marks the project, from the directory
/// that the run started in.
///
/// # Errors
///
/// Returns an error when the run names no root and stands in no project.
fn locate(
    named: Option<PathBuf>,
    context: &ClawlessContext,
) -> Result<Context, ResolveProjectRootError> {
    let root = root::resolve(named, context.current_working_directory().get())?;

    Ok(Context::builder().root(root).build())
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
fn exit_code(outcome: &Outcome) -> u8 {
    match outcome {
        Outcome::Passed { .. } | Outcome::Changed { .. } | Outcome::Skipped { .. } => EXIT_CLEAN,
        Outcome::Failed { .. } => EXIT_FINDINGS,
        Outcome::Errored { .. } => EXIT_UNANSWERED,
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::marker::PhantomData;
    use std::path::Path;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use clap::builder::{Str, ValueRange};
    use clawless::CommandResult;
    use rakko_action::{
        Action, Args, Argument, ArgumentShape, ArgumentValue, Name, ReadArgsError, SkipReason,
        argument_name,
    };

    use super::*;
    use crate::bound_member::BoundMember;
    use crate::command::Command as WrittenCommand;
    use crate::group::Group;

    /// The name of the group that the tests mount
    const GROUP: &str = "check";

    /// The log into which the members of a group write the values that their
    /// runs received, in the order of their runs
    type Log = Arc<Mutex<Vec<(Name, ArgsValues)>>>;

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
            Outcome::Passed { summary: None }
        }
    }

    /// The arguments that a member of a group declares
    ///
    /// The description of an argument set belongs to its type, so each set of
    /// arguments that a test member declares is a type of its own.
    trait Declaration: Send + Sync + 'static {
        /// Returns the arguments that the member declares
        fn schema() -> ArgsSchema;
    }

    /// The declaration of a member that takes a boolean `fix`
    struct Fix;

    impl Declaration for Fix {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([argument("fix", ArgumentShape::Boolean, FIX)])
        }
    }

    /// The declaration of a member that takes `fix` as a text
    struct FixAsText;

    impl Declaration for FixAsText {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([argument(
                "fix",
                ArgumentShape::Text,
                "Repair the files that match this pattern",
            )])
        }
    }

    /// The arguments of a member, which keep the values that its run received
    struct Kept<D> {
        /// The values that the run gave to the member
        values: ArgsValues,
        /// The declaration that describes the arguments
        declaration: PhantomData<D>,
    }

    impl<D: Declaration> Args for Kept<D> {
        fn schema() -> ArgsSchema {
            D::schema()
        }

        fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Ok(Self {
                values: values.clone(),
                declaration: PhantomData,
            })
        }
    }

    /// A member of a group that writes the values of its run into a log
    struct Member<D> {
        /// The name that identifies this action
        name: Name,
        /// The log that every member of a test writes into
        log: Log,
        /// The declaration that describes the arguments of the member
        declaration: PhantomData<D>,
    }

    impl<D: Declaration> Action for Member<D> {
        type Args = Kept<D>;

        fn name(&self) -> Name {
            self.name.clone()
        }

        async fn run(&self, _context: &Context, args: &Self::Args) -> Outcome {
            self.log
                .lock()
                .expect("the test runs one member at a time")
                .push((self.name.clone(), args.values.clone()));

            Outcome::Passed { summary: None }
        }
    }

    /// The declaration of a member that takes no argument
    struct Nothing;

    impl Declaration for Nothing {
        fn schema() -> ArgsSchema {
            ArgsSchema::empty()
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
            Self::reporting(name, || Outcome::Passed { summary: None })
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

    /// An action that declares a read, and otherwise runs as the probe that it
    /// wraps
    struct Reading {
        /// The probe that gives the action its name and runs it
        probe: Probe,
        /// Whether a caller asked the action for its access
        asked: Arc<AtomicBool>,
    }

    impl Reading {
        /// Creates an action that declares a read, with the flag that its run
        /// sets and the flag that a question for its access sets
        fn new(name: &str) -> (Self, Arc<AtomicBool>, Arc<AtomicBool>) {
            let (probe, ran) = Probe::new(name);
            let asked = Arc::new(AtomicBool::new(false));
            let reading = Self {
                probe,
                asked: Arc::clone(&asked),
            };

            (reading, ran, asked)
        }
    }

    impl Action for Reading {
        type Args = ();

        fn name(&self) -> Name {
            Action::name(&self.probe)
        }

        fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
            self.asked.store(true, Ordering::SeqCst);

            Access::Read
        }

        async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
            Action::run(&self.probe, context, args).await
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

            Outcome::Passed { summary: None }
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

            Outcome::Passed { summary: None }
        }
    }

    /// The declaration of a member that takes the path of a report
    struct ReportPath;

    impl Declaration for ReportPath {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([argument(
                "report",
                ArgumentShape::Path,
                "Write the report to this file",
            )])
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

    /// A command that declares an argument that the command line carries
    struct Reserving;

    impl WrittenCommand for Reserving {
        type Args = Reserved;

        fn name(&self) -> Name {
            "reserving"
                .parse()
                .expect("the test names a command correctly")
        }

        async fn run(
            &self,
            _project: &Context,
            _clawless: &ClawlessContext,
            _args: &Self::Args,
        ) -> CommandResult {
            Ok(())
        }
    }

    /// A group that runs its members one after another and returns what a
    /// test asked for
    struct Sequence {
        /// The name of the group
        name: Name,
        /// The actions that the group runs
        actions: Vec<Box<dyn ErasedAction>>,
        /// Whether a run of the group happened
        ran: Arc<AtomicBool>,
        /// What a run of the group returns
        ///
        /// An error cannot be cloned, and a run borrows the group, so the
        /// group holds a function that builds a fresh result per run.
        result: fn() -> CommandResult,
    }

    impl Group for Sequence {
        fn name(&self) -> Name {
            self.name.clone()
        }

        fn members(&self) -> &[Box<dyn ErasedAction>] {
            &self.actions
        }

        async fn run(
            &self,
            project: &Context,
            _clawless: &ClawlessContext,
            members: Vec<BoundMember<'_>>,
        ) -> CommandResult {
            self.ran.store(true, Ordering::SeqCst);

            for member in members {
                member.action().run(project).await;
            }

            (self.result)()
        }
    }

    /// An action whose arguments never convert from the values of a run
    struct Unbuildable;

    impl Action for Unbuildable {
        type Args = Unreadable;

        fn name(&self) -> Name {
            "unbuildable"
                .parse()
                .expect("the test names an action correctly")
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

    /// What a run of a command saw
    #[derive(Default)]
    struct Witness {
        /// Whether a run happened
        ran: AtomicBool,
        /// The root of the project that the run received
        root: Mutex<Option<PathBuf>>,
        /// The values that the run received
        values: Mutex<Option<ArgsValues>>,
    }

    /// A command that records its run and returns what a test asked for
    struct Written {
        /// The name that identifies this command
        name: Name,
        /// Where a run records what it saw
        witness: Arc<Witness>,
        /// What a run of this command returns
        ///
        /// An error cannot be cloned, and a run borrows the command, so the
        /// command holds a function that builds a fresh result per run.
        result: fn() -> CommandResult,
    }

    impl Written {
        /// Creates a command with the given name that succeeds
        fn new(name: &str) -> (Self, Arc<Witness>) {
            Self::returning(name, || Ok(()))
        }

        /// Creates a command that returns what the given function builds
        fn returning(name: &str, result: fn() -> CommandResult) -> (Self, Arc<Witness>) {
            let witness = Arc::new(Witness::default());
            let written = Self {
                name: name.parse().expect("the test names a command correctly"),
                witness: Arc::clone(&witness),
                result,
            };

            (written, witness)
        }
    }

    impl WrittenCommand for Written {
        type Args = Arguments;

        fn name(&self) -> Name {
            self.name.clone()
        }

        async fn run(
            &self,
            project: &Context,
            _clawless: &ClawlessContext,
            args: &Self::Args,
        ) -> CommandResult {
            self.witness.ran.store(true, Ordering::SeqCst);

            let mut root = self
                .witness
                .root
                .lock()
                .expect("the test holds the lock alone");
            *root = Some(project.root().get().to_path_buf());

            let mut values = self
                .witness
                .values
                .lock()
                .expect("the test holds the lock alone");
            *values = Some(args.values.clone());

            (self.result)()
        }
    }

    /// Returns the name of an action
    fn action_name(name: &str) -> Name {
        name.parse().expect("the name of the test action is valid")
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

        dispatch(naming(directory.path()), Mounted::Action(Box::new(probe)))
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

    /// Returns the flags of the group that a command line mounts
    fn flags_of_group(group: Sequence) -> Vec<Arg> {
        builder()
            .mount_groups([Box::new(group) as Box<dyn ErasedGroup>])
            .command()
            .get_subcommands()
            .find(|leaf| leaf.get_name() == GROUP)
            .expect("the test mounts the group")
            .get_arguments()
            .cloned()
            .collect()
    }

    /// Returns one erased member of a group with the given name
    ///
    /// The member declares the arguments that `D` describes, and its run
    /// writes the values that it received into the log.
    fn member<D: Declaration>(name: &str, log: &Log) -> Box<dyn ErasedAction> {
        Box::new(Member::<D> {
            name: name.parse().expect("the name of the test action is valid"),
            log: Arc::clone(log),
            declaration: PhantomData,
        })
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

    /// Returns what a run of the group with the given arguments gives back
    ///
    /// The run gives its project root, so that the test drives the group
    /// without a directory tree that marks a project.
    ///
    /// # Errors
    ///
    /// Returns the error of the run when it failed.
    fn run_group(group: Sequence, arguments: &[&str]) -> Result<u8, Box<dyn Error>> {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let mut invocation: Vec<OsString> = vec![
            "rakko".into(),
            "--project-root".into(),
            directory.path().into(),
            GROUP.into(),
        ];
        invocation.extend(arguments.iter().map(OsString::from));

        let Ok(Resolution::Entry(matches, mounted)) = builder()
            .mount_groups([Box::new(group) as Box<dyn ErasedGroup>])
            .resolve(invocation)
        else {
            panic!("expected the command line to read the run that the test describes");
        };

        dispatch(matches, mounted)
    }

    /// Returns a group of the given members that succeeds, and the flag that
    /// its run sets
    fn sequence(actions: Vec<Box<dyn ErasedAction>>) -> (Sequence, Arc<AtomicBool>) {
        sequence_returning(actions, || Ok(()))
    }

    /// Returns a group of the given members that returns what the given
    /// function builds, and the flag that its run sets
    fn sequence_returning(
        actions: Vec<Box<dyn ErasedAction>>,
        result: fn() -> CommandResult,
    ) -> (Sequence, Arc<AtomicBool>) {
        let ran = Arc::new(AtomicBool::new(false));
        let group = Sequence {
            name: action_name(GROUP),
            actions,
            ran: Arc::clone(&ran),
            result,
        };

        (group, ran)
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

        let Ok(Resolution::Entry(matches, action)) = builder()
            .mount([Box::new(reader) as Box<dyn ErasedAction>])
            .resolve(invocation)
        else {
            panic!("expected the command line to read the run that the test names");
        };

        dispatch(matches, action).expect("expected the command line to drive the action");

        seen.lock()
            .expect("the test holds the lock alone")
            .clone()
            .expect("expected the action to have run")
    }

    /// Returns the values that a run of the given arguments gives to the command
    ///
    /// The run names its project root, so that the test drives the command
    /// without a directory tree that marks a project.
    fn values_of_command(arguments: &[&str]) -> ArgsValues {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (written, witness) = Written::new("serve");

        let mut invocation: Vec<OsString> = vec![
            "rakko".into(),
            "--project-root".into(),
            directory.path().into(),
        ];
        invocation.extend(arguments.iter().map(OsString::from));

        let Ok(Resolution::Entry(matches, mounted)) = builder()
            .mount_commands([Box::new(written) as Box<dyn ErasedCommand>])
            .resolve(invocation)
        else {
            panic!("expected the command line to read the run that the test names");
        };

        dispatch(matches, mounted).expect("expected the command line to drive the command");

        witness
            .values
            .lock()
            .expect("the test holds the lock alone")
            .clone()
            .expect("expected the command to have run")
    }

    /// Returns what a run of a command that returns `result` gives back
    ///
    /// The run names its project root, so that the test drives the command
    /// without a directory tree that marks a project.
    ///
    /// # Errors
    ///
    /// Returns the error of the command when it failed.
    fn result_of(result: fn() -> CommandResult) -> Result<u8, Box<dyn Error>> {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (written, _witness) = Written::returning("serve", result);

        dispatch(
            naming(directory.path()),
            Mounted::Command(Box::new(written)),
        )
    }

    /// Returns one erased command with the given name
    fn written(name: &str) -> Box<dyn ErasedCommand> {
        let (written, _witness) = Written::new(name);

        Box::new(written)
    }

    #[test]
    fn builder_returns_a_builder_of_a_command_line() {
        let builder = builder();

        let name = builder.command().get_name().to_owned();

        assert_eq!(name, NAME);
    }

    #[test]
    fn command_gives_a_boolean_argument_no_value() {
        let flag = flag_of("fix");

        let values = flag.get_num_args();

        assert_eq!(values, Some(ValueRange::EMPTY));
    }

    #[test]
    fn command_gives_an_argument_a_long_flag() {
        let flag = flag_of("fix");

        let long = flag.get_long();

        assert_eq!(long, Some("fix"));
    }

    #[test]
    fn command_gives_an_argument_of_a_command_a_long_flag() {
        let mut command = builder().mount_commands([written("serve")]).command();
        command.build();

        let long = command
            .get_subcommands()
            .find(|leaf| leaf.get_name() == "serve")
            .expect("the test mounts the command")
            .get_arguments()
            .find(|flag| flag.get_id().as_str() == "extension")
            .expect("the test names an argument of that command")
            .get_long();

        assert_eq!(long, Some("extension"));
    }

    #[test]
    fn command_gives_an_argument_that_holds_a_number_a_value() {
        let flag = flag_of("jobs");

        let values = flag.get_num_args();

        assert_eq!(values, Some(ValueRange::SINGLE));
    }

    #[test]
    fn command_gives_each_argument_of_a_command_a_flag() {
        let command = builder().mount_commands([written("serve")]).command();

        let longs: Vec<&str> = command
            .get_subcommands()
            .find(|leaf| leaf.get_name() == "serve")
            .expect("the test mounts the command")
            .get_arguments()
            .filter_map(Arg::get_long)
            .collect();

        assert_eq!(longs, ["fix", "jobs", "report", "extension"]);
    }

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

    #[test]
    fn command_refuses_a_value_that_is_not_a_number() {
        let builder = builder().mount([reader()]);

        let Err(error) = builder.resolve(["rakko", READER, "--jobs", "many"]) else {
            panic!("expected the run to report an error");
        };

        assert_eq!(error.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn command_shows_the_documentation_of_an_argument() {
        let flag = flag_of("fix");

        let help = flag.get_help().map(ToString::to_string);

        assert_eq!(help, Some(FIX.to_owned()));
    }

    #[test]
    fn command_refuses_a_run_that_gives_an_argument_and_names_no_action() {
        let kind = error_kind(&["rakko", "--json"]);

        assert_eq!(kind, ErrorKind::MissingSubcommand);
    }

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

    #[test]
    fn dispatch_asks_the_action_for_its_access() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (reading, _ran, asked) = Reading::new("probe");

        dispatch(naming(directory.path()), Mounted::Action(Box::new(reading)))
            .expect("expected the command line to drive the action");

        assert!(asked.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_does_not_run_a_group_whose_member_cannot_bind() {
        let log = Log::default();
        let (group, ran) = sequence(vec![member::<Fix>("first", &log), Box::new(Unbuildable)]);

        let _result = run_group(group, &["--fix"]);

        assert!(!ran.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_gives_the_action_the_project_root_that_the_user_names() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let seen = Arc::new(Mutex::new(None));
        let recorder = Recorder {
            seen: Arc::clone(&seen),
        };

        dispatch(
            naming(directory.path()),
            Mounted::Action(Box::new(recorder)),
        )
        .expect("expected the command line to drive the action");

        let root = seen.lock().expect("the test holds the lock alone").clone();
        let named = rakko_action::path::canonical(directory.path())
            .expect("the test names a directory that exists");
        assert_eq!(root, Some(named));
    }

    #[test]
    fn dispatch_gives_the_command_the_project_root_that_the_user_names() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (written, witness) = Written::new("serve");

        dispatch(
            naming(directory.path()),
            Mounted::Command(Box::new(written)),
        )
        .expect("expected the command line to drive the command");

        let root = witness
            .root
            .lock()
            .expect("the test holds the lock alone")
            .clone();
        let named = rakko_action::path::canonical(directory.path())
            .expect("the test names a directory that exists");
        assert_eq!(root, Some(named));
    }

    #[test]
    fn dispatch_reports_a_group_whose_project_root_cannot_be_read() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (group, _ran) = sequence(Vec::new());
        let mounted = Mounted::Group {
            group: Box::new(group),
            arguments: Vec::new(),
        };

        let result = dispatch(naming(&directory.path().join("missing")), mounted);

        assert!(result.is_err());
    }

    #[test]
    fn dispatch_reports_a_nonzero_code_for_an_action_that_found_problems() {
        let code = code_for(|| Outcome::Failed {
            findings: Vec::new(),
            repairs: Vec::new(),
        });

        assert_ne!(code, 0);
    }

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

    #[test]
    fn dispatch_reports_a_nonzero_code_for_an_action_that_stopped() {
        let code = code_for(|| Outcome::Errored {
            source: Box::new(std::io::Error::other("boom")),
        });

        assert_ne!(code, 0);
    }

    #[test]
    fn dispatch_reports_an_action_whose_values_do_not_convert_as_stopped() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let code = dispatch(
            naming(directory.path()),
            Mounted::Action(Box::new(Unbuildable)),
        )
        .expect("expected the command line to drive the action");

        assert_eq!(code, EXIT_UNANSWERED);
    }

    #[test]
    fn dispatch_reports_every_cause_of_a_command_that_failed() {
        let Err(error) = result_of(|| {
            Err(clawless::Error::msg("the port is taken").context("failed to bind the server"))
        }) else {
            panic!("expected the run to report an error");
        };

        let message = run_error("command 'serve'", error.as_ref()).to_string();

        assert_eq!(
            message,
            "error: failed to run the command 'serve': failed to bind the server: the port is taken\n"
        );
    }

    #[test]
    fn dispatch_reports_every_cause_of_a_group_that_failed() {
        let (group, _ran) = sequence_returning(Vec::new(), || {
            Err(clawless::Error::msg("the hook is missing").context("failed to guard the commit"))
        });

        let message = run_group(group, &[])
            .map_err(|error| run_error("group 'check'", error.as_ref()).to_string());

        assert_eq!(
            message,
            Err(
                "error: failed to run the group 'check': failed to guard the commit: the hook is \
                 missing\n"
                    .to_owned()
            )
        );
    }

    #[test]
    fn dispatch_reports_every_cause_of_a_project_root_that_cannot_be_read() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let missing = directory.path().join("missing");
        let cause = rakko_action::path::canonical(&missing)
            .expect_err("the test names a directory that does not exist");
        let (probe, _ran) = Probe::new("probe");
        let Err(error) = dispatch(naming(&missing), Mounted::Action(Box::new(probe))) else {
            panic!("expected the run to report an error");
        };

        let message = run_error("action 'probe'", error.as_ref()).to_string();

        assert_eq!(
            message,
            format!(
                "error: failed to run the action 'probe': \
                 failed to read the project root at `{}`: {cause}\n",
                missing.display()
            )
        );
    }

    #[test]
    fn dispatch_reports_the_member_and_the_argument_of_a_member_that_cannot_bind() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![member::<Fix>("first", &log), Box::new(Unbuildable)]);

        let message = run_group(group, &["--fix"])
            .map_err(|error| run_error("group 'check'", error.as_ref()).to_string());

        assert_eq!(
            message,
            Err(
                "error: failed to run the group 'check': failed to bind the values of the run to \
                 the action 'unbuildable': no value for the required argument 'fix'\n"
                    .to_owned()
            )
        );
    }

    #[test]
    fn dispatch_reports_zero_for_a_command_that_succeeded() {
        let code = result_of(|| Ok(())).expect("expected the command line to drive the command");

        assert_eq!(code, 0);
    }

    #[test]
    fn dispatch_reports_zero_for_a_group_that_succeeded() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![member::<Fix>("first", &log)]);

        let code = run_group(group, &[]).expect("expected the command line to drive the group");

        assert_eq!(code, 0);
    }

    #[test]
    fn dispatch_reports_zero_for_a_passed_action() {
        let code = code_for(|| Outcome::Passed { summary: None });

        assert_eq!(code, 0);
    }

    #[test]
    fn dispatch_reports_zero_for_an_action_that_does_not_apply() {
        let code = code_for(|| Outcome::Skipped {
            reason: SkipReason::new("this project has no TOML file"),
        });

        assert_eq!(code, 0);
    }

    #[test]
    fn dispatch_reports_zero_for_an_action_that_repaired_the_project() {
        let code = code_for(|| Outcome::Changed {
            repairs: Vec::new(),
        });

        assert_eq!(code, 0);
    }

    #[test]
    fn dispatch_runs_an_action_that_declares_a_read() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (reading, ran, _asked) = Reading::new("probe");

        dispatch(naming(directory.path()), Mounted::Action(Box::new(reading)))
            .expect("expected the command line to drive the action");

        assert!(ran.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_runs_no_member_of_a_group_whose_member_cannot_bind() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![member::<Fix>("first", &log), Box::new(Unbuildable)]);

        let _result = run_group(group, &["--fix"]);

        let runs = log.lock().expect("the test holds the lock alone").len();
        assert_eq!(runs, 0);
    }

    #[test]
    fn dispatch_runs_the_action_that_it_gets() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (probe, ran) = Probe::new("probe");

        dispatch(naming(directory.path()), Mounted::Action(Box::new(probe)))
            .expect("expected the command line to drive the action");

        assert!(ran.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_runs_the_command_that_it_gets() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (written, witness) = Written::new("serve");

        dispatch(
            naming(directory.path()),
            Mounted::Command(Box::new(written)),
        )
        .expect("expected the command line to drive the command");

        assert!(witness.ran.load(Ordering::SeqCst));
    }

    #[test]
    fn mount_commands_builds_a_flat_command_tree() {
        let command = builder()
            .mount_commands([written("serve"), written("watch")])
            .command();

        let nested = command
            .get_subcommands()
            .any(|leaf| leaf.get_subcommands().next().is_some());

        assert!(!nested);
    }

    #[test]
    fn mount_commands_gives_each_command_a_place_beside_the_actions() {
        let command = builder()
            .mount([probe("lint-rust")])
            .mount_commands([written("format-all"), written("serve")])
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, ["format-all", "lint-rust", "serve"]);
    }

    #[test]
    #[should_panic(expected = "json")]
    fn mount_commands_refuses_a_command_whose_argument_the_command_line_carries() {
        let _builder = builder().mount_commands([Box::new(Reserving) as Box<dyn ErasedCommand>]);
    }

    #[test]
    #[should_panic(expected = "format-toml")]
    fn mount_commands_refuses_a_command_with_the_name_of_an_action() {
        let _builder = builder()
            .mount([probe("format-toml")])
            .mount_commands([written("format-toml")]);
    }

    #[test]
    #[should_panic(expected = "serve")]
    fn mount_commands_refuses_two_commands_with_one_name() {
        let _builder = builder().mount_commands([written("serve"), written("serve")]);
    }

    #[test]
    fn mount_gives_each_action_a_command() {
        let command = builder()
            .mount([probe("format-toml"), probe("lint-rust")])
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, ["format-toml", "lint-rust"]);
    }

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

    #[test]
    fn mount_groups_accepts_a_member_that_the_harness_also_mounts() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![member::<Fix>("first", &log)]);
        let command = builder()
            .mount([member::<Fix>("first", &log)])
            .mount_groups([Box::new(group) as Box<dyn ErasedGroup>])
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, [GROUP, "first"]);
    }

    #[test]
    fn mount_groups_accepts_two_groups_that_share_a_member() {
        let log = Log::default();
        let (check, _ran) = sequence(vec![member::<Fix>("first", &log)]);
        let (lint, _ran) = sequence(vec![member::<Fix>("first", &log)]);
        let lint = Sequence {
            name: action_name("lint"),
            ..lint
        };
        let command = builder()
            .mount_groups([
                Box::new(check) as Box<dyn ErasedGroup>,
                Box::new(lint) as Box<dyn ErasedGroup>,
            ])
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, [GROUP, "lint"]);
    }

    #[test]
    fn mount_groups_gives_a_group_one_flag_for_an_argument_that_members_share() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![
            member::<Fix>("first", &log),
            member::<Fix>("second", &log),
            member::<Nothing>("third", &log),
        ]);

        let flags = flags_of_group(group);

        let longs: Vec<&str> = flags.iter().filter_map(Arg::get_long).collect();
        assert_eq!(longs, ["fix"]);
    }

    #[test]
    fn mount_groups_gives_each_group_a_place_beside_the_actions_and_the_commands() {
        let (group, _ran) = sequence(Vec::new());
        let command = builder()
            .mount([probe("lint-rust")])
            .mount_commands([written("serve")])
            .mount_groups([Box::new(group) as Box<dyn ErasedGroup>])
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, [GROUP, "lint-rust", "serve"]);
    }

    #[test]
    fn mount_groups_orders_the_flags_as_the_members_first_declare_them() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![member::<ReportPath>("first", &log), reader()]);

        let flags = flags_of_group(group);

        let names: Vec<&str> = flags.iter().map(|flag| flag.get_id().as_str()).collect();
        assert_eq!(names, ["report", "fix", "jobs", "extension"]);
    }

    #[test]
    #[should_panic(expected = "the group 'check' lists the action 'first' twice")]
    fn mount_groups_refuses_a_group_that_lists_an_action_twice() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![
            member::<Fix>("first", &log),
            member::<Nothing>("second", &log),
            member::<Fix>("first", &log),
        ]);

        let _builder = builder().mount_groups([Box::new(group) as Box<dyn ErasedGroup>]);
    }

    #[test]
    #[should_panic(expected = "the group 'check' lists the action 'first' twice")]
    fn mount_groups_refuses_a_group_that_lists_two_actions_with_one_name() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![
            member::<Fix>("first", &log),
            member::<FixAsText>("first", &log),
        ]);

        let _builder = builder().mount_groups([Box::new(group) as Box<dyn ErasedGroup>]);
    }

    #[test]
    #[should_panic(
        expected = "the harness mounts two actions, commands, or groups with the name 'check'"
    )]
    fn mount_groups_refuses_a_group_with_the_name_of_an_action() {
        let (group, _ran) = sequence(Vec::new());

        let _builder = builder()
            .mount([probe(GROUP)])
            .mount_groups([Box::new(group) as Box<dyn ErasedGroup>]);
    }

    #[test]
    #[should_panic(
        expected = "the action 'colliding' in the group 'check' declares the argument 'json', \
                    which the command line defines itself"
    )]
    fn mount_groups_refuses_a_member_whose_argument_the_command_line_carries() {
        let (group, _ran) = sequence(vec![Box::new(Colliding)]);

        let _builder = builder().mount_groups([Box::new(group) as Box<dyn ErasedGroup>]);
    }

    #[test]
    #[should_panic(
        expected = "the group 'check' cannot merge the argument 'fix', because the action 'first' \
                    declares it as a boolean and the action 'second' declares it as a text"
    )]
    fn mount_groups_refuses_members_that_declare_one_argument_with_two_shapes() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![
            member::<Fix>("first", &log),
            member::<FixAsText>("second", &log),
        ]);

        let _builder = builder().mount_groups([Box::new(group) as Box<dyn ErasedGroup>]);
    }

    #[test]
    fn mount_groups_shows_each_line_of_a_flag_under_the_help_column() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![
            member::<Fix>("first", &log),
            member::<Fix>("second", &log),
        ]);
        let mut command = builder()
            .mount_groups([Box::new(group) as Box<dyn ErasedGroup>])
            .command();
        command.build();

        let help = command
            .find_subcommand_mut(GROUP)
            .expect("the test mounts the group")
            .render_help()
            .to_string();

        let column = |member: &str| {
            help.lines()
                .find_map(|line| line.find(&format!("{member}: ")))
                .expect("expected the help to show a line for the member")
        };
        assert_eq!(column("first"), column("second"));
    }

    #[test]
    fn mount_groups_shows_one_line_for_each_member_that_declares_an_argument() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![
            member::<Fix>("first", &log),
            member::<Nothing>("second", &log),
            member::<Fix>("third", &log),
        ]);

        let flags = flags_of_group(group);

        let help: Vec<String> = flags
            .iter()
            .filter_map(|flag| flag.get_help().map(ToString::to_string))
            .collect();
        assert_eq!(help, [format!("first: {FIX}\nthird: {FIX}")]);
    }

    #[test]
    #[should_panic(expected = "format-toml")]
    fn mount_refuses_two_actions_with_one_name() {
        let _builder = builder().mount([probe("format-toml"), probe("format-toml")]);
    }

    #[test]
    #[should_panic(expected = "json")]
    fn mount_refuses_an_action_whose_argument_the_command_line_carries() {
        let _builder = builder().mount([Box::new(Colliding) as Box<dyn ErasedAction>]);
    }

    #[test]
    fn resolve_returns_the_action_that_the_run_names() {
        let builder = builder().mount([probe("format-toml"), probe("lint-rust")]);

        let Ok(Resolution::Entry(_matches, Mounted::Action(action))) =
            builder.resolve(["rakko", "lint-rust"])
        else {
            panic!("expected the run to resolve an action");
        };

        assert_eq!(action.name().get(), "lint-rust");
    }

    #[test]
    fn resolve_returns_the_command_that_the_run_names() {
        let builder = builder()
            .mount([probe("lint-rust")])
            .mount_commands([written("serve")]);

        let Ok(Resolution::Entry(_matches, Mounted::Command(command))) =
            builder.resolve(["rakko", "serve"])
        else {
            panic!("expected the run to resolve a command");
        };

        assert_eq!(command.name().get(), "serve");
    }

    #[test]
    fn resolve_returns_the_group_that_the_run_names() {
        let (group, _ran) = sequence(Vec::new());
        let builder = builder()
            .mount([probe("lint-rust")])
            .mount_groups([Box::new(group) as Box<dyn ErasedGroup>]);

        let Ok(Resolution::Entry(_matches, Mounted::Group { group, .. })) =
            builder.resolve(["rakko", GROUP])
        else {
            panic!("expected the run to resolve a group");
        };

        assert_eq!(group.name().get(), GROUP);
    }

    #[test]
    fn run_error_returns_the_code_of_a_run_that_could_not_answer() {
        let error = std::io::Error::other("the port is taken");

        let code = run_error("command 'serve'", &error).exit_code();

        assert_eq!(code, i32::from(EXIT_UNANSWERED));
    }

    #[test]
    fn run_gives_a_flag_of_a_group_to_the_members_that_declare_its_argument() {
        let log = Log::default();
        let (group, _ran) = sequence(vec![
            member::<Fix>("first", &log),
            member::<Fix>("second", &log),
            member::<Nothing>("third", &log),
        ]);

        run_group(group, &["--fix"]).expect("expected the command line to drive the group");

        let fix = || ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);
        let runs = log.lock().expect("the test holds the lock alone").clone();
        assert_eq!(
            runs,
            [
                (action_name("first"), fix()),
                (action_name("second"), fix()),
                (action_name("third"), ArgsValues::empty()),
            ]
        );
    }

    #[test]
    fn run_gives_the_action_no_value_for_a_boolean_that_the_user_left_out() {
        let values = values_of(&[READER]);

        let value = values.get(&argument_name!("fix"));

        assert_eq!(value, None);
    }

    #[test]
    fn run_gives_the_action_no_value_for_a_flag_that_the_user_left_out() {
        let values = values_of(&[READER]);

        let value = values.get(&argument_name!("extension"));

        assert_eq!(value, None);
    }

    #[test]
    fn run_gives_the_action_the_path_that_the_user_wrote() {
        let values = values_of(&[READER, "--report", "target/report.json"]);

        let value = values.get(&argument_name!("report"));

        assert_eq!(value, Some(&ArgumentValue::new("target/report.json")));
    }

    #[test]
    fn run_gives_the_action_the_text_that_the_user_wrote() {
        let values = values_of(&[READER, "--jobs", "007"]);

        let value = values.get(&argument_name!("jobs"));

        assert_eq!(value, Some(&ArgumentValue::new("007")));
    }

    #[test]
    fn run_gives_the_command_the_text_that_the_user_wrote() {
        let values = values_of_command(&["serve", "--extension", "md"]);

        let value = values.get(&argument_name!("extension"));

        assert_eq!(value, Some(&ArgumentValue::new("md")));
    }

    #[test]
    fn run_gives_the_action_true_for_a_boolean_that_the_user_gave() {
        let values = values_of(&[READER, "--fix"]);

        let value = values.get(&argument_name!("fix"));

        assert_eq!(value, Some(&ArgumentValue::new("true")));
    }

    #[test]
    fn resolve_shows_the_help_for_a_run_without_arguments() {
        let Ok(Resolution::Commands(mut command)) = builder().resolve(["rakko"]) else {
            panic!("expected the run to ask for the commands");
        };

        let help = command.render_help().to_string();

        assert!(help.contains(ABOUT));
    }

    #[test]
    fn resolve_reports_a_run_that_it_cannot_read() {
        let kind = error_kind(&["rakko", "--unknown"]);

        assert_eq!(kind, ErrorKind::UnknownArgument);
    }

    #[test]
    fn show_commands_reports_a_clean_run() {
        let mut command = builder().command();

        let code = show_commands(&mut command);

        assert_eq!(code, EXIT_CLEAN);
    }
}
