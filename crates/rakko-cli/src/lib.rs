//! The command-line projection of the actions that a harness mounts
//!
//! A harness is the small binary that a project runs to maintain itself. It
//! mounts the actions that the project uses, alone or through a bundle, and
//! this crate turns them into a command-line interface: one command for each
//! action, the help text of each command, and the flags that every command
//! shares.
//!
//! A harness also writes commands of its own, for a maintenance activity of
//! the project that does not fit an action, and mounts them beside its
//! actions. Such a command implements the [`Command`] trait of this crate,
//! and it is an ordinary command of Clawless underneath: it receives the
//! context of Clawless and writes its output through it. This crate
//! re-exports Clawless, so a harness names what a command receives and
//! returns without a dependency of its own.
//!
//! Some of these activities run a list of actions, such as the actions that
//! guard a commit. A harness forms a [`Group`] for such an activity: a name
//! and a list of actions, its members. This crate gives the group a command
//! with the name of the group, and it derives the flags of that command from
//! the arguments that the members declare. It binds every member to the
//! values of a run, and it runs the members itself. A run of a group answers
//! as a run of one action does, with one summary and one exit code.
//!
//! A command that drives actions shows what they found with the [`Report`] of
//! this crate, so that its output takes the shape that a run of one action
//! has.
//!
//! A harness depends on this crate, and so does a crate that provides a
//! command for a harness to mount. An action, and a crate that provides a
//! group, depend on the contract crate alone, so the command-line framework
//! stays out of their build.
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
//!
//! [`Group`]: rakko_action::Group

mod builder;
mod chain;
mod command;
mod erased_command;
mod report;
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
pub use self::report::Report;
