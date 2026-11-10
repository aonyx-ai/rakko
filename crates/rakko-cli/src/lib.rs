//! The command-line projection of the actions that a harness mounts
//!
//! A harness is the small binary that a project runs to maintain itself. It
//! mounts the actions it uses, alone or through a bundle. This crate turns
//! them into a command-line interface: one command per action, help text per
//! command, and flags all commands share. The harness gives each value to the
//! [`mount`] method of the builder as it is. The builder hides the type of
//! the value itself. So a harness that only mounts depends on this crate and on
//! the crates of the actions, bundles, and commands it mounts. A harness can
//! build a list of its own, such as a group list. Such a harness also depends
//! on the contract crate, [`rakko`], for the list type.
//!
//! A harness also writes commands of its own, for a maintenance activity of the
//! project that does not fit an action. It mounts them beside its actions. Such
//! a command implements the [`Command`] trait of this crate. It is an ordinary
//! command of Clawless underneath: it receives the context of Clawless and
//! writes its output through it. This crate re-exports Clawless, so a harness
//! names what a command receives and returns without a dependency of its own.
//!
//! Some of these activities run a list of actions, such as the actions that
//! guard a commit. A harness forms a [`Group`] for such an activity: a name and
//! a list of actions, its members. This crate gives the group a command with
//! the name of the group. It derives the flags of that command from the
//! arguments that the members declare. It binds every member to the values of a
//! run, and it runs the members itself. A run of a group answers as a run of
//! one action does, with one summary and one exit code.
//!
//! Every harness also gets the command `doctor`, and the harness writes no
//! code for it. An action declares the tools that it runs. The project
//! commits a mise file, `.config/mise/conf.d/rakko.toml`, with the exact
//! version of each tool that is not a toolchain. `doctor` reports these
//! problems:
//!
//! - The file does not match the tools of the mounted actions.
//! - A pin of the project overrides a declared tool.
//! - The project does not pin a toolchain that an action requires in the
//!   range of the action.
//!
//! The check needs no network. `doctor --fix` writes the file, and it
//! removes each pin that overrides a declared tool. Then it locks the tools
//! with `mise lock`, which needs the network. The repair does not change the
//! pin of a toolchain, because its version is a decision of the project. The
//! report says which pin to change. A project runs `doctor --fix` once when
//! it adopts Rakko, and again when an update of Rakko changes a tool.
//!
//! The command line runs the same check before it runs an action, a command
//! that the harness wrote, or a group. A problem stops the run, also in CI.
//! The error gives each problem and how to repair it: `doctor --fix`, or a
//! change to the pin of a toolchain. Only the help, the list of the commands,
//! and `doctor` run without the check. So a stale file cannot pass a pull
//! request without a sign. Each run of an action, a command, or a group
//! waits for the check before it starts. A harness whose actions declare no
//! tool reads one file and asks mise nothing.
//!
//! `doctor` sees the actions that the harness mounts and the members of its
//! groups. It does not see the actions that the harness gives to a command
//! that it mounts, such as the list of a pre-commit command. A harness that
//! gives an action only to a command also mounts that action, so that
//! `doctor` pins its tools.
//!
//! A command that drives actions shows what they found with the [`Report`] of
//! this crate. So its output takes the shape that a run of one action has.
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
//! fn main() -> std::process::ExitCode {
//!     rakko_cli::builder().run()
//! }
//! ```
//!
//! [`Group`]: rakko::Group
//! [`mount`]: Builder::mount

mod builder;
mod chain;
mod command;
mod doctor;
mod erased_command;
mod report;
mod root;
mod seal;

/// The framework that a command receives its context from and writes through
///
/// A command that a harness writes names the context and the result of
/// Clawless in its signature. The harness reaches both through this
/// re-export, so it depends on no version of Clawless itself and takes the
/// one that this crate resolved.
pub use clawless;

pub use self::builder::{Builder, Mount, builder};
pub use self::command::Command;
pub use self::erased_command::ErasedCommand;
pub use self::report::Report;
