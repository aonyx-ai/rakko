use std::future::Future;
use std::pin::Pin;

use clawless::CommandResult;
use clawless::context::Context as ClawlessContext;
use rakko_action::{Args, ArgsSchema, ArgsValues, Context, Name};

use crate::command::Command;

/// One command behind an interface that hides its type
///
/// The [`Command`] trait is written for the author of a command, so it
/// carries the type of the arguments that the command reads. The builder
/// holds many commands at once, beside the actions, and cannot name a
/// different type for each of them, so it holds erased commands instead, the
/// way a registry holds erased actions.
///
/// An erased command answers everything that the projection needs without
/// naming the type of the command: the name that identifies it, the
/// description of the arguments that become flags, and a run that takes the
/// values of those flags. The conversion from those values into the arguments
/// of the command happens inside, where the type is still known.
///
/// Every command has an erased view, because this trait is implemented for
/// all of them. A harness never writes this implementation. It names the
/// trait once, where it boxes the commands that it mounts.
///
/// # Examples
///
/// ```
/// use rakko_action::{Context, Name, action_name};
/// use rakko_cli::clawless::CommandResult;
/// use rakko_cli::clawless::context::Context as ClawlessContext;
/// use rakko_cli::{Command, ErasedCommand};
///
/// struct Serve;
///
/// impl Command for Serve {
///     type Args = ();
///
///     fn name(&self) -> Name {
///         action_name!("serve")
///     }
///
///     async fn run(
///         &self,
///         _project: &Context,
///         _clawless: &ClawlessContext,
///         _args: &Self::Args,
///     ) -> CommandResult {
///         Ok(())
///     }
/// }
///
/// let commands: Vec<Box<dyn ErasedCommand>> = vec![Box::new(Serve)];
///
/// assert_eq!(commands[0].name().get(), "serve");
/// ```
// cli[impl erased.object]
// cli[impl erased.send]
// cli[impl erased.sync]
pub trait ErasedCommand: Send + Sync {
    /// Returns the name that identifies the command
    // cli[impl erased.name]
    fn name(&self) -> Name;

    /// Returns the description of the arguments that the command reads
    ///
    /// The projection reads the description to build the flags of the command
    /// before a run exists.
    // cli[impl erased.arguments]
    fn arguments(&self) -> ArgsSchema;

    /// Runs the command against the project that the context names
    ///
    /// The values hold what the command line parsed for the arguments of the
    /// command. The run converts them into the arguments of the command, and
    /// a conversion that fails is an error of the run, so a user who gave a
    /// value that the command cannot read learns which argument it was.
    // cli[impl erased.run]
    // cli[impl erased.unreadable]
    fn run<'a>(
        &'a self,
        project: &'a Context,
        clawless: &'a ClawlessContext,
        values: &'a ArgsValues,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>>;
}

// cli[impl erased.total]
impl<C: Command> ErasedCommand for C {
    fn name(&self) -> Name {
        Command::name(self)
    }

    fn arguments(&self) -> ArgsSchema {
        <C::Args as Args>::schema()
    }

    fn run<'a>(
        &'a self,
        project: &'a Context,
        clawless: &'a ClawlessContext,
        values: &'a ArgsValues,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            let args = <C::Args as Args>::from_values(values)?;

            Command::run(self, project, clawless, &args).await
        })
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;
    use std::pin::pin;
    use std::task::{Context as TaskContext, Poll, Waker};

    use clawless::event::{EventReceiver, event_channel};
    use clawless::output::Output;
    use rakko_action::{Argument, ArgumentShape, ReadArgsError, argument_name};

    use super::*;

    /// A command that reads no arguments and succeeds
    struct Probe;

    impl Command for Probe {
        type Args = ();

        fn name(&self) -> Name {
            "probe".parse().expect("the test names a command correctly")
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

    /// A command whose arguments no value can build
    struct Demanding;

    impl Command for Demanding {
        type Args = Unreadable;

        fn name(&self) -> Name {
            "demanding"
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

    /// An argument set that no value can build
    struct Unreadable;

    impl Args for Unreadable {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([Argument::builder()
                .name(argument_name!("port"))
                .shape(ArgumentShape::Integer)
                .documentation("Listen on this port")
                .build()])
        }

        fn from_values(_values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Err(ReadArgsError::MissingValue {
                name: argument_name!("port"),
            })
        }
    }

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    /// Returns the context of Clawless that a test hands to a run
    ///
    /// The receiver travels with the context, because a run that writes
    /// output fails as soon as nothing reads it.
    fn clawless() -> (ClawlessContext, EventReceiver) {
        let (sender, receiver) = event_channel();
        let context = ClawlessContext::builder()
            .current_working_directory(Path::new("/tmp/project"))
            .output(Output::new(sender))
            .build()
            .expect("the test names a working directory");

        (context, receiver)
    }

    /// Returns the result of a run of the given erased command
    ///
    /// # Errors
    ///
    /// Returns the error of the run when the command failed.
    fn drive(command: &dyn ErasedCommand) -> CommandResult {
        let project = Context::builder().root("/tmp/project").build();
        let (clawless, _receiver) = clawless();
        let values = ArgsValues::empty();
        let mut future = pin!(command.run(&project, &clawless, &values));
        let mut task_context = TaskContext::from_waker(Waker::noop());

        match future.as_mut().poll(&mut task_context) {
            Poll::Ready(result) => result,
            Poll::Pending => panic!("expected the run to end without waiting"),
        }
    }

    /// Compiles only when every command has an erased view
    fn erase<C: Command + 'static>(command: C) -> Box<dyn ErasedCommand> {
        Box::new(command)
    }

    // cli[verify erased.arguments]
    #[test]
    fn arguments_returns_the_description_of_the_command() {
        let command: &dyn ErasedCommand = &Demanding;

        let schema = command.arguments();

        assert_eq!(schema, Unreadable::schema());
    }

    // cli[verify erased.object]
    #[test]
    fn commands_of_different_types_share_one_collection() {
        let commands: Vec<Box<dyn ErasedCommand>> = vec![Box::new(Probe), Box::new(Demanding)];

        let names: Vec<String> = commands
            .iter()
            .map(|command| command.name().to_string())
            .collect();

        assert_eq!(names, ["probe", "demanding"]);
    }

    // cli[verify erased.send]
    #[test]
    fn erased_command_is_send() {
        assert_send::<Box<dyn ErasedCommand>>();
    }

    // cli[verify erased.sync]
    #[test]
    fn erased_command_is_sync() {
        assert_sync::<Box<dyn ErasedCommand>>();
    }

    // cli[verify erased.total]
    #[test]
    fn every_command_has_an_erased_view() {
        let command = erase(Probe);

        assert_eq!(command.name().get(), "probe");
    }

    // cli[verify erased.name]
    #[test]
    fn name_returns_the_name_of_the_command() {
        let command: &dyn ErasedCommand = &Probe;

        assert_eq!(command.name().get(), "probe");
    }

    // cli[verify erased.run]
    #[test]
    fn run_produces_the_result_of_the_command() {
        let result = drive(&Probe);

        assert!(result.is_ok());
    }

    // cli[verify erased.unreadable]
    #[test]
    fn run_with_values_that_build_no_arguments_reports_the_argument() {
        let Err(error) = drive(&Demanding) else {
            panic!("expected the run to report an error");
        };

        assert!(error.to_string().contains("port"));
    }
}
