//! The commands of Git that the action starts
//!
//! Git is infrastructure of a machine and not a tool that an action wraps. So
//! the operating system finds it with the rules of the platform.
//!
//! A process that runs inside a Git hook, a rebase, or a bisect inherits
//! variables from the run above it, such as `GIT_DIR`. These variables beat
//! the working directory of a command. Every command that the action starts,
//! Otto included, therefore runs without the variables whose names start with
//! `GIT_`.

mod error;

use std::ffi::OsString;
use std::path::PathBuf;

use rakko::ProjectRoot;
use rakko_tool::{Execution, Invocation};

pub(crate) use self::error::CompareCommitsError;
use crate::change::{Change, ChangeKind};
use crate::comparison::Comparison;
use crate::pull_request::Commit;

/// The program that manages the repository of a project
const GIT: &str = "git";

/// The prefix of the names of the variables that Git reads for itself
///
/// The underscore belongs to the prefix, so that a variable such as
/// `GITHUB_EVENT_PATH` stays.
const PREFIX: &[u8] = b"GIT_";

/// The status of an added file in the list of `git diff --name-status`
const ADDED: &[u8] = b"A";

/// Returns what a pull request changes between its base and its head
///
/// Git compares the head with the last commit that it shares with the base,
/// as `git diff <base>...<head>` does. So a commit that reached the base
/// branch after the pull request started is no change of the pull request.
/// The comparison spans every commit of the pull request, so a fixup commit
/// on top of a changeset does not hide that changeset. Git lists a renamed
/// file as a deletion and an addition. So a changeset that moves into the
/// directory of a unit counts as added there.
///
/// A path that is no valid UTF-8 replaces each invalid byte with U+FFFD. The
/// roots of the units come from JSON, which is UTF-8, so the replacement
/// never moves a file into another unit.
///
/// # Errors
///
/// Returns [`GitUnavailable`][unavailable] when Git does not run.
/// Returns [`UnlocatedProject`][unlocated] when Git finds no repository at the
/// root of the project, and [`IncomparableCommits`][incomparable] when it
/// cannot compare the two commits.
///
/// [incomparable]: CompareCommitsError::IncomparableCommits
/// [unavailable]: CompareCommitsError::GitUnavailable
/// [unlocated]: CompareCommitsError::UnlocatedProject
pub(crate) async fn compare(
    root: &ProjectRoot,
    base: &Commit,
    head: &Commit,
) -> Result<Comparison, CompareCommitsError> {
    let located = run(command(root).arg("rev-parse").arg("--show-prefix")).await?;
    if !located.status().success() {
        return Err(CompareCommitsError::UnlocatedProject {
            details: located.stderr().to_string_lossy().trim().to_owned(),
        });
    }

    let compared = run(command(root)
        .arg("diff")
        .arg("--name-status")
        .arg("--no-renames")
        .arg("-z")
        .arg(format!("{base}...{head}")))
    .await?;
    if !compared.status().success() {
        return Err(CompareCommitsError::IncomparableCommits {
            base: base.to_string(),
            head: head.to_string(),
            details: compared.stderr().to_string_lossy().trim().to_owned(),
        });
    }

    Ok(Comparison::builder()
        .prefix(located.stdout().to_string_lossy().trim().to_owned())
        .changes(changes(compared.stdout().get()))
        .build())
}

/// Returns the command without the variables that Git reads for itself
pub(crate) fn without_variables(invocation: Invocation) -> Invocation {
    reserved(std::env::vars_os().map(|(name, _)| name)).fold(invocation, Invocation::env_remove)
}

/// Returns the changes of the list that `git diff --name-status -z` writes
///
/// The list holds a status and a path for each file, each followed by a zero
/// byte. With `--no-renames`, no entry holds a second path.
fn changes(list: &[u8]) -> Vec<Change> {
    let mut fields = list.split(|byte| *byte == 0);
    let mut changes = Vec::new();

    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        let kind = if status == ADDED {
            ChangeKind::Added
        } else {
            ChangeKind::Other
        };

        changes.push(
            Change::builder()
                .path(PathBuf::from(String::from_utf8_lossy(path).into_owned()))
                .kind(kind)
                .build(),
        );
    }

    changes
}

/// Returns a command that runs Git in the root of the project
fn command(root: &ProjectRoot) -> Invocation {
    without_variables(Invocation::new(GIT).in_directory(root.get()))
}

/// Returns the names that Git reads for itself
///
/// The check reads the bytes of a name, because a name of the environment
/// need not be valid text.
fn reserved(names: impl Iterator<Item = OsString>) -> impl Iterator<Item = OsString> {
    names.filter(|name| name.as_encoded_bytes().starts_with(PREFIX))
}

/// Runs a command of Git and returns what it did
///
/// # Errors
///
/// Returns [`GitUnavailable`][unavailable] when Git does not run.
///
/// [unavailable]: CompareCommitsError::GitUnavailable
async fn run(invocation: Invocation) -> Result<Execution, CompareCommitsError> {
    invocation
        .run()
        .await
        .map_err(|source| CompareCommitsError::GitUnavailable { source })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "An assertion in a test panics by design, and the helpers of this module exist only for tests."
    )]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns the names that the action would drop from a set of names
    fn dropped(names: &[&str]) -> Vec<String> {
        reserved(names.iter().map(OsString::from))
            .map(|name| name.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn changes_of_a_deletion_are_no_addition() {
        let changes = changes(b"D\0crates/alpha/.changeset/add-it.md\0");

        assert_eq!(changes[0].kind(), ChangeKind::Other);
    }

    #[test]
    fn changes_of_a_list_read_the_status_and_the_path_of_each_file() {
        let changes =
            changes(b"A\0crates/alpha/.changeset/add-it.md\0M\0crates/alpha/src/lib.rs\0");

        assert_eq!(
            changes,
            [
                Change::builder()
                    .path("crates/alpha/.changeset/add-it.md")
                    .kind(ChangeKind::Added)
                    .build(),
                Change::builder()
                    .path("crates/alpha/src/lib.rs")
                    .kind(ChangeKind::Other)
                    .build(),
            ]
        );
    }

    #[test]
    fn changes_of_an_empty_list_are_none() {
        let changes = changes(b"");

        assert!(changes.is_empty());
    }

    #[tokio::test]
    async fn compare_outside_a_repository_fails() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let root = ProjectRoot::new(directory.path().to_path_buf());

        let result = compare(&root, &Commit::new("main"), &Commit::new("HEAD")).await;

        assert!(
            matches!(result, Err(CompareCommitsError::UnlocatedProject { .. })),
            "expected a project outside a repository, got {result:?}"
        );
    }

    #[test]
    fn reserved_drops_a_variable_of_git() {
        assert_eq!(dropped(&["GIT_DIR"]), vec!["GIT_DIR"]);
    }

    #[test]
    fn reserved_keeps_a_variable_of_github() {
        assert!(dropped(&["GITHUB_EVENT_PATH", "PATH"]).is_empty());
    }
}
