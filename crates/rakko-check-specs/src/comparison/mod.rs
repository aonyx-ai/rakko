//! The comparison that a pull request needs
//!
//! Tracey asks whether a staged change to the text of a requirement carries a
//! new version of that requirement, and it answers by comparing the index of a
//! repository with its HEAD. In the checkout of a contributor that comparison
//! is the right one. On a pull request it is not: a code host checks out a
//! merge commit of the branch and its base and stages nothing, so the index
//! and HEAD hold the same tree and the comparison finds no change at all.
//!
//! This module builds the comparison that such a run needs, in a copy of the
//! repository instead of in the checkout. The copy is a worktree at the first
//! parent of the merge commit, which is the base branch, and its index holds
//! the tree of the merge commit, which is the content of the pull request.
//! Tracey then compares the pull request with its base.
//!
//! Nothing in the checkout of the run changes. Moving the HEAD of the checkout
//! would build the same comparison in one command, and an action runs in
//! projects that this one never sees, where an environment that looks like a
//! pull request and a checkout that is not one would cost a contributor a
//! commit.
//!
//! Git is infrastructure of a machine and not a tool that an action wraps, so
//! the operating system finds it with the rules of the platform. A run inside
//! a git hook inherits variables that name the repository of the run above it,
//! and those variables beat the working directory of a command, so every
//! command here runs without them.

/// The error that leaves a run without the comparison it asked for
mod error;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rakko_action::ProjectRoot;
use rakko_tool::Invocation;
use tempfile::TempDir;

pub use self::error::PrepareComparisonError;
use crate::environment::inherited;

/// The program that manages the repository of a project
const GIT: &str = "git";

/// The revision of the merge commit that a code host checked out
const HEAD: &str = "HEAD";

/// The revision of the branch that a pull request asks to merge into
const FIRST_PARENT: &str = "HEAD^1";

/// The revision that a merge commit has and no other commit has
const SECOND_PARENT: &str = "HEAD^2";

/// A copy of the repository that compares a pull request with its base
///
/// The value owns the copy. A drop removes the worktree and its directory, so
/// an error and a panic leave nothing behind, and an interrupt that no
/// destructor survives leaves a directory that the operating system removes
/// and an entry that `git worktree prune` forgets.
#[derive(Debug)]
pub struct Comparison {
    /// The directory that holds the worktree
    directory: TempDir,

    /// The root of the repository that the worktree belongs to
    repository: PathBuf,
}

impl Comparison {
    /// Returns the directory that holds the comparison
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }

    /// Returns the comparison of a pull request with its base branch
    ///
    /// The caller states whether the environment of the run names the base
    /// branch of a pull request, and this method asks whether the checkout is
    /// a merge commit. Both have to hold: an environment variable alone
    /// describes a job and not a checkout, and a project that this action
    /// never saw could carry either one on its own.
    ///
    /// A run that is not a pull request answers `None`, and its caller
    /// compares the index of the checkout with its HEAD, which is what a
    /// contributor staged against what they last committed.
    ///
    /// # Errors
    ///
    /// Returns a [`PrepareComparisonError`] when git does not run, or when it
    /// refuses to build the copy.
    // checkspecs[impl version.base]
    // checkspecs[impl version.checkout]
    pub async fn prepare(
        root: &ProjectRoot,
        requested: bool,
    ) -> Result<Option<Self>, PrepareComparisonError> {
        if !requested {
            return Ok(None);
        }

        let repository = root.get().to_path_buf();

        if !resolves(&repository, SECOND_PARENT).await? {
            return Ok(None);
        }

        // The revisions are read before the copy exists, because HEAD names
        // the merge commit in the checkout and the base branch in the copy.
        let merge = capture(&repository, HEAD).await?;
        let base = capture(&repository, FIRST_PARENT).await?;

        let directory = tempfile::tempdir()
            .map_err(|source| PrepareComparisonError::UnavailableDirectory { source })?;

        // `worktree add` refuses a directory that exists, and the temporary
        // directory does. Git creates the last segment itself.
        let worktree = directory.path().join(WORKTREE);

        run(
            &repository,
            ["worktree", "add", "--detach", "--quiet"],
            [worktree.as_os_str(), base.as_ref()],
        )
        .await?;

        // The copy holds the base branch, and its index takes the tree of the
        // merge commit, which is what the pull request proposes.
        run(&worktree, ["read-tree"], [merge.as_ref()]).await?;

        Ok(Some(Self {
            directory,
            repository,
        }))
    }
}

/// The name of the directory that holds the copy
const WORKTREE: &str = "base";

impl Drop for Comparison {
    /// Removes the worktree that the value created
    ///
    /// A drop cannot wait for a future, so the removal describes its own
    /// command and waits for it on the thread that dropped the value. A
    /// failure stays quiet: the directory goes with the value either way, and
    /// `git worktree prune` forgets an entry whose directory is gone.
    fn drop(&mut self) {
        let mut command = Command::new(GIT);
        command
            .current_dir(&self.repository)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(self.directory.path().join(WORKTREE));

        for name in inherited() {
            command.env_remove(name);
        }

        let _ = command.status();
    }
}

/// Returns the commit that the revision names
///
/// # Errors
///
/// Returns a [`PrepareComparisonError`] when git does not run, or when it
/// resolves no commit for the revision.
async fn capture(directory: &Path, revision: &str) -> Result<OsString, PrepareComparisonError> {
    let execution = command(directory)
        .arg("rev-parse")
        .arg(revision)
        .run()
        .await
        .map_err(|source| PrepareComparisonError::GitUnavailable { source })?;

    if !execution.status().success() {
        return Err(PrepareComparisonError::RefusedCopy {
            details: execution.stderr().to_string_lossy().trim().to_owned(),
        });
    }

    Ok(OsString::from(
        execution.stdout().to_string_lossy().trim().to_owned(),
    ))
}

/// Returns whether the repository resolves the revision
///
/// # Errors
///
/// Returns a [`PrepareComparisonError`] when git does not run.
async fn resolves(directory: &Path, revision: &str) -> Result<bool, PrepareComparisonError> {
    let execution = command(directory)
        .arg("rev-parse")
        .arg("--verify")
        .arg("--quiet")
        .arg(revision)
        .run()
        .await
        .map_err(|source| PrepareComparisonError::GitUnavailable { source })?;

    Ok(execution.status().success())
}

/// Runs one command of git in the directory
///
/// # Errors
///
/// Returns a [`PrepareComparisonError`] when git does not run, or when it ends
/// without success.
async fn run<'a>(
    directory: &Path,
    arguments: impl IntoIterator<Item = &'a str>,
    paths: impl IntoIterator<Item = &'a std::ffi::OsStr>,
) -> Result<(), PrepareComparisonError> {
    let execution = command(directory)
        .args(arguments)
        .args(paths)
        .run()
        .await
        .map_err(|source| PrepareComparisonError::GitUnavailable { source })?;

    if execution.status().success() {
        return Ok(());
    }

    Err(PrepareComparisonError::RefusedCopy {
        details: execution.stderr().to_string_lossy().trim().to_owned(),
    })
}

/// Returns the command that runs git in the directory
fn command(directory: &Path) -> Invocation {
    inherited().fold(
        Invocation::new(GIT).in_directory(directory),
        Invocation::env_remove,
    )
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn command_runs_in_the_directory() {
        let invocation = command(Path::new("/home/otter/project"));

        assert_eq!(
            invocation
                .working_directory()
                .map(kawauso_process::invocation::WorkingDirectory::get),
            Some(Path::new("/home/otter/project"))
        );
    }
}
