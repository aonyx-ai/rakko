use std::future::Future;
use std::pin::Pin;

use crate::action::Action;
use crate::args::{Args, ArgsSchema, ArgsValues};
use crate::context::Context;
use crate::name::Name;
use crate::outcome::Outcome;

/// One action behind an interface that hides its type
///
/// The [`Action`] trait is written for the author of an action, so it carries
/// the type of the arguments that the action reads. A registry holds many
/// actions at once and cannot name a different type for each of them, so it
/// holds erased actions instead.
///
/// An erased action answers everything that the machinery needs without
/// naming the type of the action: the name that identifies it, the
/// description of the arguments that a projection turns into a command, and a
/// run that takes the values of that command. The conversion from those values
/// into the arguments of the action happens inside, where the type is still
/// known.
///
/// Every action has an erased view, because this trait is implemented for all
/// of them. An action author never writes this implementation and never names
/// this trait.
///
/// # Examples
///
/// ```
/// use rakko_action::{Action, Context, ErasedAction, Name, Outcome, action_name};
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
///         Outcome::Passed
///     }
/// }
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LineCount)];
///
/// assert_eq!(actions[0].name().get(), "line-count");
/// ```
// action[impl erased.object]
// action[impl erased.send]
// action[impl erased.sync]
pub trait ErasedAction: Send + Sync {
    /// Returns the name that identifies the action
    // action[impl erased.name]
    fn name(&self) -> Name;

    /// Returns the description of the arguments that the action reads
    ///
    /// A projection reads the description to build a command before a run
    /// exists.
    // action[impl erased.arguments]
    fn arguments(&self) -> ArgsSchema;

    /// Runs the action against the project that the context names
    ///
    /// The values hold what the machinery parsed for the arguments of the
    /// action. The run converts them into the arguments of the action, and a
    /// conversion that fails travels in [`Outcome::Errored`], so this method
    /// has no error type of its own.
    // action[impl erased.run]
    // action[impl erased.unreadable]
    fn run<'a>(
        &'a self,
        context: &'a Context,
        values: &'a ArgsValues,
    ) -> Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;
}

// action[impl erased.total]
impl<A: Action> ErasedAction for A {
    fn name(&self) -> Name {
        Action::name(self)
    }

    fn arguments(&self) -> ArgsSchema {
        <A::Args as Args>::schema()
    }

    fn run<'a>(
        &'a self,
        context: &'a Context,
        values: &'a ArgsValues,
    ) -> Pin<Box<dyn Future<Output = Outcome> + Send + 'a>> {
        Box::pin(async move {
            match <A::Args as Args>::from_values(values) {
                Ok(args) => Action::run(self, context, &args).await,
                Err(error) => Outcome::Errored {
                    source: Box::new(error),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::pin::pin;
    use std::task::{Context as TaskContext, Poll, Waker};

    use super::*;
    use crate::action_name;
    use crate::args::{Argument, ArgumentName, ArgumentShape, ReadArgsError};

    /// An action that reads no arguments and passes
    struct Probe;

    impl Action for Probe {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("probe")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed
        }
    }

    /// An argument set that no value can build
    struct Unreadable;

    impl Args for Unreadable {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([Argument::builder()
                .name("fix")
                .shape(ArgumentShape::Boolean)
                .documentation("Rewrite the files that the tool can format")
                .build()])
        }

        fn from_values(_values: &ArgsValues) -> Result<Self, ReadArgsError> {
            Err(ReadArgsError::MissingValue {
                name: ArgumentName::new("fix"),
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
            Outcome::Passed
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
            if let Poll::Ready(outcome) = future.as_mut().poll(&mut task_context) {
                return outcome;
            }
        }
    }

    /// Compiles only when every action has an erased view
    fn require_erased<A: Action + 'static>(action: A) -> Box<dyn ErasedAction> {
        Box::new(action)
    }

    // action[verify erased.arguments]
    #[test]
    fn arguments_returns_the_schema_of_the_arguments() {
        let action = Unbuildable;

        let schema = ErasedAction::arguments(&action);

        assert_eq!(schema.arguments().len(), 1);
    }

    // action[verify erased.arguments]
    #[test]
    fn arguments_of_an_action_without_arguments_is_empty() {
        let action = Probe;

        let schema = ErasedAction::arguments(&action);

        assert!(schema.arguments().is_empty());
    }

    // action[verify erased.object]
    // action[verify erased.total]
    #[test]
    fn every_action_becomes_a_trait_object() {
        let erased = require_erased(Probe);

        assert_eq!(erased.name().get(), "probe");
    }

    // action[verify erased.name]
    #[test]
    fn name_returns_the_name_of_the_action() {
        let action = Probe;

        let name = ErasedAction::name(&action);

        assert_eq!(name.get(), "probe");
    }

    // action[verify erased.run]
    #[test]
    fn run_produces_the_outcome_of_the_action() {
        let action = Probe;
        let values = ArgsValues::empty();

        let outcome = drive(&action, &values);

        assert!(matches!(outcome, Outcome::Passed));
    }

    // action[verify erased.unreadable]
    #[test]
    fn run_reports_arguments_that_it_cannot_read() {
        let action = Unbuildable;
        let values = ArgsValues::empty();

        let outcome = drive(&action, &values);

        assert!(matches!(outcome, Outcome::Errored { .. }));
    }

    // action[verify erased.send]
    #[test]
    fn trait_object_is_send() {
        fn assert_send<T: Send + ?Sized>() {}

        assert_send::<dyn ErasedAction>();
    }

    // action[verify erased.sync]
    #[test]
    fn trait_object_is_sync() {
        fn assert_sync<T: Sync + ?Sized>() {}

        assert_sync::<dyn ErasedAction>();
    }
}
