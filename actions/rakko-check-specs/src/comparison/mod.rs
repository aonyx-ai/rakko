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

use rakko::ProjectRoot;
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
pub(crate) struct Comparison {
    /// The directory that holds the worktree
    directory: TempDir,

    /// The root of the repository that the worktree belongs to
    repository: PathBuf,
}

impl Comparison {
    /// Returns the directory that holds the comparison
    pub(crate) fn directory(&self) -> &Path {
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
    pub(crate) async fn prepare(
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

    /// A specification with one requirement
    const SPECIFICATION: &str = "# Probe\n\nprobe[alpha]\nThe thing MUST do alpha.\n";

    /// A specification whose requirement says something else, at the same
    /// version
    const CHANGED: &str = "# Probe\n\nprobe[alpha]\nThe thing MUST do alpha and gamma.\n";

    /// A project that a test builds in a temporary directory
    struct Project {
        /// The directory that holds the project
        directory: TempDir,
    }

    impl Project {
        /// Returns a project whose checkout is a merge commit of a pull
        /// request
        ///
        /// The branch changes the text of a requirement and bumps no version,
        /// which is what the comparison of a pull request has to find. The
        /// comparison only runs git, so the project holds the specification
        /// and nothing else.
        fn merged() -> Self {
            let project = Self {
                directory: tempfile::tempdir().unwrap(),
            };
            project.write(SPECIFICATION);

            project.git(&["init", "--quiet", "--initial-branch", "main", "."]);
            project.git(&["add", "-A"]);
            project.git(&["commit", "--quiet", "-m", "base"]);
            project.git(&["checkout", "--quiet", "-b", "feature"]);
            project.write(CHANGED);
            project.git(&["add", "-A"]);
            project.git(&["commit", "--quiet", "-m", "change the requirement"]);
            project.git(&["checkout", "--quiet", "main"]);
            project.git(&["merge", "--quiet", "--no-ff", "feature", "-m", "Merge"]);
            let merge = project.git(&["rev-parse", "HEAD"]);
            project.git(&["checkout", "--quiet", "--detach", &merge]);

            project
        }

        /// Returns the canonical root of the project
        ///
        /// The root is canonical, so the paths that git reports do not depend
        /// on the symbolic links of the temporary directory.
        fn root(&self) -> ProjectRoot {
            ProjectRoot::canonical(self.directory.path()).unwrap()
        }

        /// Writes the specification of the project
        fn write(&self, content: &str) {
            std::fs::write(self.directory.path().join("SPECIFICATION.md"), content).unwrap();
        }

        /// Runs one command of git in the project
        fn git(&self, arguments: &[&str]) -> String {
            git(self.directory.path(), arguments)
        }
    }

    /// Runs one command of git in the directory
    fn git(directory: &Path, arguments: &[&str]) -> String {
        // The commits of a fixture are thrown away with its directory, and
        // the signing key of a contributor is not the test's to ask for.
        let mut command = Command::new(GIT);
        command
            .arg("-c")
            .arg("commit.gpgsign=false")
            .args(arguments)
            .current_dir(directory);

        // A test that runs inside a git hook inherits the variables that name
        // the repository of the hook, and those beat the working directory, so
        // the fixture would act on the checkout of the contributor. The
        // identity below is set afterwards, because it carries the prefix too.
        for (name, _) in std::env::vars_os() {
            if name.as_encoded_bytes().starts_with(b"GIT_") {
                command.env_remove(name);
            }
        }

        let output = command
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "expected git to run {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

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

    #[tokio::test]
    async fn comparison_leaves_the_commit_of_the_checkout_alone() {
        let project = Project::merged();
        let before = project.git(&["rev-parse", "HEAD"]);

        let comparison = Comparison::prepare(&project.root(), true).await.unwrap();
        drop(comparison);

        assert_eq!(project.git(&["rev-parse", "HEAD"]), before);
    }

    #[tokio::test]
    async fn comparison_leaves_the_index_of_the_checkout_alone() {
        let project = Project::merged();

        let comparison = Comparison::prepare(&project.root(), true).await.unwrap();
        drop(comparison);

        assert_eq!(project.git(&["diff", "--cached", "--name-only"]), "");
    }

    // A code host checks out a merge commit of the branch and its base, and
    // stages nothing, so a comparison of that index with that HEAD finds no
    // change at all.
    #[tokio::test]
    async fn comparison_of_a_merge_commit_stages_the_pull_request() {
        let project = Project::merged();

        let comparison = Comparison::prepare(&project.root(), true)
            .await
            .unwrap()
            .unwrap();

        let staged = git(
            &comparison.directory().join(WORKTREE),
            &["diff", "--cached", "--name-only"],
        );
        assert_eq!(staged, "SPECIFICATION.md");
    }

    // A run that no environment calls a pull request compares the checkout
    // itself.
    #[tokio::test]
    async fn comparison_that_nothing_asked_for_is_none() {
        let project = Project::merged();

        let comparison = Comparison::prepare(&project.root(), false).await.unwrap();

        assert!(comparison.is_none(), "expected no comparison");
    }
}
