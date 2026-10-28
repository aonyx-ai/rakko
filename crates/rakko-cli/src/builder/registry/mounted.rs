use std::fmt;

use rakko_action::{ErasedAction, Name};

use crate::builder::members::MergedArgument;
use crate::erased_command::ErasedCommand;
use crate::erased_group::ErasedGroup;

/// One entry of the command tree: an action, a command, or a group
///
/// The projection derives a command from an action, it places a command that
/// the harness wrote as it is, and it derives the flags of a group from the
/// arguments of its members when the harness mounts the group. Each of the
/// three carries a name and a place in one flat tree, so the registry holds
/// them as one type. Their flags and their runs differ, so the code that
/// renders the flags, checks them at the mount, and drives a run tells the
/// three apart.
pub(crate) enum Mounted {
    /// An action, whose command the projection derives
    Action(Box<dyn ErasedAction>),

    /// A command that the harness wrote
    Command(Box<dyn ErasedCommand>),

    /// A group that the harness wrote, with the flags that its members give it
    Group {
        /// The group, which holds its members
        group: Box<dyn ErasedGroup>,
        /// The flags of the group, merged from the arguments of its members
        /// when the harness mounted it
        arguments: Vec<MergedArgument>,
    },
}

impl Mounted {
    /// Returns the name that identifies the entry
    pub(crate) fn name(&self) -> Name {
        match self {
            Self::Action(action) => action.name(),
            Self::Command(command) => command.name(),
            Self::Group { group, .. } => group.name(),
        }
    }
}

/// Shows the kind and the name of the entry, the way a message names it
impl fmt::Display for Mounted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action(action) => write!(formatter, "action '{}'", action.name()),
            Self::Command(command) => write!(formatter, "command '{}'", command.name()),
            Self::Group { group, .. } => write!(formatter, "group '{}'", group.name()),
        }
    }
}
