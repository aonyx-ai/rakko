use std::future::Future;
use std::pin::Pin;

use crate::access::Access;
use crate::context::Context;
use crate::file_access::FileAccess;
use crate::outcome::Outcome;

/// One action with the arguments of one run, behind an interface that hides
/// its type
///
/// An [`ErasedAction`] binds the values of a run to the arguments of the
/// action and returns this view. The view answers three questions with the
/// same arguments: whether the run writes, which files it reads and writes,
/// and what the run found. So the declaration describes the run that follows
/// it. A caller can order several runs by their declarations before it starts
/// any of them.
///
/// The arguments are bound once. The view does not read the values again
/// when the caller asks for the access or runs the action. So a value that
/// cannot be read fails when the values are bound, and never between the
/// declaration and the run.
///
/// # Examples
///
/// ```
/// use rakko_action::{Access, Action, ArgsValues, Context, ErasedAction, Name, Outcome, action_name};
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
/// [`ErasedAction`]: crate::ErasedAction
pub trait BoundAction: Send + Sync {
    /// Returns how the run uses the project
    ///
    /// The answer comes from the arguments that the action was bound to and
    /// from the context. The method does no I/O, so a caller can ask before
    /// any run starts.
    fn access(&self, context: &Context) -> Access;

    /// Returns the files that the run reads and writes
    ///
    /// The answer comes from the arguments that the action was bound to and
    /// from the context, as for [`access`][BoundAction::access]. The method
    /// does no I/O. A caller that needs the files that exist calls
    /// [`FileAccess::expand`] on the answer.
    fn files(&self, context: &Context) -> FileAccess;

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
