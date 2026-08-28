use std::path::PathBuf;

use kawauso_project::error::LoadProjectError;
use thiserror::Error;

/// An error that occurs when a run looks for the root of its project
///
/// A run reads and writes paths that start at the root of the project, so a
/// run that cannot name the root reports nothing instead of reporting against
/// a directory that it guessed.
#[derive(Debug, Error)]
pub(crate) enum DiscoverProjectRootError {
    /// No directory at or above the start of the search marks a project
    ///
    /// The user runs the harness outside a project, or inside a project that
    /// has not adopted Rakko. The message names the file that marks a root,
    /// because creating that file is what corrects the run.
    #[error(
        "no directory at or above `{start}` marks a project: create `{marker}` in the root of the project"
    )]
    UnmarkedProject {
        /// The directory that the search started in
        start: PathBuf,

        /// The entry that marks the root of a project
        marker: &'static str,
    },

    /// The search could not read the directories that it walks
    ///
    /// The start of the search does not exist, or the file system refused to
    /// report on a directory on the way up. Whether a project exists is
    /// unknown, so the run reports the refusal instead of a project.
    #[error("failed to search for the root of the project")]
    UnreadableStart {
        /// The cause of the failure
        source: LoadProjectError,
    },
}
