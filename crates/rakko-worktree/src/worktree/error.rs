use std::io::Error as IoError;
use std::path::PathBuf;

use kawauso_process::error::RunCommandError;
use rakko_action::ProjectRoot;
use thiserror::Error;

/// An error that occurs when the crate stands up a copy of a project
///
/// The variants separate what a caller can do about the failure. A project
/// without a repository and a project below the top level are answers about
/// the project, and the message names the directory that has to change. Git
/// that does not run is a broken environment, and every copy of every project
/// fails with it. The rest are failures of one step, and the message carries
/// what git or the operating system said about it.
///
/// No variant leaves a copy behind. A failure that happens after git created
/// the worktree removes the worktree first, so a caller that reads one of
/// these errors has nothing to clean up.
#[derive(Debug, Error)]
pub enum CreateWorktreeError {
    /// Git did not run
    ///
    /// No program answers to the name `git`, or the operating system refused
    /// to start it. Git is infrastructure of the machine, so the failure
    /// belongs to the environment of the run and not to the project.
    #[error("failed to ask git about `{project}`")]
    GitUnavailable {
        /// The root of the project that the crate asked about
        project: ProjectRoot,

        /// The cause of the failure
        source: RunCommandError,
    },

    /// The project is not in a git repository
    ///
    /// A project marks its root with a file of its own and needs no
    /// repository otherwise, so a project can be well formed and still have
    /// no revision that a copy could check out.
    #[error("`{project}` is not in a git repository: {details}")]
    MissingRepository {
        /// The root of the project that the crate asked about
        project: ProjectRoot,

        /// What git said about the project
        details: String,
    },

    /// The project root is not the top level of its repository
    ///
    /// A copy of a project below the top level would hold the whole
    /// repository and would sync paths that belong to no project. The crate
    /// stops instead, and the message names the directory that a project has
    /// to move its marker to.
    #[error("`{project}` is not the top level of its git repository, which is `{top_level}`")]
    NestedProject {
        /// The root of the project that the crate asked about
        project: ProjectRoot,

        /// The top level of the repository that holds the project
        top_level: PathBuf,
    },

    /// The temporary directory of the system did not take a new directory
    ///
    /// The copy lives outside the project, so it needs a directory that the
    /// system hands out. A system that hands out none has no room for a copy.
    #[error("failed to create a directory for the copy of `{project}`")]
    TemporaryDirectoryUnavailable {
        /// The root of the project that the crate copies
        project: ProjectRoot,

        /// The cause of the failure
        source: IoError,
    },

    /// Git created no worktree
    ///
    /// Git ran and ended without success. A repository without a commit has
    /// no HEAD to check out, and a repository that the run cannot write to
    /// takes no new worktree, so the details carry what git wrote about it.
    #[error("git created no worktree of `{project}`: {details}")]
    WorktreeUnavailable {
        /// The root of the project that the crate copies
        project: ProjectRoot,

        /// What git said about the worktree
        details: String,
    },

    /// The crate cannot read what changed in the project
    ///
    /// Git reported no status of the project, or it reported one that the
    /// crate cannot read. The sync names its paths from that report, so a
    /// report that the crate skipped would leave the copy at the commit while
    /// the caller believes it holds the tree of the contributor.
    #[error("failed to read what changed in `{project}`: {details}")]
    UnreadableStatus {
        /// The root of the project that the crate copies
        project: ProjectRoot,

        /// What the crate observed about the report
        details: String,
    },

    /// A changed path did not reach the copy
    ///
    /// The operating system refused to copy the file into the worktree, or to
    /// remove it from there. The copy then differs from the project at that
    /// path, and a job that ran in it would answer for a tree that nobody
    /// has.
    #[error("failed to sync `{path}` into the copy of the project")]
    UnsyncedPath {
        /// The path that the crate synced, relative to the project root
        path: PathBuf,

        /// The cause of the failure
        source: IoError,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read. This test holds the error
    // to the auto traits that make this possible, because a field of a later
    // version could take them away without a word from the compiler.
    #[test]
    fn create_worktree_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<CreateWorktreeError>();
    }
}
