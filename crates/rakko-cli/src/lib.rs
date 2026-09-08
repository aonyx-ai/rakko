//! The command-line projection of the actions that a harness mounts
//!
//! A harness is the small binary that a project runs to maintain itself. It
//! mounts the actions that the project uses, and this crate turns them into a
//! command-line interface: one command for each action, the help text of each
//! command, and the flags that every command shares.
//!
//! A harness also writes commands of its own, for a maintenance activity of
//! the project that does not fit an action, and mounts them beside its
//! actions. Such a command implements the [`Command`] trait of this crate,
//! and it is an ordinary command of Clawless underneath: it receives the
//! context of Clawless and writes its output through it. This crate
//! re-exports Clawless, so a harness names what a command receives and
//! returns without a dependency of its own.
//!
//! Only a harness depends on this crate. An action depends on the contract
//! crate alone, so the command-line framework stays out of the build of an
//! action.
//!
//! The crate builds the command line when the harness runs, and not when the
//! harness compiles. A command can therefore come from a crate that the project
//! depends on, at the version that Cargo resolved.
//!
//! # Examples
//!
//! A harness runs the command line from its `main`:
//!
//! ```no_run
//! rakko_cli::builder().run();
//! ```

/// The command line that a harness builds and runs
mod builder;
/// The command that a harness writes for an activity that does not fit an
/// action
mod command;
/// The view of a command that hides its type
mod erased_command;
/// What a run reports about the action that it drove
mod report;
/// The root of the project that a run maintains
mod root;

/// The framework that a command receives its context from and writes through
///
/// A command that a harness writes names the context and the result of
/// Clawless in its signature. The harness reaches both through this
/// re-export, so it depends on no version of Clawless itself and takes the
/// one that this crate resolved.
pub use clawless;

pub use self::builder::{Builder, builder};
pub use self::command::Command;
pub use self::erased_command::ErasedCommand;
