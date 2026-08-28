//! The command-line projection of the actions that a harness mounts
//!
//! A harness is the small binary that a project runs to maintain itself. It
//! mounts the actions that the project uses, and this crate turns them into a
//! command-line interface: one command for each action, the help text of each
//! command, and the flags that every command shares.
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
/// What a run reports about the action that it drove
mod report;

pub use builder::{Builder, builder};
