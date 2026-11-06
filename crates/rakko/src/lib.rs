//! The contract that every Rakko action depends on
//!
//! An action is a library crate that does one piece of project maintenance.
//! This crate holds what every action and every harness shares: the trait
//! that an action implements, the name that identifies an action, the
//! arguments that a run reads, the context that an action reads when it runs,
//! the finding that an action produces, and the outcome that the run returns.
//! It also holds the view of an action that hides its type, which is how a
//! registry holds many actions at once, and the list of such views that a
//! bundle exports.
//!
//! This crate builds no command line. A harness is the small binary that a
//! project runs to maintain itself, and the author of a harness starts with
//! [`rakko-cli`][rakko-cli]. That crate turns the actions that a harness
//! mounts into a command line.
//!
//! A group is a named list of actions, such as the actions that guard a
//! commit. The command line of a harness runs a group as one command. This
//! crate holds the trait that a group implements, so a crate that provides a
//! group depends on this crate alone.
//!
//! A run takes three steps. The machinery binds the values of the run to the
//! arguments of the action. Then it asks the bound action how the run uses
//! the project. Then it runs the same bound action. The declaration and the
//! run therefore see the same arguments. The shorthand [`ErasedAction::run`]
//! binds and runs in one call, without the question, for a caller that does
//! not ask.
//!
//! The crate stays small on purpose. Everything in it reaches every action and
//! every project that mounts one, so an addition here is a change for all of
//! them.
//!
//! [`ErasedAction::run`]: ErasedAction::run
//! [rakko-cli]: https://docs.rs/rakko-cli

/// How a run of an action uses the project
mod access;
/// The trait that every action implements
mod action;
/// Types for the arguments that a run of an action reads
mod args;
/// The view of an action whose arguments are bound for one run
mod bound_action;
/// The list of actions that a bundle exports
mod bundle;
/// Types for the data that an action reads when it runs
mod context;
/// The view of an action that hides its type
mod erased_action;
/// Types for a problem that an action found in a project
mod finding;
/// The named list of actions that the command line runs as one command
mod group;
/// Types for the identifier of an action
mod name;
/// Types for the result of an action run
mod outcome;
/// The spelling of a path that an action reports
pub mod path;
/// The seal on the views of an action that hide its type
mod seal;

pub use self::access::Access;
pub use self::action::Action;
pub use self::args::{
    Args, ArgsSchema, ArgsValues, Argument, ArgumentName, ArgumentShape, ArgumentValue,
    Documentation, ReadArgsError,
};
pub use self::bound_action::BoundAction;
pub use self::bundle::Bundle;
pub use self::context::{Context, ProjectRoot};
pub use self::erased_action::ErasedAction;
pub use self::finding::{
    Column, DirectoryPath, FilePath, Finding, FindingMessage, Line, Location,
    ParseDirectoryPathError, ParseFilePathError, Position, Span,
};
pub use self::group::Group;
pub use self::name::{Name, ParseNameError};
pub use self::outcome::{Outcome, SkipReason, Summary};
