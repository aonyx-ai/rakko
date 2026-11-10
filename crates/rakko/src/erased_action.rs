//! The view of an action that hides its type

use std::future::Future;
use std::pin::Pin;

use crate::access::Access;
use crate::action::Action;
use crate::args::{Args, ArgsSchema, ArgsValues, ReadArgsError};
use crate::bound_action::BoundAction;
use crate::context::Context;
use crate::name::Name;
use crate::outcome::{Outcome, SkipReason};
use crate::seal::{Sealed, SealedBound};
use crate::tool::{Host, ToolRequirement};

/// One action behind an interface that hides its type
///
/// The [`Action`] trait is written for the author of an action, so it carries
/// the type of the arguments that the action reads. A registry holds many
/// actions at once and cannot name a different type for each of them, so it
/// holds erased actions instead.
///
/// An erased action answers everything that the machinery needs without
/// naming the type of the action. It gives the name that identifies it and
/// the external tools that it runs. It gives the description of the arguments
/// that a projection turns into a command. It also gives a bind that takes the
/// values of that command. The conversion from those values into the
/// arguments of the action happens inside, where the type is still known.
/// The bind returns a [`BoundAction`], which declares how the run uses the
/// project and then runs with the same arguments.
///
/// Every action has an erased view, because this trait is implemented for all
/// of them. An action author never writes this implementation and never
/// writes this trait. A harness does not write it either. The harness gives
/// each action as it is to the builder of the command line, or to the list of
/// a [`Bundle`]. These erase the action themselves. Only an author of a group writes the
/// trait, in the members of the group.
///
/// No crate outside this one can implement the trait. So a later release can
/// add a method to it, such as one that reads a new declaration of the
/// action, and no implementation breaks.
///
/// [`Bundle`]: crate::Bundle
///
/// # Examples
///
/// ```
/// use rakko::{Action, Context, ErasedAction, Name, Outcome, action_name};
///
/// struct LineCount;
///
/// impl Action for LineCount {
///     type Args = ();
///
///     fn name(&self) -> Name {
///         action_name!("line-count")
///     }
///
///     async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
///         Outcome::Passed { summary: None }
///     }
/// }
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LineCount)];
///
/// assert_eq!(actions[0].name().get(), "line-count");
/// ```
///
/// An implementation outside this crate does not compile:
///
/// ```compile_fail
/// use rakko::{ArgsSchema, ArgsValues, BoundAction, ErasedAction, Name, ReadArgsError, action_name};
///
/// struct LineCount;
///
/// impl ErasedAction for LineCount {
///     fn name(&self) -> Name {
///         action_name!("line-count")
///     }
///
///     fn arguments(&self) -> ArgsSchema {
///         ArgsSchema::empty()
///     }
///
///     fn bind<'a>(
///         &'a self,
///         _values: &ArgsValues,
///     ) -> Result<Box<dyn BoundAction + 'a>, ReadArgsError> {
///         Err(ReadArgsError::MissingValue {
///             name: rakko::argument_name!("fix"),
///         })
///     }
/// }
/// ```
pub trait ErasedAction: Send + Sync + Sealed {
    /// Returns the name that identifies the action
    fn name(&self) -> Name;

    /// Returns the description of the arguments that the action reads
    ///
    /// A projection reads the description to build a command before a run
    /// exists.
    fn arguments(&self) -> ArgsSchema;

    /// Returns the external tools that the action runs
    ///
    /// The answer is the one that [`Action::tools`] gives. A caller reads it
    /// before any run, for example to list the tools that the mounted
    /// actions need.
    fn tools(&self) -> Vec<ToolRequirement>;

    /// Converts the values of a run into the arguments of the action
    ///
    /// The values hold what the machinery parsed for the arguments of the
    /// action. The answer holds the action together with its arguments. So a
    /// caller can ask it how the run uses the project and then run it, and both
    /// see the same arguments.
    ///
    /// # Errors
    ///
    /// Returns a [`ReadArgsError`] when the values do not convert into the
    /// arguments of the action. A caller that reports runs reports this
    /// error as the outcome of the run, in [`Outcome::Errored`], as
    /// [`run`][ErasedAction::run] does.
    fn bind<'a>(&'a self, values: &ArgsValues) -> Result<Box<dyn BoundAction + 'a>, ReadArgsError>;

    /// Binds the values of a run and runs the action with them
    ///
    /// This is the shorthand for a caller that does not ask how the run uses
    /// the project. It runs the action whatever kind of access the run
    /// declares. A caller that orders runs, or that refuses a kind of access,
    /// binds the action and asks the bound action instead. A conversion of the
    /// values that fails travels in [`Outcome::Errored`], so this method has
    /// no error type of its own.
    ///
    /// A run where a tool of the action has no build skips, as the run of
    /// [`BoundAction`] does.
    fn run<'a>(
        &'a self,
        context: &'a Context,
        values: &'a ArgsValues,
    ) -> Pin<Box<dyn Future<Output = Outcome> + Send + 'a>> {
        Box::pin(async move {
            match self.bind(values) {
                Ok(bound) => bound.run(context).await,
                Err(error) => Outcome::Errored {
                    source: Box::new(error),
                },
            }
        })
    }
}

impl<A: Action> Sealed for A {}

impl<A: Action> ErasedAction for A {
    fn name(&self) -> Name {
        Action::name(self)
    }

    fn arguments(&self) -> ArgsSchema {
        <A::Args as Args>::schema()
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        Action::tools(self)
    }

    fn bind<'a>(&'a self, values: &ArgsValues) -> Result<Box<dyn BoundAction + 'a>, ReadArgsError> {
        let args = <A::Args as Args>::from_values(values)?;

        Ok(Box::new(Bound { action: self, args }))
    }
}

/// An action and the arguments of one run, as the bind of [`ErasedAction`]
/// returns them
struct Bound<'a, A: Action> {
    /// The action that runs
    action: &'a A,
    /// The arguments that the declaration and the run both see
    args: A::Args,
}

impl<A: Action> SealedBound for Bound<'_, A> {}

impl<A: Action> BoundAction for Bound<'_, A> {
    fn access(&self, context: &Context) -> Access {
        Action::access(self.action, context, &self.args)
    }

    fn run<'a>(
        &'a self,
        context: &'a Context,
    ) -> Pin<Box<dyn Future<Output = Outcome> + Send + 'a>> {
        if let Some(reason) = missing_build(&Action::tools(self.action), Host::current()) {
            return Box::pin(async move { Outcome::Skipped { reason } });
        }

        Box::pin(Action::run(self.action, context, &self.args))
    }
}

/// Returns the reason to skip a run on the host, or `None` when every tool
/// has a build there
///
/// A tool without a list of platforms has a build on every platform. A tool
/// with a list has a build only on the platforms of the list. The reason gives
/// the first tool without a build on the host, and the host.
fn missing_build(tools: &[ToolRequirement], host: Host) -> Option<SkipReason> {
    tools
        .iter()
        .find(|tool| {
            tool.platforms()
                .is_some_and(|platforms| !platforms.iter().any(|platform| host.is_on(platform)))
        })
        .map(|tool| SkipReason::new(format!("the tool {} has no build for {host}", tool.tool())))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::pin::pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::{Context as TaskContext, Poll, Waker};

    use super::*;
    use crate::args::{Argument, ArgumentShape, ArgumentValue};
    use crate::tool::{OperatingSystem, Platform, RequiredVersion, ToolVersion};
    use crate::{action_name, argument_name};

    /// What a formatter does with the files that it examines
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
    enum Operation {
        /// Rewrite the files that the formatter can format
        Rewrite,

        /// Report the files that a rewrite would change, and rewrite nothing
        #[default]
        Report,
    }

    /// The arguments of the formatter: whether the run repairs what it finds
    struct Fix {
        /// What the run does with the files that it examines
        fix: Operation,
    }

    impl Args for Fix {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([Argument::builder()
                .name(argument_name!("fix"))
                .shape(ArgumentShape::Boolean)
                .documentation("Rewrite the files that the tool can format")
                .build()])
        }

        fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
            let fix = if values.get(&argument_name!("fix")).is_some() {
                Operation::Rewrite
            } else {
                Operation::Report
            };
            Ok(Self { fix })
        }
    }

    /// An action that writes only when its arguments ask for a repair
    struct Formatter;

    impl Action for Formatter {
        type Args = Fix;

        fn name(&self) -> Name {
            action_name!("formatter")
        }

        fn access(&self, _context: &Context, args: &Self::Args) -> Access {
            match args.fix {
                Operation::Rewrite => Access::Write,
                Operation::Report => Access::Read,
            }
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// An action that declares the one tool that it runs
    struct Wrapper;

    impl Action for Wrapper {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("wrapper")
        }

        fn tools(&self) -> Vec<ToolRequirement> {
            vec![taplo()]
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// Returns the requirement that the wrapper declares
    fn taplo() -> ToolRequirement {
        ToolRequirement::builder()
            .tool("taplo")
            .executable("taplo")
            .version(RequiredVersion::Exact(ToolVersion::new("0.10.0")))
            .build()
    }

    /// An action that declares taplo and a tool with a build only on some
    /// platforms, and records whether it ran
    struct Limited {
        /// The tool with a build only on some platforms
        tool: ToolRequirement,
        /// Whether a run of the action started
        ran: AtomicBool,
    }

    impl Limited {
        /// Creates an action whose second tool has a build only on the
        /// platform
        fn on(platform: Platform) -> Self {
            Self {
                tool: ToolRequirement::builder()
                    .tool("github:aonyx-ai/stiles")
                    .executable("stiles")
                    .exact_version_on(ToolVersion::new("0.6.0"), [platform])
                    .build(),
                ran: AtomicBool::new(false),
            }
        }
    }

    impl Action for Limited {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("limited")
        }

        fn tools(&self) -> Vec<ToolRequirement> {
            vec![taplo(), self.tool.clone()]
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            self.ran.store(true, Ordering::SeqCst);

            Outcome::Passed { summary: None }
        }
    }

    /// Returns the operating system of the run of the test
    fn this_system() -> OperatingSystem {
        if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::Macos
        } else {
            OperatingSystem::Linux
        }
    }

    /// Returns a platform with an operating system that is not the one of the
    /// run of the test
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

        format!("{}/{architecture}", this_system())
    }

    /// An action that reads no arguments and passes
    struct Probe;

    impl Action for Probe {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("probe")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// An argument set that no value can build
    struct Unreadable;

    impl Args for Unreadable {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([Argument::builder()
                .name(argument_name!("fix"))
                .shape(ArgumentShape::Boolean)
                .documentation("Rewrite the files that the tool can format")
                .build()])
        }

        fn from_values(_values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Err(ReadArgsError::MissingValue {
                name: argument_name!("fix"),
            })
        }
    }

    /// An action whose arguments never build from the values of a run
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

    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Drives an erased run to its outcome without a runtime
    fn drive(action: &dyn ErasedAction, values: &ArgsValues) -> Outcome {
        let context = context();
        let mut future = pin!(action.run(&context, values));
        let mut task_context = TaskContext::from_waker(Waker::noop());
        loop {
            match future.as_mut().poll(&mut task_context) {
                Poll::Ready(outcome) => return outcome,
                Poll::Pending => {}
            }
        }
    }

    /// Drives a bound run to its outcome without a runtime
    fn drive_bound(bound: &dyn BoundAction) -> Outcome {
        let context = context();
        let mut future = pin!(bound.run(&context));
        let mut task_context = TaskContext::from_waker(Waker::noop());
        loop {
            match future.as_mut().poll(&mut task_context) {
                Poll::Ready(outcome) => return outcome,
                Poll::Pending => {}
            }
        }
    }

    /// Returns the values of a run that asks for a repair
    fn fixing() -> ArgsValues {
        ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
    }

    /// Compiles only when every action has an erased view
    fn require_erased<A: Action + 'static>(action: A) -> Box<dyn ErasedAction> {
        Box::new(action)
    }

    #[test]
    fn arguments_of_an_action_without_arguments_is_empty() {
        let action = Probe;

        let schema = ErasedAction::arguments(&action);

        assert!(schema.arguments().is_empty());
    }

    #[test]
    fn arguments_returns_the_schema_of_the_arguments() {
        let action = Unbuildable;

        let schema = ErasedAction::arguments(&action);

        assert_eq!(schema.arguments().len(), 1);
    }

    #[test]
    fn bind_reports_arguments_that_it_cannot_read() {
        let action = Unbuildable;
        let values = ArgsValues::empty();

        let bound = ErasedAction::bind(&action, &values);

        assert!(matches!(bound, Err(ReadArgsError::MissingValue { .. })));
    }

    #[test]
    fn bound_access_of_an_action_that_declares_a_read_is_read() {
        let action = Formatter;
        let values = ArgsValues::empty();
        let bound = ErasedAction::bind(&action, &values).unwrap();

        let access = bound.access(&context());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn bound_access_of_an_action_that_declares_nothing_is_write() {
        let action = Probe;
        let values = ArgsValues::empty();
        let bound = ErasedAction::bind(&action, &values).unwrap();

        let access = bound.access(&context());

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn bound_access_reads_the_arguments_that_the_action_was_bound_to() {
        let action = Formatter;
        let values = fixing();
        let bound = ErasedAction::bind(&action, &values).unwrap();

        let access = bound.access(&context());

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn bound_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Bound<'static, Formatter>>();
    }

    #[test]
    fn bound_run_of_an_action_whose_tool_has_no_build_here_skips() {
        let action = Limited::on(another_platform());
        let values = ArgsValues::empty();
        let bound = ErasedAction::bind(&action, &values).unwrap();

        let outcome = drive_bound(bound.as_ref());

        assert!(matches!(outcome, Outcome::Skipped { .. }));
    }

    #[test]
    fn bound_run_produces_the_outcome_of_the_action() {
        let action = Probe;
        let values = ArgsValues::empty();
        let bound = ErasedAction::bind(&action, &values).unwrap();

        let outcome = drive_bound(bound.as_ref());

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn bound_trait_object_is_send() {
        fn assert_send<T: Send + ?Sized>() {}

        assert_send::<dyn BoundAction>();
    }

    #[test]
    fn bound_trait_object_is_sync() {
        fn assert_sync<T: Sync + ?Sized>() {}

        assert_sync::<dyn BoundAction>();
    }

    #[test]
    fn every_action_becomes_a_trait_object() {
        let erased = require_erased(Probe);

        assert_eq!(erased.name().get(), "probe");
    }

    #[test]
    fn name_returns_the_name_of_the_action() {
        let action = Probe;

        let name = ErasedAction::name(&action);

        assert_eq!(name.get(), "probe");
    }

    #[test]
    fn operation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Operation>();
    }

    #[test]
    fn run_of_an_action_whose_tool_has_a_build_here_runs_the_action() {
        let action = Limited::on(Platform::new(this_system()));

        let _outcome = drive(&action, &ArgsValues::empty());

        assert!(action.ran.load(Ordering::SeqCst));
    }

    #[test]
    fn run_of_an_action_whose_tool_has_no_build_here_gives_the_tool_and_the_platform() {
        let action = Limited::on(another_platform());

        let outcome = drive(&action, &ArgsValues::empty());

        let Outcome::Skipped { reason } = outcome else {
            panic!("expected the run to skip, got {outcome:?}");
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
    fn run_of_an_action_whose_tool_has_no_build_here_starts_no_run() {
        let action = Limited::on(another_platform());

        let _outcome = drive(&action, &ArgsValues::empty());

        assert!(!action.ran.load(Ordering::SeqCst));
    }

    #[test]
    fn run_produces_the_outcome_of_the_action() {
        let action = Probe;
        let values = ArgsValues::empty();

        let outcome = drive(&action, &values);

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_reports_arguments_that_it_cannot_read() {
        let action = Unbuildable;
        let values = ArgsValues::empty();

        let outcome = drive(&action, &values);

        assert!(matches!(outcome, Outcome::Errored { .. }));
    }

    #[test]
    fn tools_of_an_action_that_declares_nothing_is_empty() {
        let action = Probe;

        let tools = ErasedAction::tools(&action);

        assert!(tools.is_empty());
    }

    #[test]
    fn tools_returns_what_the_action_declares() {
        let action = Wrapper;

        let tools = ErasedAction::tools(&action);

        assert_eq!(tools, vec![taplo()]);
    }

    #[test]
    fn trait_object_is_send() {
        fn assert_send<T: Send + ?Sized>() {}

        assert_send::<dyn ErasedAction>();
    }

    #[test]
    fn trait_object_is_sync() {
        fn assert_sync<T: Sync + ?Sized>() {}

        assert_sync::<dyn ErasedAction>();
    }
}
