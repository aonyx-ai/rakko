use std::future::Future;

use crate::access::Access;
use crate::args::Args;
use crate::context::Context;
use crate::file_access::FileAccess;
use crate::name::Name;
use crate::outcome::Outcome;

/// The unit of maintenance work
///
/// An action examines a project and reports what it found as an [`Outcome`].
/// The trait describes the action as data: the name that identifies it and
/// the type of the arguments that a run reads. The machinery reads this data
/// without running the action, for example to show the action in a list.
///
/// Before a run, the machinery can ask the action how that run uses the
/// project, with the arguments of the run: whether the run writes, and which
/// files it reads and writes. An action that does not answer writes the whole
/// project. That answer is always safe and can be slow: a caller that reads
/// it never runs the action beside another action.
///
/// A scheduler can run many actions in parallel and can move each run to a
/// different thread. The trait therefore requires [`Send`] and [`Sync`] of
/// the action and of its arguments, and [`run`] returns a future that is
/// [`Send`].
///
/// # Examples
///
/// The trait declares [`run`] with an explicit return type, but an
/// implementation writes a plain `async fn`:
///
/// ```
/// use rakko_action::{Action, Context, Name, Outcome, action_name};
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
///         // Examine the project under `context.root()` here.
///         Outcome::Passed { summary: None }
///     }
/// }
/// ```
///
/// [`run`]: Action::run
pub trait Action: Send + Sync {
    /// The arguments that a run of the action reads
    ///
    /// Each action defines its own type, and that type implements [`Args`].
    /// It describes itself as data, so that a projection can build a command
    /// before a run exists, and it builds itself from what the machinery
    /// parsed. An action that reads no arguments uses the unit type.
    ///
    /// [`run`]: Action::run
    type Args: Args;

    /// Returns the name that identifies the action
    fn name(&self) -> Name;

    /// Returns how a run with the given arguments uses the project
    ///
    /// The machinery asks before the run, with the arguments and the context
    /// that the run receives. So the answer describes that run. Some actions
    /// write only when an argument asks for it, such as a formatter with a
    /// `fix` argument. Such an action answers [`Access::Read`] for a run
    /// without that argument.
    ///
    /// The method must not read a file or start a tool, because the answer
    /// has to be ready before the run starts. When only the configuration of
    /// the project can settle the answer, the action answers
    /// [`Access::Write`].
    ///
    /// An action that does not implement the method answers [`Access::Write`]
    /// for every run. That answer is safe, because a write gets the files
    /// that it writes to itself, and those are the whole project unless
    /// [`files`] narrows them. But a caller that orders runs by their
    /// declarations then makes an action that only reads wait for the actions
    /// that write.
    ///
    /// [`files`]: Action::files
    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Write
    }

    /// Returns the files that a run with the given arguments reads and writes
    ///
    /// The machinery asks before the run, with the arguments and the context
    /// that the run receives, as it does for [`access`]. The answer narrows
    /// the files that [`access`] covers, and it does not replace that answer:
    /// only a run that declares [`Access::Write`] writes the files of the
    /// write side. A run that declares [`Access::Read`] writes no file,
    /// whatever the write side covers.
    ///
    /// Each side covers the whole project or the files that its patterns
    /// match. The patterns describe the files that the tool selects when the
    /// project configures no selection, such as `**/*.toml` for a formatter
    /// of TOML files. They also describe the files that the action and its
    /// tool read as configuration, such as `mise.toml`, because another run
    /// can change those files too. The tool never receives the patterns, so
    /// they do not change what the run examines.
    ///
    /// The method must not read a file or start a tool, as for [`access`].
    /// A caller that needs the files that exist calls [`FileAccess::expand`]
    /// on the answer.
    ///
    /// An action that does not implement the method reads and writes the
    /// whole project, which is the answer that is always safe.
    ///
    /// [`access`]: Action::access
    fn files(&self, _context: &Context, _args: &Self::Args) -> FileAccess {
        FileAccess::default()
    }

    /// Runs the action against the project that the context names
    ///
    /// The method returns a future, and the caller drives that future to get
    /// the [`Outcome`]. The run reports every result in the outcome: problems
    /// travel as findings in [`Outcome::Failed`], problems that the action
    /// repaired travel as repairs in [`Outcome::Changed`], and an error that
    /// stops the run travels in [`Outcome::Errored`]. The method itself has no
    /// error type.
    fn run(&self, context: &Context, args: &Self::Args) -> impl Future<Output = Outcome> + Send;
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::pin::{Pin, pin};
    use std::task::{Context as TaskContext, Poll, Waker};

    use super::*;
    use crate::file_access::Files;

    /// A minimal action that yields once and then passes
    struct Probe;

    impl Action for Probe {
        type Args = ();

        fn name(&self) -> Name {
            "probe".parse().unwrap()
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            YieldOnce::default().await;
            Outcome::Passed { summary: None }
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

    /// Compiles only when the trait makes every action `Send`
    fn require_action_send<A: Action>() {
        assert_send::<A>();
    }

    /// Compiles only when the trait makes every action `Sync`
    fn require_action_sync<A: Action>() {
        assert_sync::<A>();
    }

    /// Compiles only when the trait makes the arguments of every action `Send`
    fn require_args_send<A: Action>() {
        assert_send::<A::Args>();
    }

    /// Compiles only when the trait makes the arguments of every action `Sync`
    fn require_args_sync<A: Action>() {
        assert_sync::<A::Args>();
    }

    /// Compiles only when the trait makes the future of every run `Send`
    fn require_run_send<A: Action>(action: &A, context: &Context, args: &A::Args) {
        fn require_send<T: Send>(_: T) {}
        require_send(action.run(context, args));
    }

    #[test]
    fn access_of_an_action_that_declares_nothing_is_write() {
        let action = Probe;
        let context = Context::builder().root("/tmp/project").build();

        let access = action.access(&context, &());

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn action_is_send() {
        require_action_send::<Probe>();
    }

    #[test]
    fn action_is_sync() {
        require_action_sync::<Probe>();
    }

    #[test]
    fn args_are_send() {
        require_args_send::<Probe>();
    }

    #[test]
    fn args_are_sync() {
        require_args_sync::<Probe>();
    }

    #[test]
    fn files_of_an_action_that_declares_nothing_read_the_whole_project() {
        let action = Probe;
        let context = Context::builder().root("/tmp/project").build();

        let files = action.files(&context, &());

        assert_eq!(files.reads(), &Files::Project);
    }

    #[test]
    fn files_of_an_action_that_declares_nothing_write_the_whole_project() {
        let action = Probe;
        let context = Context::builder().root("/tmp/project").build();

        let files = action.files(&context, &());

        assert_eq!(files.writes(), &Files::Project);
    }

    #[test]
    fn name_returns_what_the_action_declares() {
        let action = Probe;

        assert_eq!(action.name().get(), "probe");
    }

    #[test]
    fn run_is_send() {
        let action = Probe;
        let context = Context::builder().root("/tmp/project").build();
        let args = ();

        require_run_send(&action, &context, &args);
    }

    #[test]
    fn run_produces_the_outcome_when_driven_again() {
        let action = Probe;
        let context = Context::builder().root("/tmp/project").build();
        let args = ();
        let mut future = pin!(action.run(&context, &args));
        let mut task_context = TaskContext::from_waker(Waker::noop());
        let Poll::Pending = future.as_mut().poll(&mut task_context) else {
            panic!("expected the run to wait first");
        };

        let outcome = future.as_mut().poll(&mut task_context);

        assert!(matches!(outcome, Poll::Ready(Outcome::Passed { .. })));
    }

    #[test]
    fn run_returns_control_while_it_waits() {
        let action = Probe;
        let context = Context::builder().root("/tmp/project").build();
        let args = ();
        let mut future = pin!(action.run(&context, &args));
        let mut task_context = TaskContext::from_waker(Waker::noop());

        let state = future.as_mut().poll(&mut task_context);

        assert!(matches!(state, Poll::Pending));
    }
}
