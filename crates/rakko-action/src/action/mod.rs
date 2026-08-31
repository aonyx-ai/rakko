use std::future::Future;

use crate::args::Args;
use crate::context::Context;
use crate::name::Name;
use crate::outcome::Outcome;

/// The unit of maintenance work
///
/// An action examines a project and reports what it found as an [`Outcome`].
/// The trait describes the action as data: the name that identifies it and
/// the type of the arguments that a run reads. The machinery reads this data without running the action, for
/// example to show the action in a list.
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
// action[impl action.send]
// action[impl action.sync]
pub trait Action: Send + Sync {
    /// The arguments that a run of the action reads
    ///
    /// Each action defines its own type, and that type implements [`Args`].
    /// It describes itself as data, so that a projection can build a command
    /// before a run exists, and it builds itself from what the machinery
    /// parsed. An action that reads no arguments uses the unit type.
    ///
    /// [`run`]: Action::run
    // action[impl action.args]
    type Args: Args;

    /// Returns the name that identifies the action
    // action[impl action.name]
    fn name(&self) -> Name;

    /// Runs the action against the project that the context names
    ///
    /// The method returns a future, and the caller drives that future to get
    /// the [`Outcome`]. The run reports every result in the outcome: problems
    /// travel as findings in [`Outcome::Failed`], problems that the action
    /// repaired travel as repairs in [`Outcome::Changed`], and an error that
    /// stops the run travels in [`Outcome::Errored`]. The method itself has no
    /// error type.
    // action[impl run.outcome]
    // action[impl run.wait]
    // action[impl run.send]
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

    // action[verify action.send]
    #[test]
    fn action_is_send() {
        require_action_send::<Probe>();
    }

    // action[verify action.sync]
    #[test]
    fn action_is_sync() {
        require_action_sync::<Probe>();
    }

    // action[verify action.args]
    #[test]
    fn args_are_send() {
        require_args_send::<Probe>();
    }

    // action[verify action.args]
    #[test]
    fn args_are_sync() {
        require_args_sync::<Probe>();
    }

    // action[verify action.name]
    #[test]
    fn name_returns_what_the_action_declares() {
        let action = Probe;

        assert_eq!(action.name().get(), "probe");
    }

    // action[verify run.send]
    #[test]
    fn run_is_send() {
        let action = Probe;
        let context = Context::builder().root("/tmp/project").build();
        let args = ();

        require_run_send(&action, &context, &args);
    }

    // action[verify run.outcome]
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

    // action[verify run.wait]
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
