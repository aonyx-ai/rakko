/// The entries that one value adds to the command line
mod entries;

use rakko::{Action, ErasedAction, Group};

use self::entries::Entries;
use crate::builder::members;
use crate::builder::registry::Mounted;
use crate::command::Command;
use crate::erased_command::ErasedCommand;

/// A value that a harness gives to the builder as it is
///
/// The [`mount`] method of the builder takes four kinds of value:
///
/// - an action
/// - a list of actions, such as a [`Bundle`]
/// - a command that implements [`Command`]
/// - a [`Group`]
///
/// The builder hides the type of each value itself, so a harness writes no
/// erased type when it mounts.
///
/// The parameter `K` states the kind of the value. The compiler infers it
/// from the value, so a harness does not write it. A type that is two kinds
/// at the same time, such as an action that is also a group, has two
/// implementations. For such a type, the harness gives the kind to the call,
/// as in `mount::<dyn Group>(value)`.
///
/// The builder alone implements this trait. A harness that needs another kind
/// of value turns it into one of these four.
///
/// [`Bundle`]: rakko::Bundle
/// [`mount`]: crate::Builder::mount
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no action, list of actions, command, or group",
    label = "the builder cannot mount this value",
    note = "the builder mounts a type that implements `Action`, `Command`, or `Group`, and a \
            list of actions such as a `Bundle`"
)]
pub trait Mount<K: ?Sized> {
    /// Returns the entries that the value adds to the command line
    ///
    /// The method runs the checks that only this kind of value needs, so it
    /// can panic as [`mount`] describes.
    ///
    /// [`mount`]: crate::Builder::mount
    #[doc(hidden)]
    fn entries(self) -> Entries;
}

/// Mounts one action, whose command the projection derives
impl<A: Action + 'static> Mount<dyn ErasedAction> for A {
    fn entries(self) -> Entries {
        Entries::new(vec![Mounted::Action(Box::new(self))])
    }
}

/// Mounts a list of actions, such as a bundle, in the order of the list
impl<I: IntoIterator<Item = Box<dyn ErasedAction>>> Mount<[Box<dyn ErasedAction>]> for I {
    fn entries(self) -> Entries {
        Entries::new(self.into_iter().map(Mounted::Action).collect())
    }
}

/// Mounts a command that the harness wrote, as it is
impl<C: Command + 'static> Mount<dyn ErasedCommand> for C {
    fn entries(self) -> Entries {
        Entries::new(vec![Mounted::Command(Box::new(self))])
    }
}

/// Mounts a group, with the flags that the arguments of its members give it
///
/// # Panics
///
/// Panics when the group lists two members with the same name, and when two
/// members declare one argument with two different shapes.
impl<G: Group + 'static> Mount<dyn Group> for G {
    fn entries(self) -> Entries {
        let name = self.name();
        members::refuse_repeated(&name, self.members());
        let arguments = members::merge(&name, self.members());

        Entries::new(vec![Mounted::Group {
            group: Box::new(self),
            arguments,
        }])
    }
}
