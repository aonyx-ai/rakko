//! The variables that a run keeps away from git
//!
//! Git reads a set of variables for itself, and they name the repository, the
//! working tree, and the index that a command works on. They beat both the
//! working directory of a command and its `-C` option, so a process that runs
//! inside a git hook, a rebase, or a bisect passes them to every program that
//! it starts, and a program that reads a repository then reads the one of the
//! run above it.
//!
//! This crate names the directory that each command works on, and it means the
//! directory that it named. The variables therefore leave the environment of
//! every command that reads a repository, whether the crate starts git itself
//! or starts a tool that does.
//!
//! What leaves is the whole prefix, and not a list of the names that redirect
//! a command today. A list is a promise that nobody renews: git adds a
//! variable, the list stays as it was, and the command that it redirects is
//! the one that writes.

use std::ffi::OsString;

/// The prefix of the names that git reads for itself
///
/// The underscore belongs to the prefix. Without it the crate would also drop
/// a variable such as `GITHUB_TOKEN`, which belongs to another program.
const PREFIX: &[u8] = b"GIT_";

/// Returns the names of this environment that git reads for itself
pub(crate) fn inherited() -> impl Iterator<Item = OsString> {
    reserved(std::env::vars_os().map(|(name, _)| name))
}

/// Returns the names that git reads for itself
///
/// The check reads the bytes of a name, because a name of the environment need
/// not be valid text, and a name that the crate could not read would be a name
/// that it could not drop.
pub(crate) fn reserved(names: impl Iterator<Item = OsString>) -> impl Iterator<Item = OsString> {
    names.filter(|name| name.as_encoded_bytes().starts_with(PREFIX))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns the names that the crate would drop from a set of names
    fn dropped(names: &[&str]) -> Vec<String> {
        reserved(names.iter().map(OsString::from))
            .map(|name| name.to_string_lossy().into_owned())
            .collect()
    }

    // The variable that a hook exports is the one that would send every
    // command of the crate at the repository of the hook.
    #[test]
    fn reserved_names_a_variable_of_git() {
        assert_eq!(dropped(&["GIT_DIR"]), vec!["GIT_DIR"]);
    }

    // A name that nobody has heard of yet is what the prefix is for.
    #[test]
    fn reserved_names_a_variable_that_git_has_yet_to_add() {
        assert_eq!(dropped(&["GIT_SOMETHING_NEW"]), vec!["GIT_SOMETHING_NEW"]);
    }

    // The underscore of the prefix keeps the variables of other programs.
    #[test]
    fn reserved_keeps_a_variable_of_another_program() {
        assert_eq!(dropped(&["GITHUB_TOKEN", "PATH"]), Vec::<String>::new());
    }
}
