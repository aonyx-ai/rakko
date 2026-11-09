//! The view of an action whose arguments are bound for one run

use std::future::Future;
use std::pin::Pin;

use crate::access::Access;
use crate::context::Context;
use crate::outcome::Outcome;
use crate::seal::SealedBound;

/// One action with the arguments of one run, behind an interface that hides
/// its type
///
/// An [`ErasedAction`] binds the values of a run to the arguments of the
/// action and returns this view. The view answers two questions with the same
/// arguments: how the run uses the project, and what the run found. So the
/// declaration describes the run that follows it. A caller can order several
/// runs by their declarations before it starts any of them.
///
/// The arguments are bound once. The view does not read the values again
/// when the caller asks for the access or runs the action. So a value that
/// cannot be read fails when the values are bound, and never between the
/// declaration and the run.
///
/// An author of an action, of a group, or of a harness never writes this
/// trait. The [`bind`] method of an erased action returns it. No crate outside
/// this one can implement the trait. So a later release can add a method to
/// it, and no implementation breaks.
///
/// # Examples
///
/// ```
/// use rakko::{Access, Action, ArgsValues, Context, ErasedAction, Name, Outcome, action_name};
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
///     fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
///         Access::Read
///     }
///
///     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
///         Outcome::Passed { summary: None }
///     }
/// }
///
/// let action: Box<dyn ErasedAction> = Box::new(LineCount);
/// let context = Context::builder().root("/tmp/project").build();
/// let values = ArgsValues::empty();
///
/// let bound = action.bind(&values).expect("the action reads no arguments");
///
/// assert_eq!(bound.access(&context), Access::Read);
/// ```
///
/// An implementation outside this crate does not compile, also for a type
/// that implements [`Action`]:
///
/// ```compile_fail
/// use std::future::Future;
/// use std::pin::Pin;
///
/// use rakko::{Access, Action, BoundAction, Context, Name, Outcome, action_name};
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
///     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
///         Outcome::Passed { summary: None }
///     }
/// }
///
/// impl BoundAction for LineCount {
///     fn access(&self, _context: &Context) -> Access {
///         Access::Read
///     }
///
///     fn run<'a>(
///         &'a self,
///         _context: &'a Context,
///     ) -> Pin<Box<dyn Future<Output = Outcome> + Send + 'a>> {
///         Box::pin(async { Outcome::Passed { summary: None } })
///     }
/// }
/// ```
///
/// [`Action`]: crate::Action
/// [`bind`]: crate::ErasedAction::bind
/// [`ErasedAction`]: crate::ErasedAction
pub trait BoundAction: Send + Sync + SealedBound {
    /// Returns how the run uses the project
    ///
    /// The answer comes from the arguments that the action was bound to and
    /// from the context. The method does no I/O, so a caller can ask before
    /// any run starts.
    fn access(&self, context: &Context) -> Access;

    /// Runs the action with the arguments that it was bound to
    ///
    /// The run reports every result in the outcome, as [`Action::run`] does,
    /// so this method has no error type.
    ///
    /// [`Action::run`]: crate::Action::run
    fn run<'a>(
        &'a self,
        context: &'a Context,
    ) -> Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;
}
