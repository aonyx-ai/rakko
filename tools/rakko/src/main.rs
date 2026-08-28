//! The maintenance commands of this repository
//!
//! This binary is the harness of the Rakko repository: the one place that
//! states which maintenance actions run here. It mounts the actions that this
//! repository uses, and the command line that it builds turns each of them
//! into a command.
//!
//! Run it with `mise run rakko`, or with `rakko` where the environment
//! supplies the shortcut.

/// Builds the command line of this repository and runs it
///
/// The call ends the process itself, so a harness stays a `main` that names
/// what the repository mounts and returns nothing.
fn main() {
    rakko_cli::builder().run();
}
