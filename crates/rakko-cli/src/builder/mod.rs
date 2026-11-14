//! The command line that a harness builds and runs

mod arguments;
mod error;
mod members;
mod mount;
mod registry;
mod scheduler;
mod tool_lines;

use std::collections::BTreeSet;
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
use rakko::{Access, ArgsSchema, ArgsValues, Context, ErasedAction, Group, Outcome};

use self::error::RunEntryError;
pub use self::mount::Mount;
use self::registry::{Mounted, Registry};
use self::tool_lines::ToolLines;
use crate::chain::Chain;
use crate::doctor::{self, Declaration};
use crate::erased_command::ErasedCommand;
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
/// event for whoever reads the result. So one code covers both.
const EXIT_UNANSWERED: u8 = 2;

/// The name of the argument with which a user names the project root
///
/// The name is also the long flag, so a reader of a workflow file sees
/// `--project-root` and finds this argument in the help.
const PROJECT_ROOT: &str = "project-root";

/// The name that the command line carries
///
/// A reader sees the name of the harness binary instead of this name. This is
/// because clap takes the name of a usage line from the command that started
/// the process. This name is the fallback for the places that have nothing else.
const NAME: &str = "rakko";

/// Creates the builder of the command line of a harness
///
/// A harness calls this function in its `main`, mounts what the project uses,
/// and runs the result.
///
/// # Examples
///
/// ```no_run
/// fn main() -> std::process::ExitCode {
///     rakko_cli::builder().run()
/// }
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
/// type is the whole surface that the harness touches. The harness creates it
/// with [`builder`], mounts the actions, the commands, and the groups that the
/// project uses, and then calls [`run`].
///
/// The command line of every project has the same shape, because one type
/// builds all of them. A harness cannot name a flag of its own. Every command
/// line also carries `doctor`. It reports whether the tool pins of the
/// project match the tools that the mounted actions declare. With `--fix`, it
/// repairs them. A run of an action, a command, or a group checks the tool
/// pins in the same way before it starts. A problem stops the run. The help
/// and the list of the commands run without the check.
///
/// # Examples
///
/// ```no_run
/// use std::process::ExitCode;
///
/// fn main() -> ExitCode {
///     rakko_cli::builder().mount([]).run()
/// }
/// ```
///
/// [`run`]: Builder::run
#[derive(Debug)]
pub struct Builder {
    /// The actions, the commands, and the groups that the harness mounted
    registry: Registry,
}

impl Builder {
    /// Mounts an action, a list of actions, a command, or a group in the
    /// command line
    ///
    /// A harness gives each value as it is, and the builder hides its type
    /// itself, so a harness writes no erased type. [`Mount`] lists the kinds of
    /// value that the method takes. Each action, command, and group becomes
    /// one command, and the command tree stays flat. The commands of the
    /// actions, the commands that the harness wrote, and the commands of the
    /// groups share one list, in the help as well. Each call adds to the calls
    /// before it, so a reader of the harness finds every activity of the
    /// project in one place.
    ///
    /// An action becomes one command. A list of actions gives each action of
    /// the list a command. A list is an ordinary value: the harness can mount
    /// the list that a bundle exports, join two lists, or leave one action out.
    /// Reading the harness therefore reports what the project runs.
    ///
    /// A command is one that the harness wrote, with the [`Command`] trait, for
    /// a maintenance activity of the project that does not fit an action. It
    /// takes its place beside the commands of the actions.
    ///
    /// A [`Group`] is a named list of actions, its members, that the command
    /// line runs as one command. The command line derives the flags of the
    /// group from the arguments of the members. Each argument that a member
    /// declares becomes one flag. An argument that several members declare
    /// stays one flag. A flag reaches every member that declares its argument,
    /// and no other member. A group has no arguments of its own, so it defines
    /// no flag. The help of a flag shows one line for each member that declares
    /// its argument, in the order of the members. Each line starts with the
    /// name of the member and continues with the sentence that the member
    /// wrote for the argument.
    ///
    /// A run of the group binds every member to the values of the run first.
    /// When a member cannot bind its values, no member runs, and the error
    /// gives the member and the argument. Then the run asks every member how
    /// its run uses the project, as [`Access`] declares it. When a member
    /// declares a kind of access that the command line does not know, no
    /// member runs either. The error tells the user to update Rakko.
    ///
    /// Then the members that write run, one at a time and in the order of the
    /// list. A member that declares nothing writes. The members that read
    /// start after the last of them finished, so that they examine the project
    /// as the writers left it. They all start at the same time and share the
    /// run. Only one of them runs code at a time, and they overlap while they
    /// wait, such as for a tool. So a member that blocks the thread delays the
    /// others.
    ///
    /// Two members that read can collide when they share state that their
    /// declarations do not count as a write. An example is the cache of a tool
    /// that the tool does not lock. A harness whose members collide in this
    /// way takes one of them out of the group.
    ///
    /// Every member runs to its end, also after another member found problems
    /// or stopped, so one run reports every problem. The report of each member
    /// appears as a message when the member finishes, with the text that a run
    /// of that action alone shows. So the reports of the members that read
    /// follow the order in which they finish, and not the order of the list.
    ///
    /// A run of the group answers as a run of one action does. When every
    /// member finished, the run shows one summary of the group. The summary
    /// holds the report of every member in the order of the list. Its text
    /// gives the members that did not pass and counts them. With `--json`, the
    /// standard output therefore carries one document for the run, and the
    /// messages go to the standard error. With `--quiet`, the messages do not
    /// appear, and the summary still gives the members that did not pass.
    ///
    /// With `--verbose`, the run also shows what the tools of each member
    /// write. A member that writes shows its tools while they run. A member
    /// that reads holds what its tools write until it finishes. Then that
    /// output appears in one block under a header that gives the member, just
    /// before its report. So the output of two members never mixes. With
    /// `--json`, this output goes to the standard error as well.
    ///
    /// The run exits with the worst code of its members. A member that stopped
    /// gives the code of an action that stopped. Otherwise, a member that found
    /// problems gives the code of an action with findings. Otherwise, the run
    /// exits zero, also when a member changed the project or skipped. A member
    /// that cannot bind stops the run with an error before any member runs. So
    /// does a member that declares a kind of access that the command line does
    /// not know.
    ///
    /// The members of a group need no mount of their own. A harness that wants
    /// a command for one of them therefore mounts that action as well.
    ///
    /// # Panics
    ///
    /// Panics when the value carries the name of an action, a command, or a
    /// group that the harness mounted before, and reports that name. A list
    /// that carries one name twice panics in the same way. A run cannot choose
    /// between them, and only a change of the harness corrects it, so the
    /// failure happens here instead of at a run.
    ///
    /// Panics when the value carries the name `doctor` or `help`, and reports
    /// the action, the command, or the group. The command line keeps these
    /// names for commands of its own.
    ///
    /// Panics when an action, a command, or a member of a group declares an
    /// argument whose name the command line defines itself. The message gives
    /// the action, the command, or the group and its member, and the argument.
    ///
    /// Panics when a group lists two members with the same name, and reports
    /// the group and the name.
    ///
    /// Panics when two members of a group declare one argument with two
    /// different shapes, and reports the group, both members, the argument,
    /// and both shapes.
    ///
    /// # Examples
    ///
    /// A harness mounts an action as it is:
    ///
    /// ```
    /// # use rakko::{Action, Context, Documented, Name, Outcome, action_name};
    /// # /// Counts the lines of the project
    /// # #[derive(Documented)]
    /// # struct LineCount;
    /// # impl Action for LineCount {
    /// #     type Args = ();
    /// #     fn name(&self) -> Name { action_name!("line-count") }
    /// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    /// #         Outcome::Passed { summary: None }
    /// #     }
    /// # }
    /// let command_line = rakko_cli::builder().mount(LineCount);
    /// ```
    ///
    /// A bundle exports its actions as a [`Bundle`], and a harness mounts that
    /// list as it is. A second call adds to the first, so the harness mounts a
    /// bundle beside its own actions, commands, and groups. A group such as
    /// pre-commit takes its members as a list, which the harness builds from
    /// the actions as they are:
    ///
    /// ```
    /// # use rakko::{
    /// #     Action, Bundle, Context, Documented, ErasedAction, Group, Name, Outcome, action_name,
    /// # };
    /// # use rakko_cli::Command;
    /// # use rakko_cli::clawless::CommandResult;
    /// # use rakko_cli::clawless::context::Context as ClawlessContext;
    /// # mod aonyx_rakko_baseline {
    /// #     use rakko::{Action, Bundle, Context, Documented, Name, Outcome, action_name};
    /// #     /// Formats the JSON files of a project
    /// #     #[derive(Documented)]
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
    /// # /// Checks a project against the Rust version it promises
    /// # #[derive(Documented)]
    /// # struct CheckMsrv;
    /// # impl Action for CheckMsrv {
    /// #     type Args = ();
    /// #     fn name(&self) -> Name { action_name!("check-msrv") }
    /// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    /// #         Outcome::Passed { summary: None }
    /// #     }
    /// # }
    /// # /// Lints the Rust code of a project
    /// # #[derive(Documented)]
    /// # struct LintRust;
    /// # impl Action for LintRust {
    /// #     type Args = ();
    /// #     fn name(&self) -> Name { action_name!("lint-rust") }
    /// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    /// #         Outcome::Passed { summary: None }
    /// #     }
    /// # }
    /// # /// Serves the files of the project
    /// # #[derive(Documented)]
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
    /// # /// Runs the actions that guard a commit
    /// # #[derive(Documented)]
    /// # struct PreCommit(Bundle);
    /// # impl PreCommit {
    /// #     fn new(actions: impl IntoIterator<Item = Box<dyn ErasedAction>>) -> Self {
    /// #         Self(actions.into_iter().collect())
    /// #     }
    /// # }
    /// # impl Group for PreCommit {
    /// #     fn name(&self) -> Name { action_name!("pre-commit") }
    /// #     fn members(&self) -> &[Box<dyn ErasedAction>] { self.0.actions() }
    /// # }
    /// let pre_commit = PreCommit::new(Bundle::default().with(LintRust));
    ///
    /// let command_line = rakko_cli::builder()
    ///     .mount(aonyx_rakko_baseline::bundle())
    ///     .mount(CheckMsrv)
    ///     .mount(Serve)
    ///     .mount(pre_commit);
    /// ```
    ///
    /// [`Bundle`]: rakko::Bundle
    /// [`Command`]: crate::Command
    /// [`Group`]: rakko::Group
    #[must_use]
    pub fn mount<K: ?Sized>(mut self, value: impl Mount<K>) -> Self {
        self.add(value.entries().into_vec());

        self
    }

    /// Adds entries to the registry, after the checks that a mount runs
    ///
    /// Every mount ends here, so an action, a command, and a group pass the
    /// same checks and land in the same tree.
    fn add(&mut self, entries: impl IntoIterator<Item = Mounted>) {
        let entries: Vec<Mounted> = entries.into_iter().collect();

        self::arguments::refuse_reserved(&entries);

        self.registry.add(entries);
    }

    /// Runs the command line against the arguments of the process
    ///
    /// A request for help, and a request that the command line cannot read,
    /// are reported in this method. The user gets the message of the parser
    /// and this method returns an exit code. A run that names nothing ends here
    /// as well. The user gets the commands that the harness mounted, and this
    /// method returns the code of a clean run. A run that failed ends here too.
    /// The user gets the error and every cause of it. This method returns the
    /// code of a run that could not answer, whether the failure stopped an
    /// action, a command, or a group. A problem of the tool pins of the
    /// project is such a failure. The run of an action, a command, or a group
    /// then stops before it starts. The error gives each problem and how to
    /// repair it: `doctor --fix`, or a change to the pin of a toolchain. The
    /// help, the list of the commands, and `doctor` run without this check.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::process::ExitCode;
    ///
    /// // A harness that mounts nothing lists no commands and ends cleanly.
    /// assert_eq!(rakko_cli::builder().run(), ExitCode::SUCCESS);
    /// ```
    pub fn run(self) -> std::process::ExitCode {
        let (matches, mounted, declarations) = match self.resolve(std::env::args_os()) {
            Ok(Resolution::Entry(matches, mounted, declarations)) => {
                (matches, mounted, declarations)
            }
            Ok(Resolution::Commands(mut command)) => {
                return std::process::ExitCode::from(show_commands(&mut command));
            }
            Err(error) => {
                let _ = error.print();
                return process_exit_code(error.exit_code());
            }
            Ok(Resolution::Doctor(matches, declarations)) => {
                return match drive_doctor(matches, declarations) {
                    Ok(code) => std::process::ExitCode::from(code),
                    Err(error) => {
                        let printed_error =
                            run_error(&format!("command '{}'", doctor::DOCTOR), error.as_ref());
                        let _ = printed_error.print();
                        process_exit_code(printed_error.exit_code())
                    }
                };
            }
        };
        let entry = mounted.to_string();

        match dispatch(matches, mounted, declarations) {
            Ok(code) => std::process::ExitCode::from(code),
            Err(error) => {
                let printed_error = run_error(&entry, error.as_ref());
                let _ = printed_error.print();
                process_exit_code(printed_error.exit_code())
            }
        }
    }

    /// Parses the arguments and takes the action, the command, or the group
    /// that the run asks for
    ///
    /// A run that gives no argument names nothing. It gets the command
    /// line back instead of an error, so that the caller lists the commands
    /// that the harness mounted. Every other run gets the tools that every
    /// mounted action declares, the members of each group included. A run of
    /// `doctor` checks them. A run of an action, a command, or a group checks
    /// them before it starts.
    ///
    /// The method takes the arguments as a parameter, so that a test drives
    /// the command line without the arguments of the test process.
    ///
    /// The parser reports `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand`
    /// for a run that gives no argument at all, and for no other run. That kind
    /// is the whole test for a run that names nothing.
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
            Err(error) if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                return Ok(Resolution::Commands(Box::new(command)));
            }
            Err(error) => return Err(error),
        };

        if matches
            .subcommand_name()
            .is_some_and(|name| name == doctor::DOCTOR)
        {
            let declarations = self.declarations();

            return Ok(Resolution::Doctor(matches, declarations));
        }

        let declarations = self.declarations();
        let mounted = matches
            .subcommand()
            .and_then(|(name, _)| self.registry.take(name));

        match mounted {
            Some(mounted) => Ok(Resolution::Entry(matches, mounted, declarations)),
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
    /// take the same shape here. The shape has a name, the flags of the
    /// arguments, and a place directly under the command line. Each of them
    /// also shows the sentence that describes it.
    fn command(&self) -> Command {
        let mut command = shell();

        for mounted in self.registry.entries() {
            command = command.subcommand(
                Command::new(mounted.name().get().to_owned())
                    .about(mounted.description().get().to_owned())
                    .args(self::arguments::render(mounted)),
            );
        }

        command.subcommand(doctor::command())
    }

    /// Returns the tools that the mounted actions declare
    ///
    /// A member of a group runs without a mount of its own, so the members of
    /// each group count as well. An action that the harness mounts alone and
    /// as a member declares its tools once. A command that the harness wrote
    /// declares no tools, and the command line does not see the actions that
    /// a command runs. So an action that the harness gives only to a command
    /// declares nothing here. The harness also mounts it, so that `doctor` sees
    /// its tools.
    fn declarations(&self) -> BTreeSet<Declaration> {
        let mut declarations = BTreeSet::new();

        for mounted in self.registry.entries() {
            let actions: Vec<&dyn ErasedAction> = match mounted {
                Mounted::Action(action) => vec![action.as_ref()],
                Mounted::Group { group, .. } => group.members().iter().map(AsRef::as_ref).collect(),
                Mounted::Command(_) => Vec::new(),
            };

            for action in actions {
                let name = action.name();

                declarations.extend(action.tools().into_iter().map(|requirement| {
                    Declaration::builder()
                        .action(name.clone())
                        .requirement(requirement)
                        .build()
                }));
            }
        }

        declarations
    }
}

/// Returns the exit code of the process for the code that the parser chose
///
/// The parser names its codes as an `i32`, while a process ends with a byte.
/// A code outside that range reports a failure.
fn process_exit_code(code: i32) -> std::process::ExitCode {
    u8::try_from(code).map_or(
        std::process::ExitCode::FAILURE,
        std::process::ExitCode::from,
    )
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
    /// The action, the written command, or the group that the run asks for
    ///
    /// It holds the arguments that the parser read and the tools that the
    /// mounted actions declare.
    Entry(ArgMatches, Mounted, BTreeSet<Declaration>),
    /// The command line of a run that names nothing
    Commands(Box<Command>),
    /// A run of `doctor`, with the arguments that the parser read and the
    /// tools that the mounted actions declare
    Doctor(ArgMatches, BTreeSet<Declaration>),
}

/// Shows the commands that a harness mounted and returns the code of the run
///
/// The help of the command line lists the commands. So a user who wrote the
/// name of the harness and nothing else reads what the project runs. The run
/// drove no action and found no problem, so it is clean. A workflow that
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
/// Before the action, the command, or the group starts, the run checks the
/// precondition on the tools against the declarations, as `doctor` checks
/// it. A problem stops the run, also in CI, because a warning lets a broken
/// environment pass a pull request without a sign.
///
/// # Errors
///
/// Returns the error of the command line when it cannot build the context of
/// a command. It also returns that error when it cannot start the runtime
/// that drives the run. Returns an error for a run that names no root and
/// stands in no project. This is because a run that receives a guessed root
/// reports paths that mean nothing.
/// Returns the errors that [`CheckPreconditionError`] describes when the tool
/// pins of the project have a problem or the check gets no answer.
/// Returns an error for a group whose member cannot bind its values or
/// declares a kind of access that the command line does not know. Returns the
/// error of a command that failed, and the error of a report that cannot reach
/// the reader.
///
/// [`CheckPreconditionError`]: doctor::CheckPreconditionError
fn dispatch(
    matches: ArgMatches,
    mounted: Mounted,
    declarations: BTreeSet<Declaration>,
) -> Result<u8, Box<dyn Error>> {
    let named = matches.get_one::<PathBuf>(PROJECT_ROOT).cloned();

    match mounted {
        Mounted::Action(action) => {
            let values = values(&action.arguments(), &matches);

            drive_action(matches, named, declarations, values, action)
        }
        Mounted::Command(command) => {
            let values = values(&command.arguments(), &matches);

            drive_command(matches, named, declarations, values, command)
        }
        Mounted::Group { group, .. } => {
            let values = group
                .members()
                .iter()
                .map(|member| values(&member.arguments(), &matches))
                .collect();

            drive_group(matches, named, declarations, values, group)
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
            self::arguments::collect(schema, command)
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
/// What the action returned reaches the reader as one report. The flags
/// of the run decide whether that report renders as text or as JSON. The
/// report travels as the result of the command and not as a message. So the
/// flags that reduce the output do not suppress what the run found.
///
/// # Errors
///
/// Returns the errors that [`dispatch`] describes.
fn drive_action(
    matches: ArgMatches,
    named: Option<PathBuf>,
    declarations: BTreeSet<Declaration>,
    values: ArgsValues,
    action: Box<dyn ErasedAction>,
) -> Result<u8, Box<dyn Error>> {
    let lines = ToolLines::from_matches(&matches);

    drive(matches, move |context| async move {
        let project = locate(named, &context)?;
        doctor::require()
            .declarations(&declarations)
            .context(&project)
            .call()
            .await?;

        let name = action.name();
        let outcome = match action.bind(&values) {
            Ok(bound) => match bound.access(&project) {
                Access::Read | Access::Write => {
                    self::tool_lines::stream(
                        lines,
                        &project,
                        context.output(),
                        |project| async move { bound.run(&project).await },
                    )
                    .await?
                }
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
            .artifact(Report::builder().action(name).outcome(outcome).build())
            .await?;

        Ok(code)
    })
}

/// Runs one command that the harness wrote and returns the code of the run
///
/// The command receives the context of the project and the context of
/// Clawless, and it writes what it has to say through the latter. A run that
/// ends in success is clean. A run that ends in an error carries that error
/// to the caller. The caller shows it and gives the process the code of a run
/// that could not answer. This is because the command line cannot tell a
/// problem of the project from a failure of the command.
///
/// # Errors
///
/// Returns the errors that [`dispatch`] describes.
fn drive_command(
    matches: ArgMatches,
    named: Option<PathBuf>,
    declarations: BTreeSet<Declaration>,
    values: ArgsValues,
    command: Box<dyn ErasedCommand>,
) -> Result<u8, Box<dyn Error>> {
    drive(matches, move |context| async move {
        let project = locate(named, &context)?;
        doctor::require()
            .declarations(&declarations)
            .context(&project)
            .call()
            .await?;

        command.run(&project, &context, &values).await?;

        Ok(EXIT_CLEAN)
    })
}

/// Runs `doctor` and returns the code of the run
///
/// The run reports as a run of one action does, so its findings, its
/// repairs, and its exit code follow the rules of an action. A run that found
/// a problem that remains ends with the code of a run with findings, also
/// after it repaired others.
///
/// # Errors
///
/// Returns the errors that [`dispatch`] describes for a run that names no
/// root and stands in no project. It also returns them for a report that
/// cannot reach the reader.
fn drive_doctor(
    matches: ArgMatches,
    declarations: BTreeSet<Declaration>,
) -> Result<u8, Box<dyn Error>> {
    let named = matches.get_one::<PathBuf>(PROJECT_ROOT).cloned();
    let mode = matches
        .subcommand_matches(doctor::DOCTOR)
        .map_or(doctor::Mode::Report, doctor::mode);

    drive(matches, move |context| async move {
        let project = locate(named, &context)?;

        let outcome = doctor::run()
            .declarations(&declarations)
            .context(&project)
            .mode(mode)
            .call()
            .await;
        let code = exit_code(&outcome);

        context
            .output()
            .artifact(
                Report::builder()
                    .action(doctor::name())
                    .outcome(outcome)
                    .build(),
            )
            .await?;

        Ok(code)
    })
}

/// Runs the members of one group and returns the code of the run
///
/// The values hold one entry for each member of the group, in the order of
/// the members. Every member is bound to its values before any member runs,
/// so a member that cannot bind stops the run before any member runs. The
/// scheduler then runs the bound members and reports them.
///
/// The run answers as a run of one action does. The report of each member
/// travels as a message, so the flags that reduce the output suppress it. The
/// summary of the group holds the report of every member. It travels as the
/// result of the command, so these flags do not suppress it. The code is the
/// worst code of the members. A group whose member stopped ends with the code
/// of a run that could not answer. Otherwise, a group whose member found
/// problems ends with the code of a run with findings.
///
/// # Errors
///
/// Returns [`BindMembersError::UnboundMember`] for a member whose values do
/// not convert into its arguments. Returns the errors of the scheduler, which
/// [`RunGroupError`] describes. Returns the other errors that [`dispatch`]
/// describes.
///
/// [`BindMembersError::UnboundMember`]: self::members::BindMembersError::UnboundMember
/// [`RunGroupError`]: self::scheduler::RunGroupError
fn drive_group(
    matches: ArgMatches,
    named: Option<PathBuf>,
    declarations: BTreeSet<Declaration>,
    values: Vec<ArgsValues>,
    group: Box<dyn Group>,
) -> Result<u8, Box<dyn Error>> {
    let lines = ToolLines::from_matches(&matches);

    drive(matches, move |context| async move {
        let project = locate(named, &context)?;
        doctor::require()
            .declarations(&declarations)
            .context(&project)
            .call()
            .await?;

        let members = self::members::bind(group.members(), &values)?;
        let code = self::scheduler::run(&group.name(), members, &project, &context, lines).await?;

        Ok(code)
    })
}

/// Runs one entry through the runner of Clawless and returns what it returned
///
/// The runner renders everything that the entry sends before it returns, and
/// it ends every run with an exit of its own. That exit holds an error only
/// as text, and it holds a code that the process cannot read back. The entry
/// therefore gives its result back beside the runner. This lets the caller get
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
/// The message carries every cause of the error. This is because the error of
/// a command usually states only what failed, and the reason sits in its
/// source. The parser exits with the code of a run that could not answer for
/// every error of this kind. So the message and the code travel together.
fn run_error(entry: &str, error: &dyn Error) -> clap::Error {
    clap::Error::raw(
        ErrorKind::Io,
        format!("failed to run the {entry}: {}\n", Chain::new(error)),
    )
}

/// Returns the context of the project that a run maintains
///
/// A user who named the project root gets that root. Every other run
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
/// A run that does not apply is clean, because a skip is an answer. A bundle
/// that mounts an action for a stack that a project does not use must not turn
/// that project red. A run that repaired the project is clean as well. The
/// caller asked for the repair, and the repairs travel in the report. So a
/// step that runs a fix in CI must not go red over its success. A run that
/// repaired some of what it found still has a problem, so it takes the code
/// for a problem.
///
/// The codes grow with how bad the outcome is. A clean run gives the lowest
/// code, and a run that could not answer gives the highest. A run of a group
/// takes the highest code of its members, so the worst member decides it.
fn exit_code(outcome: &Outcome) -> u8 {
    match outcome {
        Outcome::Passed { .. } | Outcome::Changed { .. } | Outcome::Skipped { .. } => EXIT_CLEAN,
        Outcome::Failed { .. } => EXIT_FINDINGS,
        Outcome::Errored { .. } => EXIT_UNANSWERED,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::marker::PhantomData;
    use std::path::Path;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use clap::builder::{Str, ValueRange};
    use clawless::CommandResult;
    use rakko::Documented;
    use rakko::{
        Action, Args, Argument, ArgumentShape, ArgumentValue, Bundle, Name, OperatingSystem,
        Platform, ReadArgsError, RequiredVersion, SkipReason, ToolRequirement, ToolVersion,
        argument_name,
    };

    use super::*;
    use crate::command::Command as WrittenCommand;

    /// The name of the group that the tests mount
    const GROUP: &str = "check";

    /// The log into which the members of a group write the values that their
    /// runs received, in the order of their runs
    type Log = Arc<Mutex<Vec<(Name, ArgsValues)>>>;

    /// The documentation of the boolean argument that the reader reads
    const FIX: &str = "Rewrite the files that the tool can format";

    /// The command that repairs the tool pins, as the error of a stopped run
    /// gives it
    const REPAIR: &str = "mise run rakko -- doctor --fix";

    /// The name of the action that reads an argument of every shape
    const READER: &str = "reader";

    /// The arguments that the reader reads
    ///
    /// The set holds one argument of every shape. It keeps the values that
    /// it was built from so that a test reads what the command line
    /// collected.
    #[derive(Clone, Eq, PartialEq, Debug)]
    struct Arguments {
        /// The values that the run gave to the action
        values: ArgsValues,
    }

    impl Args for Arguments {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([
                argument()
                    .name("fix")
                    .shape(ArgumentShape::Boolean)
                    .documentation(FIX)
                    .call(),
                argument()
                    .name("jobs")
                    .shape(ArgumentShape::Integer)
                    .documentation("Examine this many files at once")
                    .call(),
                argument()
                    .name("report")
                    .shape(ArgumentShape::Path)
                    .documentation("Write the report to this file")
                    .call(),
                argument()
                    .name("extension")
                    .shape(ArgumentShape::Text)
                    .documentation("Examine only the files with this extension")
                    .call(),
            ])
        }

        fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Ok(Self {
                values: values.clone(),
            })
        }
    }

    /// A command that runs one action through its erased view, as a command
    /// that a harness writes runs an action, and keeps the outcome
    #[derive(Documented)]
    struct Calling {
        /// The action that the command runs
        action: Box<dyn ErasedAction>,
        /// The outcome of the run of the action, once the command ran
        outcome: Arc<Mutex<Option<Outcome>>>,
    }

    impl WrittenCommand for Calling {
        type Args = ();

        fn name(&self) -> Name {
            action_name("calling")
        }

        async fn run(
            &self,
            project: &Context,
            _clawless: &ClawlessContext,
            _args: &Self::Args,
        ) -> CommandResult {
            let outcome = self.action.run(project, &ArgsValues::empty()).await;

            let mut kept = self.outcome.lock().expect("the test holds the lock alone");
            *kept = Some(outcome);

            Ok(())
        }
    }

    /// An action that declares an argument that the command line carries
    #[derive(Documented)]
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
    /// The description of an argument set belongs to its type. So each set of
    /// arguments that a test member declares is a type of its own.
    trait Declaration: Send + Sync + 'static {
        /// Returns the arguments that the member declares
        fn schema() -> ArgsSchema;
    }

    /// The declaration of a member that takes a boolean `fix`
    struct Fix;

    impl Declaration for Fix {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([argument()
                .name("fix")
                .shape(ArgumentShape::Boolean)
                .documentation(FIX)
                .call()])
        }
    }

    /// The declaration of a member that takes `fix` as a text
    struct FixAsText;

    impl Declaration for FixAsText {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([argument()
                .name("fix")
                .shape(ArgumentShape::Text)
                .documentation("Repair the files that match this pattern")
                .call()])
        }
    }

    /// An action that declares one tool and otherwise runs as the probe that
    /// it wraps
    #[derive(Documented)]
    struct Gated {
        /// The probe that gives the action its name and runs it
        probe: Probe,
        /// The tool that the action declares
        tool: ToolRequirement,
    }

    impl Gated {
        /// Creates an action whose tool has a build only on the platform, and
        /// the flag that its run sets
        fn on(platform: Platform) -> (Self, Arc<AtomicBool>) {
            let (probe, ran) = Probe::new("lint-stiles");
            let gated = Self {
                probe,
                tool: ToolRequirement::builder()
                    .tool("github:aonyx-ai/stiles")
                    .executable("stiles")
                    .exact_version_on(ToolVersion::new("0.6.0"), [platform])
                    .build(),
            };

            (gated, ran)
        }
    }

    impl Action for Gated {
        type Args = ();

        fn name(&self) -> Name {
            Action::name(&self.probe)
        }

        fn tools(&self) -> Vec<ToolRequirement> {
            vec![self.tool.clone()]
        }

        async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
            Action::run(&self.probe, context, args).await
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
    #[derive(Documented)]
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
    #[derive(Documented)]
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
    #[derive(Documented)]
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
    #[derive(Documented)]
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
    #[derive(Documented)]
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
            ArgsSchema::new([argument()
                .name("report")
                .shape(ArgumentShape::Path)
                .documentation("Write the report to this file")
                .call()])
        }
    }

    /// The arguments of an action that declares a name of the command line
    struct Reserved;

    impl Args for Reserved {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([argument()
                .name("json")
                .shape(ArgumentShape::Boolean)
                .documentation("Report what the action found as JSON")
                .call()])
        }

        fn from_values(_values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Ok(Self)
        }
    }

    /// A command that declares an argument that the command line carries
    #[derive(Documented)]
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

    /// A group that the tests mount, with the members that a test gives it
    #[derive(Documented)]
    struct Sequence {
        /// The name of the group
        name: Name,
        /// The actions that the group lists
        actions: Vec<Box<dyn ErasedAction>>,
    }

    impl Group for Sequence {
        fn name(&self) -> Name {
            self.name.clone()
        }

        fn members(&self) -> &[Box<dyn ErasedAction>] {
            &self.actions
        }
    }

    /// An action that declares one tool at an exact version
    #[derive(Documented)]
    struct Tooled {
        /// The name that identifies this action
        name: Name,
        /// The tool that the action declares
        tool: ToolRequirement,
    }

    impl Action for Tooled {
        type Args = ();

        fn name(&self) -> Name {
            self.name.clone()
        }

        fn tools(&self) -> Vec<ToolRequirement> {
            vec![self.tool.clone()]
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// An action whose arguments never convert from the values of a run
    #[derive(Documented)]
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
    #[derive(Documented)]
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
    #[bon::builder]
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

        dispatch(
            naming(directory.path()),
            Mounted::Action(Box::new(probe)),
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the action")
    }

    /// Returns the declaration of a tool at a version by an action
    #[bon::builder]
    fn declaration(action: &str, tool: &str, version: &str) -> doctor::Declaration {
        doctor::Declaration::builder()
            .action(action_name(action))
            .requirement(exact().tool(tool).version(version).call())
            .build()
    }

    /// Returns the requirement of a tool at an exact version
    #[bon::builder]
    fn exact(tool: &str, version: &str) -> ToolRequirement {
        ToolRequirement::builder()
            .tool(tool)
            .executable(tool)
            .version(RequiredVersion::Exact(ToolVersion::new(version)))
            .build()
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
            .mount(group)
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

        let Ok(Resolution::Entry(matches, mounted, declarations)) =
            builder().mount(group).resolve(invocation)
        else {
            panic!("expected the command line to read the run that the test describes");
        };

        dispatch(matches, mounted, declarations)
    }

    /// Returns a group of the given members
    fn sequence(actions: Vec<Box<dyn ErasedAction>>) -> Sequence {
        Sequence {
            name: action_name(GROUP),
            actions,
        }
    }

    /// Returns one erased action that declares taplo at the version of the
    /// other tests, with a build only on the platform
    fn limited_to(platform: Platform) -> Box<dyn ErasedAction> {
        Box::new(Tooled {
            name: action_name("format-toml"),
            tool: ToolRequirement::builder()
                .tool("taplo")
                .executable("taplo")
                .exact_version_on(ToolVersion::new("0.10.0"), [platform])
                .build(),
        })
    }

    /// Returns the platform of the run of the test, on every architecture
    fn this_platform() -> Platform {
        let system = if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::Macos
        } else {
            OperatingSystem::Linux
        };

        Platform::new(system)
    }

    /// Returns a platform that is not the platform of the run of the test
    fn another_platform() -> Platform {
        let system = if cfg!(target_os = "windows") {
            OperatingSystem::Linux
        } else {
            OperatingSystem::Windows
        };

        Platform::new(system)
    }

    /// Returns the platform of the run of the test, as mise writes it
    fn this_host() -> String {
        let architecture = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x64"
        };

        format!("{}/{architecture}", this_platform())
    }

    /// Returns the code of a run of `doctor` with the arguments, in the
    /// project at the root, for the harness of the builder
    #[bon::builder]
    fn doctor_in(root: &Path, builder: Builder, arguments: &[&str]) -> u8 {
        let mut line: Vec<OsString> = vec!["rakko".into(), "--project-root".into(), root.into()];
        line.extend(arguments.iter().map(OsString::from));

        let Ok(Resolution::Doctor(matches, declarations)) = builder.resolve(line) else {
            panic!("expected the run to ask for doctor");
        };

        drive_doctor(matches, declarations).expect("expected doctor to run")
    }

    /// A project in a temporary directory that mise trusts
    ///
    /// Mise refuses a configuration file that nobody trusts when a pin in it
    /// has options, such as the platforms of a tool. So the project holds a
    /// `mise.toml` that mise trusts, which covers the generated file too.
    struct Trusted {
        /// The directory that holds the project
        directory: tempfile::TempDir,
    }

    impl Trusted {
        /// Creates the project and trusts its configuration
        fn new() -> Self {
            let directory = tempfile::tempdir().expect("the test creates a temporary directory");
            let pins = directory.path().join("mise.toml");
            std::fs::write(&pins, "[tools]\n").expect("the test writes the mise.toml");

            let trusted = std::process::Command::new("mise")
                .arg("trust")
                .arg("--quiet")
                .arg(&pins)
                .status()
                .expect("the test runs mise");
            assert!(trusted.success(), "expected mise to trust the project");

            Self { directory }
        }

        /// Returns the root of the project
        fn path(&self) -> &Path {
            self.directory.path()
        }
    }

    impl Drop for Trusted {
        /// Withdraws the trust that the project received
        ///
        /// A failure stays quiet, because the test already reported what it
        /// was about.
        fn drop(&mut self) {
            let _ = std::process::Command::new("mise")
                .arg("trust")
                .arg("--quiet")
                .arg("--untrust")
                .arg(self.directory.path().join("mise.toml"))
                .status();
        }
    }

    /// Writes the generated file of the tool pins with the text, in the
    /// project at the root
    fn write_generated(root: &Path, text: &str) {
        let generated = root.join(".config/mise/conf.d/rakko.toml");
        std::fs::create_dir_all(
            generated
                .parent()
                .expect("the generated file sits in a directory"),
        )
        .expect("the test creates the directory of the generated file");
        std::fs::write(&generated, text).expect("the test writes the generated file");
    }

    /// Returns one erased action that declares a tool at a version
    #[bon::builder]
    fn tooled(name: &str, tool: &str, version: &str) -> Box<dyn ErasedAction> {
        Box::new(Tooled {
            name: action_name(name),
            tool: exact().tool(tool).version(version).call(),
        })
    }

    /// Returns what a run of `doctor` in an empty project gives back, for
    /// a harness that mounts an action that declares taplo
    ///
    /// The run names its project root, so that the test drives `doctor`
    /// without a directory tree that marks a project. The project stays on
    /// disk until the caller drops it.
    fn run_doctor(arguments: &[&str]) -> (tempfile::TempDir, u8) {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let root = directory.path().as_os_str().to_owned();
        let builder = builder().mount([tooled()
            .name("format-toml")
            .tool("taplo")
            .version("0.10.0")
            .call()]);
        let mut line: Vec<OsString> = vec!["rakko".into(), "--project-root".into(), root];
        line.extend(arguments.iter().map(OsString::from));

        let Ok(Resolution::Doctor(matches, declarations)) = builder.resolve(line) else {
            panic!("expected the run to ask for doctor");
        };
        let code = drive_doctor(matches, declarations).expect("expected doctor to run");

        (directory, code)
    }

    /// Returns the message of the error that stops a run of the entry, in a
    /// project whose generated file of the tool pins is stale
    ///
    /// The harness also mounts an action that declares taplo, so that the
    /// generated file must pin it. The file pins another version. The run
    /// gives its project root, so that the test drives the entry without a
    /// directory tree that marks a project.
    fn stop_of(builder: Builder, entry: &str) -> String {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let generated = directory.path().join(".config/mise/conf.d/rakko.toml");
        std::fs::create_dir_all(
            generated
                .parent()
                .expect("the generated file sits in a directory"),
        )
        .expect("the test creates the directory of the generated file");
        std::fs::write(&generated, "[tools]\ntaplo = \"0.9.0\"\n")
            .expect("the test writes a stale generated file");

        let builder = builder.mount([tooled()
            .name("format-toml")
            .tool("taplo")
            .version("0.10.0")
            .call()]);
        let line: Vec<OsString> = vec![
            "rakko".into(),
            "--project-root".into(),
            directory.path().into(),
            entry.into(),
        ];
        let Ok(Resolution::Entry(matches, mounted, declarations)) = builder.resolve(line) else {
            panic!("expected the command line to read the run that the test names");
        };

        let Err(error) = dispatch(matches, mounted, declarations) else {
            panic!("expected the stale tool pins to stop the run");
        };

        run_error(entry, error.as_ref()).to_string()
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

        let Ok(Resolution::Entry(matches, action, declarations)) =
            builder().mount(reader).resolve(invocation)
        else {
            panic!("expected the command line to read the run that the test names");
        };

        dispatch(matches, action, declarations)
            .expect("expected the command line to drive the action");

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

        let Ok(Resolution::Entry(matches, mounted, declarations)) =
            builder().mount(written).resolve(invocation)
        else {
            panic!("expected the command line to read the run that the test names");
        };

        dispatch(matches, mounted, declarations)
            .expect("expected the command line to drive the command");

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
            BTreeSet::new(),
        )
    }

    /// Returns one command with the given name
    fn written(name: &str) -> Written {
        let (written, _witness) = Written::new(name);

        written
    }

    #[test]
    fn builder_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Builder>();
    }

    #[test]
    fn builder_returns_a_builder_of_a_command_line() {
        let builder = builder();

        let name = builder.command().get_name().to_owned();

        assert_eq!(name, NAME);
    }

    #[test]
    fn command_carries_the_doctor_in_every_harness() {
        let command = builder().command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, [doctor::DOCTOR]);
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
    fn command_gives_a_boolean_argument_no_value() {
        let flag = flag_of("fix");

        let values = flag.get_num_args();

        assert_eq!(values, Some(ValueRange::EMPTY));
    }

    /// Returns the sentence that the help shows for an entry of the command
    /// line
    fn about_of(command: &Command, name: &str) -> Option<String> {
        command
            .get_subcommands()
            .find(|leaf| leaf.get_name() == name)
            .expect("the test mounts the entry")
            .get_about()
            .map(ToString::to_string)
    }

    #[test]
    fn command_gives_a_command_the_description_of_its_doc_comment() {
        let command = builder().mount(written("serve")).command();

        let about = about_of(&command, "serve");

        assert_eq!(
            about.as_deref(),
            Some("A command that records its run and returns what a test asked for")
        );
    }

    #[test]
    fn command_gives_a_group_the_description_of_its_doc_comment() {
        let command = builder().mount(sequence(Vec::new())).command();

        let about = about_of(&command, GROUP);

        assert_eq!(
            about.as_deref(),
            Some("A group that the tests mount, with the members that a test gives it")
        );
    }

    #[test]
    fn command_gives_an_action_the_description_of_its_doc_comment() {
        let command = builder().mount([reader()]).command();

        let about = about_of(&command, READER);

        assert_eq!(
            about.as_deref(),
            Some("An action that records the values that its run received")
        );
    }

    #[test]
    fn command_gives_an_argument_a_long_flag() {
        let flag = flag_of("fix");

        let long = flag.get_long();

        assert_eq!(long, Some("fix"));
    }

    #[test]
    fn command_gives_an_argument_of_a_command_a_long_flag() {
        let mut command = builder().mount(written("serve")).command();
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
        let command = builder().mount(written("serve")).command();

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
    fn command_refuses_a_run_that_gives_an_argument_and_names_no_action() {
        let kind = error_kind(&["rakko", "--json"]);

        assert_eq!(kind, ErrorKind::MissingSubcommand);
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
    fn dispatch_asks_the_action_for_its_access() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (reading, _ran, asked) = Reading::new("probe");

        dispatch(
            naming(directory.path()),
            Mounted::Action(Box::new(reading)),
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the action");

        assert!(asked.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_gives_a_command_the_skip_of_an_action_whose_tool_has_no_build_here() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (gated, _ran) = Gated::on(another_platform());
        let outcome = Arc::new(Mutex::new(None));
        let calling = Calling {
            action: Box::new(gated),
            outcome: Arc::clone(&outcome),
        };

        dispatch(
            naming(directory.path()),
            Mounted::Command(Box::new(calling)),
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the command");

        let kept = outcome
            .lock()
            .expect("the test holds the lock alone")
            .take();
        let Some(Outcome::Skipped { reason }) = kept else {
            panic!("expected the action to skip, got {kept:?}");
        };
        assert_eq!(
            reason.get(),
            format!(
                "the tool github:aonyx-ai/stiles has no build for {}",
                this_host()
            )
        );
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
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the action");

        let root = seen.lock().expect("the test holds the lock alone").clone();
        let named = rakko::path::canonical(directory.path())
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
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the command");

        let root = witness
            .root
            .lock()
            .expect("the test holds the lock alone")
            .clone();
        let named = rakko::path::canonical(directory.path())
            .expect("the test names a directory that exists");
        assert_eq!(root, Some(named));
    }

    #[test]
    fn dispatch_reports_a_group_whose_member_found_problems_as_findings() {
        let (failing, _ran) = Probe::reporting("second", || Outcome::Failed {
            findings: Vec::new(),
            repairs: Vec::new(),
        });
        let group = sequence(vec![probe("first"), Box::new(failing)]);

        let code = run_group(group, &[]).expect("expected the command line to drive the group");

        assert_eq!(code, EXIT_FINDINGS);
    }

    #[test]
    fn dispatch_reports_a_group_whose_member_stopped_as_unanswered() {
        let (stopping, _ran) = Probe::reporting("second", || Outcome::Errored {
            source: Box::new(std::io::Error::other("boom")),
        });
        let group = sequence(vec![probe("first"), Box::new(stopping)]);

        let code = run_group(group, &[]).expect("expected the command line to drive the group");

        assert_eq!(code, EXIT_UNANSWERED);
    }

    #[test]
    fn dispatch_reports_a_group_whose_project_root_cannot_be_read() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let group = sequence(Vec::new());
        let mounted = Mounted::Group {
            group: Box::new(group),
            arguments: Vec::new(),
        };

        let result = dispatch(
            naming(&directory.path().join("missing")),
            mounted,
            BTreeSet::new(),
        );

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
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the action");

        assert_eq!(code, EXIT_UNANSWERED);
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
    fn dispatch_reports_every_cause_of_a_project_root_that_cannot_be_read() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let missing = directory.path().join("missing");
        let cause = rakko::path::canonical(&missing)
            .expect_err("the test names a directory that does not exist");
        let (probe, _ran) = Probe::new("probe");
        let Err(error) = dispatch(
            naming(&missing),
            Mounted::Action(Box::new(probe)),
            BTreeSet::new(),
        ) else {
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
        let group = sequence(vec![member::<Fix>("first", &log), Box::new(Unbuildable)]);

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
        let group = sequence(vec![member::<Fix>("first", &log)]);

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
    fn dispatch_runs_an_action_beside_a_tool_on_another_platform() {
        let directory = Trusted::new();
        let limited = || builder().mount([limited_to(another_platform())]);
        let (probe, ran) = Probe::new("probe");
        let line: Vec<OsString> = vec![
            "rakko".into(),
            "--project-root".into(),
            directory.path().into(),
            "probe".into(),
        ];
        let Ok(Resolution::Entry(matches, mounted, declarations)) =
            limited().mount(probe).resolve(line)
        else {
            panic!("expected the command line to read the run that the test names");
        };
        doctor::write_pins()
            .declarations(&declarations)
            .root(directory.path())
            .call()
            .expect("the test writes the generated file");
        let checked = doctor_in()
            .root(directory.path())
            .builder(limited())
            .arguments(&["doctor"])
            .call();
        let generated =
            std::fs::read_to_string(directory.path().join(".config/mise/conf.d/rakko.toml"))
                .expect("the test reads the generated file");

        dispatch(matches, mounted, declarations)
            .expect("expected the command line to drive the action");

        assert_eq!(
            (
                checked,
                generated.contains(&format!("os = [\"{}\"]", another_platform())),
                ran.load(Ordering::SeqCst)
            ),
            (0, true, true),
            "got {generated}"
        );
    }

    #[test]
    fn dispatch_runs_an_action_that_declares_a_read() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (reading, ran, _asked) = Reading::new("probe");

        dispatch(
            naming(directory.path()),
            Mounted::Action(Box::new(reading)),
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the action");

        assert!(ran.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_runs_an_action_whose_tool_has_a_build_here() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (gated, ran) = Gated::on(this_platform());

        let code = dispatch(
            naming(directory.path()),
            Mounted::Action(Box::new(gated)),
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the action");

        assert_eq!((code, ran.load(Ordering::SeqCst)), (0, true));
    }

    #[test]
    fn dispatch_runs_an_action_whose_tool_pins_are_current() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (probe, ran) = Probe::new("probe");
        let builder = builder().mount(probe).mount([tooled()
            .name("format-toml")
            .tool("taplo")
            .version("0.10.0")
            .call()]);
        let line: Vec<OsString> = vec![
            "rakko".into(),
            "--project-root".into(),
            directory.path().into(),
            "probe".into(),
        ];
        let Ok(Resolution::Entry(matches, mounted, declarations)) = builder.resolve(line) else {
            panic!("expected the command line to read the run that the test names");
        };
        doctor::write_pins()
            .declarations(&declarations)
            .root(directory.path())
            .call()
            .expect("the test writes the generated file");

        dispatch(matches, mounted, declarations)
            .expect("expected the command line to drive the action");

        assert!(ran.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_runs_no_member_of_a_group_whose_member_cannot_bind() {
        let log = Log::default();
        let group = sequence(vec![member::<Fix>("first", &log), Box::new(Unbuildable)]);

        let _result = run_group(group, &["--fix"]);

        let runs = log.lock().expect("the test holds the lock alone").len();
        assert_eq!(runs, 0);
    }

    #[test]
    fn dispatch_runs_the_action_that_it_gets() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let (probe, ran) = Probe::new("probe");

        dispatch(
            naming(directory.path()),
            Mounted::Action(Box::new(probe)),
            BTreeSet::new(),
        )
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
            BTreeSet::new(),
        )
        .expect("expected the command line to drive the command");

        assert!(witness.ran.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_skips_an_action_whose_tool_has_no_build_here() {
        let directory = Trusted::new();
        let (gated, ran) = Gated::on(another_platform());
        let line: Vec<OsString> = vec![
            "rakko".into(),
            "--project-root".into(),
            directory.path().into(),
            "lint-stiles".into(),
        ];
        let Ok(Resolution::Entry(matches, mounted, declarations)) =
            builder().mount(gated).resolve(line)
        else {
            panic!("expected the command line to read the run that the test names");
        };
        doctor::write_pins()
            .declarations(&declarations)
            .root(directory.path())
            .call()
            .expect("the test writes the generated file");

        let code = dispatch(matches, mounted, declarations)
            .expect("expected the command line to drive the action");

        assert_eq!((code, ran.load(Ordering::SeqCst)), (0, false));
    }

    #[test]
    fn dispatch_stops_a_command_whose_tool_pins_are_stale() {
        let (written, witness) = Written::new("serve");

        let message = stop_of(builder().mount(written), "serve");

        assert_eq!(
            (witness.ran.load(Ordering::SeqCst), message.contains(REPAIR)),
            (false, true),
            "got {message}"
        );
    }

    #[test]
    fn dispatch_stops_a_group_whose_tool_pins_are_stale() {
        let (probe, ran) = Probe::new("probe");

        let message = stop_of(builder().mount(sequence(vec![Box::new(probe)])), GROUP);

        assert_eq!(
            (ran.load(Ordering::SeqCst), message.contains(REPAIR)),
            (false, true),
            "got {message}"
        );
    }

    #[test]
    fn dispatch_stops_an_action_whose_tool_pins_are_stale() {
        let (probe, ran) = Probe::new("probe");

        let message = stop_of(builder().mount(probe), "probe");

        assert_eq!(
            (ran.load(Ordering::SeqCst), message.contains(REPAIR)),
            (false, true),
            "got {message}"
        );
    }

    #[test]
    fn drive_doctor_reports_a_problem_as_findings() {
        let (_project, code) = run_doctor(&["doctor"]);

        assert_eq!(code, EXIT_FINDINGS);
    }

    #[test]
    fn drive_doctor_with_a_tool_on_another_platform_reports_a_stale_pin() {
        let directory = Trusted::new();
        write_generated(
            directory.path(),
            &format!(
                "[tools]\ntaplo = {{ version = \"0.9.0\", os = [\"{}\"] }}\n",
                another_platform()
            ),
        );

        let code = doctor_in()
            .root(directory.path())
            .builder(builder().mount([limited_to(another_platform())]))
            .arguments(&["doctor"])
            .call();

        assert_eq!(code, EXIT_FINDINGS);
    }

    #[test]
    fn drive_doctor_with_a_tool_on_the_platform_of_the_run_reports_a_stale_pin() {
        let directory = Trusted::new();
        write_generated(
            directory.path(),
            &format!(
                "[tools]\ntaplo = {{ version = \"0.9.0\", os = [\"{}\"] }}\n",
                this_platform()
            ),
        );

        let code = doctor_in()
            .root(directory.path())
            .builder(builder().mount([limited_to(this_platform())]))
            .arguments(&["doctor"])
            .call();

        assert_eq!(code, EXIT_FINDINGS);
    }

    #[test]
    fn drive_doctor_with_the_flag_repairs_the_project() {
        let (project, _code) = run_doctor(&["doctor", "--fix"]);

        assert!(
            project
                .path()
                .join(".config/mise/conf.d/rakko.toml")
                .is_file()
        );
    }

    #[test]
    fn mount_accepts_a_member_of_a_group_that_the_harness_also_mounts() {
        let log = Log::default();
        let group = sequence(vec![member::<Fix>("first", &log)]);
        let command = builder()
            .mount([member::<Fix>("first", &log)])
            .mount(group)
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, [GROUP, "first", doctor::DOCTOR]);
    }

    #[test]
    fn mount_accepts_two_groups_that_share_a_member() {
        let log = Log::default();
        let check = sequence(vec![member::<Fix>("first", &log)]);
        let lint = sequence(vec![member::<Fix>("first", &log)]);
        let lint = Sequence {
            name: action_name("lint"),
            ..lint
        };
        let command = builder().mount(check).mount(lint).command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, [GROUP, "lint", doctor::DOCTOR]);
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
    fn mount_builds_a_flat_command_tree_of_commands() {
        let command = builder()
            .mount(written("serve"))
            .mount(written("watch"))
            .command();

        let nested = command
            .get_subcommands()
            .any(|leaf| leaf.get_subcommands().next().is_some());

        assert!(!nested);
    }

    #[test]
    fn mount_gives_a_group_one_flag_for_an_argument_that_members_share() {
        let log = Log::default();
        let group = sequence(vec![
            member::<Fix>("first", &log),
            member::<Fix>("second", &log),
            member::<Nothing>("third", &log),
        ]);

        let flags = flags_of_group(group);

        let longs: Vec<&str> = flags.iter().filter_map(Arg::get_long).collect();
        assert_eq!(longs, ["fix"]);
    }

    #[test]
    fn mount_gives_an_action_that_it_takes_as_it_is_a_command() {
        let (probe, _ran) = Probe::new("lint-rust");

        let command = builder().mount(probe).command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();
        assert_eq!(names, ["lint-rust", doctor::DOCTOR]);
    }

    #[test]
    fn mount_gives_each_action_a_command() {
        let command = builder()
            .mount([probe("format-toml"), probe("lint-rust")])
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, ["format-toml", "lint-rust", doctor::DOCTOR]);
    }

    #[test]
    fn mount_gives_each_action_of_a_bundle_a_command() {
        let (format, _ran) = Probe::new("format-toml");
        let (lint, _ran) = Probe::new("lint-rust");
        let bundle = Bundle::default().with(format).with(lint);

        let command = builder().mount(bundle).command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();
        assert_eq!(names, ["format-toml", "lint-rust", doctor::DOCTOR]);
    }

    #[test]
    fn mount_gives_each_command_a_place_beside_the_actions() {
        let command = builder()
            .mount([probe("lint-rust")])
            .mount(written("format-all"))
            .mount(written("serve"))
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, ["format-all", "lint-rust", "serve", doctor::DOCTOR]);
    }

    #[test]
    fn mount_gives_each_group_a_place_beside_the_actions_and_the_commands() {
        let group = sequence(Vec::new());
        let command = builder()
            .mount([probe("lint-rust")])
            .mount(written("serve"))
            .mount(group)
            .command();

        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();

        assert_eq!(names, [GROUP, "lint-rust", "serve", doctor::DOCTOR]);
    }

    #[test]
    fn mount_orders_the_flags_of_a_group_as_the_members_first_declare_them() {
        let log = Log::default();
        let group = sequence(vec![member::<ReportPath>("first", &log), reader()]);

        let flags = flags_of_group(group);

        let names: Vec<&str> = flags.iter().map(|flag| flag.get_id().as_str()).collect();
        assert_eq!(names, ["report", "fix", "jobs", "extension"]);
    }

    #[test]
    #[should_panic(
        expected = "the harness mounts the command 'help', whose name the command line \
                    reserves for itself"
    )]
    fn mount_refuses_a_command_named_help() {
        let _builder = builder().mount(written("help"));
    }

    #[test]
    #[should_panic(expected = "json")]
    fn mount_refuses_a_command_whose_argument_the_command_line_carries() {
        let _builder = builder().mount(Reserving);
    }

    #[test]
    #[should_panic(expected = "format-toml")]
    fn mount_refuses_a_command_with_the_name_of_an_action() {
        let _builder = builder()
            .mount([probe("format-toml")])
            .mount(written("format-toml"));
    }

    #[test]
    #[should_panic(
        expected = "the harness mounts the group 'doctor', whose name the command line \
                    reserves for itself"
    )]
    fn mount_refuses_a_group_named_doctor() {
        let group = Sequence {
            name: action_name("doctor"),
            actions: Vec::new(),
        };

        let _builder = builder().mount(group);
    }

    #[test]
    #[should_panic(expected = "the group 'check' lists the action 'first' twice")]
    fn mount_refuses_a_group_that_lists_an_action_twice() {
        let log = Log::default();
        let group = sequence(vec![
            member::<Fix>("first", &log),
            member::<Nothing>("second", &log),
            member::<Fix>("first", &log),
        ]);

        let _builder = builder().mount(group);
    }

    #[test]
    #[should_panic(expected = "the group 'check' lists the action 'first' twice")]
    fn mount_refuses_a_group_that_lists_two_actions_with_one_name() {
        let log = Log::default();
        let group = sequence(vec![
            member::<Fix>("first", &log),
            member::<FixAsText>("first", &log),
        ]);

        let _builder = builder().mount(group);
    }

    #[test]
    #[should_panic(
        expected = "the harness mounts two actions, commands, or groups with the name 'check'"
    )]
    fn mount_refuses_a_group_with_the_name_of_an_action() {
        let group = sequence(Vec::new());

        let _builder = builder().mount([probe(GROUP)]).mount(group);
    }

    #[test]
    #[should_panic(
        expected = "the action 'colliding' in the group 'check' declares the argument 'json', \
                    which the command line defines itself"
    )]
    fn mount_refuses_a_member_whose_argument_the_command_line_carries() {
        let group = sequence(vec![Box::new(Colliding)]);

        let _builder = builder().mount(group);
    }

    #[test]
    #[should_panic(
        expected = "the harness mounts the action 'doctor', whose name the command line \
                    reserves for itself"
    )]
    fn mount_refuses_an_action_named_doctor() {
        let _builder = builder().mount([probe("doctor")]);
    }

    #[test]
    #[should_panic(expected = "json")]
    fn mount_refuses_an_action_whose_argument_the_command_line_carries() {
        let _builder = builder().mount(Colliding);
    }

    #[test]
    #[should_panic(
        expected = "the group 'check' cannot merge the argument 'fix', because the action 'first' \
                    declares it as a boolean and the action 'second' declares it as a text"
    )]
    fn mount_refuses_members_that_declare_one_argument_with_two_shapes() {
        let log = Log::default();
        let group = sequence(vec![
            member::<Fix>("first", &log),
            member::<FixAsText>("second", &log),
        ]);

        let _builder = builder().mount(group);
    }

    #[test]
    #[should_panic(expected = "format-toml")]
    fn mount_refuses_two_actions_with_one_name() {
        let _builder = builder().mount([probe("format-toml"), probe("format-toml")]);
    }

    #[test]
    #[should_panic(expected = "serve")]
    fn mount_refuses_two_commands_with_one_name() {
        let _builder = builder().mount(written("serve")).mount(written("serve"));
    }

    #[test]
    fn mount_shows_each_line_of_a_flag_of_a_group_under_the_help_column() {
        let log = Log::default();
        let group = sequence(vec![
            member::<Fix>("first", &log),
            member::<Fix>("second", &log),
        ]);
        let mut command = builder().mount(group).command();
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
    fn mount_shows_one_line_for_each_member_that_declares_an_argument() {
        let log = Log::default();
        let group = sequence(vec![
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
    fn resolution_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Resolution>();
    }

    #[test]
    fn resolve_gives_the_doctor_each_tool_of_the_actions_and_the_members_once() {
        let group = sequence(vec![
            tooled()
                .name("format-toml")
                .tool("taplo")
                .version("0.10.0")
                .call(),
            tooled()
                .name("lint-json")
                .tool("jq")
                .version("1.8.2")
                .call(),
        ]);
        let builder = builder()
            .mount([
                tooled()
                    .name("format-toml")
                    .tool("taplo")
                    .version("0.10.0")
                    .call(),
                tooled()
                    .name("lint-yaml")
                    .tool("yamllint")
                    .version("1.38.0")
                    .call(),
            ])
            .mount(group);

        let Ok(Resolution::Doctor(_, declarations)) = builder.resolve(["rakko", doctor::DOCTOR])
        else {
            panic!("expected the run to ask for doctor");
        };

        assert_eq!(
            declarations,
            BTreeSet::from([
                declaration()
                    .action("format-toml")
                    .tool("taplo")
                    .version("0.10.0")
                    .call(),
                declaration()
                    .action("lint-json")
                    .tool("jq")
                    .version("1.8.2")
                    .call(),
                declaration()
                    .action("lint-yaml")
                    .tool("yamllint")
                    .version("1.38.0")
                    .call(),
            ])
        );
    }

    #[test]
    fn resolve_gives_the_entry_the_tools_of_every_mounted_action() {
        let builder = builder().mount([
            tooled()
                .name("format-toml")
                .tool("taplo")
                .version("0.10.0")
                .call(),
            tooled()
                .name("lint-yaml")
                .tool("yamllint")
                .version("1.38.0")
                .call(),
        ]);

        let Ok(Resolution::Entry(_, _, declarations)) = builder.resolve(["rakko", "lint-yaml"])
        else {
            panic!("expected the run to resolve an action");
        };

        assert_eq!(
            declarations,
            BTreeSet::from([
                declaration()
                    .action("format-toml")
                    .tool("taplo")
                    .version("0.10.0")
                    .call(),
                declaration()
                    .action("lint-yaml")
                    .tool("yamllint")
                    .version("1.38.0")
                    .call(),
            ])
        );
    }

    #[test]
    fn resolve_reports_a_run_that_it_cannot_read() {
        let kind = error_kind(&["rakko", "--unknown"]);

        assert_eq!(kind, ErrorKind::UnknownArgument);
    }

    #[test]
    fn resolve_returns_the_action_that_the_run_names() {
        let builder = builder().mount([probe("format-toml"), probe("lint-rust")]);

        let Ok(Resolution::Entry(_matches, Mounted::Action(action), _)) =
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
            .mount(written("serve"));

        let Ok(Resolution::Entry(_matches, Mounted::Command(command), _)) =
            builder.resolve(["rakko", "serve"])
        else {
            panic!("expected the run to resolve a command");
        };

        assert_eq!(command.name().get(), "serve");
    }

    #[test]
    fn resolve_returns_the_doctor_for_a_run_that_names_it() {
        let resolution = builder().resolve(["rakko", doctor::DOCTOR]);

        assert!(matches!(resolution, Ok(Resolution::Doctor(..))));
    }

    #[test]
    fn resolve_returns_the_group_that_the_run_names() {
        let group = sequence(Vec::new());
        let builder = builder().mount([probe("lint-rust")]).mount(group);

        let Ok(Resolution::Entry(_matches, Mounted::Group { group, .. }, _)) =
            builder.resolve(["rakko", GROUP])
        else {
            panic!("expected the run to resolve a group");
        };

        assert_eq!(group.name().get(), GROUP);
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
    fn resolve_shows_the_help_of_an_action_that_declares_a_tool() {
        let builder = builder().mount([tooled()
            .name("format-toml")
            .tool("taplo")
            .version("0.10.0")
            .call()]);

        let Err(error) = builder.resolve(["rakko", "format-toml", "--help"]) else {
            panic!("expected the run to ask for the help");
        };

        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
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
        let group = sequence(vec![
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
    fn run_gives_the_action_true_for_a_boolean_that_the_user_gave() {
        let values = values_of(&[READER, "--fix"]);

        let value = values.get(&argument_name!("fix"));

        assert_eq!(value, Some(&ArgumentValue::new("true")));
    }

    #[test]
    fn run_gives_the_command_the_text_that_the_user_wrote() {
        let values = values_of_command(&["serve", "--extension", "md"]);

        let value = values.get(&argument_name!("extension"));

        assert_eq!(value, Some(&ArgumentValue::new("md")));
    }

    #[test]
    fn show_commands_reports_a_clean_run() {
        let mut command = builder().command();

        let code = show_commands(&mut command);

        assert_eq!(code, EXIT_CLEAN);
    }
}
