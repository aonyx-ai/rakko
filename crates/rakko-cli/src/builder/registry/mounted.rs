use std::fmt;

use rakko_action::{ArgsSchema, ErasedAction, Name};

use crate::erased_command::ErasedCommand;

/// One entry of the command tree: an action or a command
///
/// The projection derives a command from an action, and it places a command
/// that the harness wrote as it is. Everything between the mount and the run
/// treats the two alike: each carries a name, arguments that become flags,
/// and a place in one flat tree. Only the run differs, so this type holds
/// both until the run separates them.
pub(crate) enum Mounted {
    /// An action, whose command the projection derives
    Action(Box<dyn ErasedAction>),

    /// A command that the harness wrote
    Command(Box<dyn ErasedCommand>),
}

impl Mounted {
    /// Returns the name that identifies the entry
    pub(crate) fn name(&self) -> Name {
        match self {
            Self::Action(action) => action.name(),
            Self::Command(command) => command.name(),
        }
    }

    /// Returns the description of the arguments that the entry reads
    pub(crate) fn arguments(&self) -> ArgsSchema {
        match self {
            Self::Action(action) => action.arguments(),
            Self::Command(command) => command.arguments(),
        }
    }
}

/// Shows the kind and the name of the entry, the way a message names it
impl fmt::Display for Mounted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action(action) => write!(formatter, "action '{}'", action.name()),
            Self::Command(command) => write!(formatter, "command '{}'", command.name()),
        }
    }
}
