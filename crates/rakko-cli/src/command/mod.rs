use std::future::Future;

use clawless::CommandResult;
use clawless::context::Context as ClawlessContext;
use rakko_action::{Args, Context, Name};

/// A command that a harness writes for an activity that does not fit an action
///
/// An action runs once and reports one outcome, and the projection derives
/// one command from each action that a harness mounts. Some maintenance
/// activities of a project do not fit that shape: a run that drives many
/// actions and reports on all of them, a watch that never returns, or a
/// server. A harness writes such an activity as a command and mounts it
/// beside its actions, and the command line lists both as one flat list.
///
/// The trait describes the command as data, the way the [`Action`] trait
/// describes an action: the name that identifies it and the type of the
/// arguments that a run reads. The projection builds the command from that
/// data before a run exists, so a command names no flag and no syntax, and
/// its arguments render under the rules that the arguments of an action
/// follow.
///
/// The run is arbitrary code. It receives the context of the project and the
/// context of Clawless, writes its output through Clawless, and returns what
/// a Clawless command returns: success or an error, and no value. The
/// projection hands a command no view of the actions that the harness
/// mounted. A command that runs actions calls them as the libraries that they
/// are.
///
/// The framework drives a command on a runtime of its own and can move the
/// run to a different thread. The trait therefore requires [`Send`] and
/// [`Sync`] of the command and of its arguments, and [`run`] returns a future
/// that is [`Send`].
///
/// # Examples
///
/// The trait declares [`run`] with an explicit return type, but an
/// implementation writes a plain `async fn`:
///
/// ```
/// use rakko_action::{Context, Name, action_name};
/// use rakko_cli::Command;
/// use rakko_cli::clawless::CommandResult;
/// use rakko_cli::clawless::context::Context as ClawlessContext;
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
///         project: &Context,
///         clawless: &ClawlessContext,
///         _args: &Self::Args,
///     ) -> CommandResult {
///         let root = project.root().get().display();
///         clawless.output().message(format!("serving {root}")).await?;
///
///         Ok(())
///     }
/// }
/// ```
///
/// [`Action`]: rakko_action::Action
/// [`run`]: Command::run
// cli[impl written.send]
// cli[impl written.sync]
pub trait Command: Send + Sync {
    /// The arguments that a run of the command reads
    ///
    /// The type implements [`Args`], the vocabulary of the contract crate, so
    /// the projection renders the arguments of a command as it renders the
    /// arguments of an action. A command that reads no arguments uses the
    /// unit type.
    // cli[impl written.args]
    type Args: Args;

    /// Returns the name that identifies the command
    // cli[impl written.name]
    fn name(&self) -> Name;

    /// Runs the command against the project that the context names
    ///
    /// The method returns a future, and the framework drives that future to
    /// its end. The command writes what it has to say through the output of
    /// the Clawless context, and it returns an error when it fails. The
    /// command line turns success into a clean exit code and an error into
    /// the code of a run that could not answer, and it shows the error.
    // cli[impl written.run]
    // cli[impl written.wait]
    // cli[impl written.send]
    fn run(
        &self,
        project: &Context,
        clawless: &ClawlessContext,
        args: &Self::Args,
    ) -> impl Future<Output = CommandResult> + Send;
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::{Path, PathBuf};
    use std::pin::{Pin, pin};
    use std::sync::Mutex;
    use std::task::{Context as TaskContext, Poll, Waker};

    use clawless::event::{EventReceiver, event_channel};
    use clawless::output::Output;
    use rakko_action::ArgsSchema;

    use super::*;

    /// A command that yields once, records the root it received, and succeeds
    #[derive(Default)]
    struct Probe {
        /// The root of the project that a run received
        root: Mutex<Option<PathBuf>>,
    }

    impl Command for Probe {
        type Args = ();

        fn name(&self) -> Name {
            "probe".parse().expect("the test names a command correctly")
        }

        async fn run(
            &self,
            project: &Context,
            _clawless: &ClawlessContext,
            _args: &Self::Args,
        ) -> CommandResult {
            YieldOnce::default().await;

            let mut root = self.root.lock().expect("the test holds the lock alone");
            *root = Some(project.root().get().to_path_buf());

            Ok(())
        }
    }

    /// A future that returns control once before it resolves
    #[derive(Default)]
    struct YieldOnce {
        /// Whether the future has returned control already
        yielded: bool,
    }

    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<()> {
            if self.yielded {
                Poll::Ready(())
            } else {
                self.yielded = true;
                Poll::Pending
            }
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

    /// Compiles only when the trait makes every command `Send`
    fn require_command_send<C: Command>() {
        assert_send::<C>();
    }

    /// Compiles only when the trait makes every command `Sync`
    fn require_command_sync<C: Command>() {
        assert_sync::<C>();
    }

    /// Compiles only when the trait makes the future of every run `Send`
    fn require_run_send<C: Command>(
        command: &C,
        project: &Context,
        clawless: &ClawlessContext,
        args: &C::Args,
    ) {
        fn require_send<T: Send>(_: T) {}
        require_send(command.run(project, clawless, args));
    }

    /// Returns the description of the arguments of a command
    fn schema_of<C: Command>() -> ArgsSchema {
        <C::Args as Args>::schema()
    }

    // cli[verify written.args]
    #[test]
    fn args_describe_themselves_in_the_vocabulary() {
        let schema = schema_of::<Probe>();

        assert!(schema.arguments().is_empty());
    }

    // cli[verify written.send]
    #[test]
    fn command_is_send() {
        require_command_send::<Probe>();
    }

    // cli[verify written.sync]
    #[test]
    fn command_is_sync() {
        require_command_sync::<Probe>();
    }

    // cli[verify written.name]
    #[test]
    fn name_returns_what_the_command_declares() {
        let command = Probe::default();

        assert_eq!(command.name().get(), "probe");
    }

    // cli[verify written.send]
    #[test]
    fn run_is_send() {
        let command = Probe::default();
        let project = Context::builder().root("/tmp/project").build();
        let (clawless, _receiver) = clawless();

        require_run_send(&command, &project, &clawless, &());
    }

    // cli[verify written.run]
    #[test]
    fn run_produces_the_result_when_driven_again() {
        let command = Probe::default();
        let project = Context::builder().root("/tmp/project").build();
        let (clawless, _receiver) = clawless();
        let mut future = pin!(command.run(&project, &clawless, &()));
        let mut task_context = TaskContext::from_waker(Waker::noop());
        let Poll::Pending = future.as_mut().poll(&mut task_context) else {
            panic!("expected the run to wait first");
        };

        let result = future.as_mut().poll(&mut task_context);

        assert!(matches!(result, Poll::Ready(Ok(()))));
    }

    // cli[verify written.run]
    #[test]
    fn run_receives_the_root_of_the_project() {
        let command = Probe::default();
        let project = Context::builder().root("/tmp/project").build();
        let (clawless, _receiver) = clawless();
        let mut future = pin!(command.run(&project, &clawless, &()));
        let mut task_context = TaskContext::from_waker(Waker::noop());
        let _first = future.as_mut().poll(&mut task_context);
        let _second = future.as_mut().poll(&mut task_context);

        let root = command.root.lock().expect("the test holds the lock alone");

        assert_eq!(root.as_deref(), Some(Path::new("/tmp/project")));
    }

    // cli[verify written.wait]
    #[test]
    fn run_returns_control_while_it_waits() {
        let command = Probe::default();
        let project = Context::builder().root("/tmp/project").build();
        let (clawless, _receiver) = clawless();
        let mut future = pin!(command.run(&project, &clawless, &()));
        let mut task_context = TaskContext::from_waker(Waker::noop());

        let state = future.as_mut().poll(&mut task_context);

        assert!(matches!(state, Poll::Pending));
    }
}
