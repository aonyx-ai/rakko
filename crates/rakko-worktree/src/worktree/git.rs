//! The commands of git that the crate starts
//!
//! Git is infrastructure of a machine and not a tool that an action wraps, so
//! the operating system finds it with the rules of the platform.
//!
//! Every command starts in a directory that the caller names, and it works on
//! the repository of that directory. That is not what a bare git does: a
//! process that runs inside a git hook, a rebase, or a bisect inherits
//! variables that name the repository, the working tree, and the index of the
//! run above it, and those variables beat both the working directory and the
//! `-C` option. A command of the crate would then read the index of that run
//! and write into its repository, which is the one thing that the crate
//! promises never to do.
//!
//! The variables therefore leave the environment of the command. Setting them
//! is no answer: git reads an empty `GIT_DIR` as a repository named by the
//! empty string, and a value that would be right names the repository that the
//! command has not found yet.
//!
//! What leaves is the whole prefix, and not a list of the names that redirect
//! a command today. A list is a promise that nobody renews: git adds a
//! variable, the list stays as it was, and the command that it redirects is
//! the one that writes. The prefix holds whatever git adds later.

use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

use kawauso_process::Invocation;

/// The program that manages the repository of a project
///
/// Git is not a tool that a project pins, so the name reaches whatever the
/// operating system finds by it.
const GIT: &str = "git";

/// The prefix of the names that git reads for itself
///
/// The underscore belongs to the prefix. Without it the crate would also drop
/// a variable such as `GITHUB_TOKEN`, which belongs to another program.
const PREFIX: &[u8] = b"GIT_";

/// Returns a command that runs git in the directory, without a runtime
///
/// A drop cannot wait for a future, and the crate that describes a command
/// runs one only as a future, so the removal of a copy describes its own
/// command and waits for it on the thread that dropped the value.
// worktree[impl project.environment]
pub(super) fn blocking_command(directory: &Path) -> Command {
    let mut command = Command::new(GIT);

    command.current_dir(directory).stdin(Stdio::null());

    for name in reserved(std::env::vars_os().map(|(name, _)| name)) {
        command.env_remove(name);
    }

    command
}

/// Returns the command that runs git in the directory
///
/// The caller adds the arguments of the operation that it wants, and then runs
/// the command. Nothing starts until it does, so a caller can name the command
/// in a log line or in an error first.
// worktree[impl project.environment]
pub(super) fn command(directory: &Path) -> Invocation {
    let names = reserved(std::env::vars_os().map(|(name, _)| name));

    names.fold(
        Invocation::new(GIT).in_directory(directory),
        Invocation::env_remove,
    )
}

/// Returns the names that git reads for itself
///
/// The check reads the bytes of a name, because a name of the environment need
/// not be valid text, and a name that the crate could not read would be a name
/// that it could not drop.
fn reserved(names: impl Iterator<Item = OsString>) -> impl Iterator<Item = OsString> {
    names.filter(|name| name.as_encoded_bytes().starts_with(PREFIX))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use kawauso_process::invocation::WorkingDirectory;

    use super::*;

    /// The directory that the commands of the tests run in
    const DIRECTORY: &str = "/home/otter/project";

    /// Returns the names that the crate would drop from a set of names
    fn dropped(names: &[&str]) -> Vec<String> {
        reserved(names.iter().map(OsString::from))
            .map(|name| name.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn blocking_command_runs_in_the_directory() {
        let command = blocking_command(Path::new(DIRECTORY));

        assert_eq!(command.get_current_dir(), Some(Path::new(DIRECTORY)));
    }

    #[test]
    fn command_runs_in_the_directory() {
        let invocation = command(Path::new(DIRECTORY));

        assert_eq!(
            invocation.working_directory().map(WorkingDirectory::get),
            Some(Path::new(DIRECTORY))
        );
    }

    // The variable that a hook exports is the one that would send every
    // command of the crate at the repository of the hook.
    // worktree[verify project.environment]
    #[test]
    fn reserved_names_a_variable_of_git() {
        assert_eq!(dropped(&["GIT_DIR"]), vec!["GIT_DIR"]);
    }

    // A name that nobody has heard of yet is what the prefix is for: it goes
    // without anybody adding it to a list.
    // worktree[verify project.environment]
    #[test]
    fn reserved_names_a_variable_that_git_has_yet_to_add() {
        assert_eq!(dropped(&["GIT_SOMETHING_NEW"]), vec!["GIT_SOMETHING_NEW"]);
    }

    // The underscore of the prefix keeps the variables of other programs, and
    // this is the name that a prefix without it would take away.
    // worktree[verify project.environment]
    #[test]
    fn reserved_keeps_a_variable_of_another_program() {
        assert_eq!(
            dropped(&["GITHUB_TOKEN", "PATH", "HOME"]),
            Vec::<String>::new()
        );
    }
}
