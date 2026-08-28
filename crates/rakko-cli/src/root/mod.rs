//! The root of the project that a run maintains
//!
//! Every action reads the root of its project from the context of a run. The
//! layout derives the directories of a project from that root, and a finding
//! reports its location relative to it, so the root decides what an action
//! may touch, where it writes, and what its output says.
//!
//! A project marks its root with one file. The search tests whether the entry
//! exists and never reads it, so a project that keeps nothing in the file
//! still names its root.

/// What a run reports when it cannot name the root of its project
mod error;

use std::path::{Path, PathBuf};

pub(crate) use error::DiscoverProjectRootError;
use kawauso_project::error::{DiscoverProjectError, LoadProjectError};
use kawauso_project::{Project, Search};
use rakko_action::ProjectRoot;

/// The name that identifies Rakko to the search
///
/// The search derives the conventional location of a configuration file from
/// this name. A run reads no configuration today, so the name reaches nothing
/// but a path that stays unused.
const APPLICATION: &str = "rakko";

/// The entry that marks the root of a project
///
/// The path is relative to a directory of the walk. A project writes the file
/// when it adopts Rakko, and Rakko never reads it: the presence of the entry
/// is the whole test.
const MARKER: &str = ".config/rakko.toml";

/// Returns the root of the project that holds the given directory
///
/// The search starts at `start` and walks up to the root of the file system.
/// The first directory that holds the marker is the project, so a run reports
/// the same paths from every directory of a project.
///
/// The search reads no file. A project whose marker is empty is a project.
///
/// # Errors
///
/// Returns an error when no directory at or above `start` holds the marker,
/// and when the file system refuses to report on a directory of the walk. A
/// run stops in both cases, because an action that receives a guessed root
/// reads the wrong files and reports paths that mean nothing.
// cli[impl root.marker]
// cli[impl root.unmarked]
pub(crate) fn discover(start: &Path) -> Result<ProjectRoot, DiscoverProjectRootError> {
    let search = Search::start(start).marker(MARKER);

    let project: Project = Project::builder()
        .application(APPLICATION)
        .without_configuration()
        .load(&search)
        .map_err(|source| classify(source, start))?;

    Ok(ProjectRoot::new(project.root().get().to_path_buf()))
}

/// Returns the error that a run reports for a search that found no project
///
/// A walk that tested every directory and matched nothing is the ordinary
/// failure, and its message names the file that a project creates to correct
/// it. Every other failure leaves open whether a project exists, so it keeps
/// the cause that the search reported.
fn classify(source: LoadProjectError, start: &Path) -> DiscoverProjectRootError {
    match source {
        LoadProjectError::UndiscoverableProject {
            source: DiscoverProjectError::MissingProject { .. },
            ..
        } => DiscoverProjectRootError::UnmarkedProject {
            start: PathBuf::from(start),
            marker: MARKER,
        },
        source => DiscoverProjectRootError::UnreadableStart { source },
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use tempfile::TempDir;

    use super::*;

    /// Creates a project whose root holds the marker, and returns it
    fn project() -> TempDir {
        let project = tempfile::tempdir().expect("the test creates a temporary directory");
        let configuration = project.path().join(".config");

        std::fs::create_dir_all(&configuration).expect("the test creates the configuration");
        std::fs::write(configuration.join("rakko.toml"), "").expect("the test creates the marker");

        project
    }

    // cli[verify root.marker]
    #[test]
    fn discover_from_a_subdirectory_reports_the_marked_directory() {
        let project = project();
        let subdirectory = project.path().join("crates").join("example");
        std::fs::create_dir_all(&subdirectory).expect("the test creates a subdirectory");

        let root = discover(&subdirectory).expect("expected the search to find the project");

        assert_eq!(
            root.get().canonicalize().ok(),
            project.path().canonicalize().ok()
        );
    }

    // cli[verify root.unmarked]
    #[test]
    fn discover_without_a_marker_reports_the_file_that_marks_a_root() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let Err(error) = discover(directory.path()) else {
            panic!("expected the search to report an error");
        };

        assert!(error.to_string().contains(MARKER));
    }
}
