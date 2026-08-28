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
//!
//! A root is absolute, holds no `.` or `..` component, and has its symbolic
//! links resolved, whether the search found it or the user named it. An
//! action joins its own paths onto the root, so a root that moves with the
//! working directory would move every path that an action reports.

/// What a run reports when it cannot name the root of its project
mod error;

use std::path::{Path, PathBuf};

use kawauso_project::error::{DiscoverProjectError, LoadProjectError};
use kawauso_project::{Project, Search};
use rakko_action::ProjectRoot;

pub(crate) use self::error::ResolveProjectRootError;

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

/// Returns the root of the project that a run maintains
///
/// A user who names a root gets that root, and every other run searches for
/// one from `start` upwards. The user knows what the search would look for,
/// so a named root answers for a checkout that Rakko does not expect.
///
/// # Errors
///
/// Returns an error when the user names a directory that the file system does
/// not answer for, when no directory at or above `start` holds the marker, and
/// when the file system refuses to report on a directory of the walk. A run
/// stops in every case, because an action that receives a guessed root reads
/// the wrong files and reports paths that mean nothing.
// cli[impl root.named]
pub(crate) fn resolve(
    named: Option<PathBuf>,
    start: &Path,
) -> Result<ProjectRoot, ResolveProjectRootError> {
    match named {
        Some(root) => canonical(root),
        None => discover(start),
    }
}

/// Returns the root that the user named, in the form that an action reads
///
/// The file system resolves the path, so a relative argument and a path
/// through a symbolic link both reach an action as the directory that they
/// name. A path that no directory answers for is a mistake in an argument,
/// and the run reports it instead of a project.
///
/// # Errors
///
/// Returns an error when the file system does not answer for the path.
fn canonical(root: PathBuf) -> Result<ProjectRoot, ResolveProjectRootError> {
    let resolved = root
        .canonicalize()
        .map_err(|source| ResolveProjectRootError::UnreadableRoot { root, source })?;

    Ok(ProjectRoot::new(resolved))
}

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
/// and when the file system refuses to report on a directory of the walk.
// cli[impl root.marker]
// cli[impl root.unmarked]
fn discover(start: &Path) -> Result<ProjectRoot, ResolveProjectRootError> {
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
fn classify(source: LoadProjectError, start: &Path) -> ResolveProjectRootError {
    match source {
        LoadProjectError::UndiscoverableProject {
            source: DiscoverProjectError::MissingProject { .. },
            ..
        } => ResolveProjectRootError::UnmarkedProject {
            start: PathBuf::from(start),
            marker: MARKER,
        },
        source => ResolveProjectRootError::UnreadableStart { source },
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

    // cli[verify root.named]
    #[test]
    fn resolve_with_a_named_root_reports_it_without_a_relative_component() {
        let project = project();
        let named = project.path().join("crates").join("..");
        std::fs::create_dir_all(project.path().join("crates"))
            .expect("the test creates a subdirectory");

        let root = resolve(Some(named), project.path()).expect("expected the run to take the root");

        assert_eq!(
            root.get().canonicalize().ok(),
            project.path().canonicalize().ok()
        );
    }

    // cli[verify root.named]
    #[test]
    fn resolve_with_a_named_root_that_no_directory_answers_reports_the_path() {
        let missing = PathBuf::from("/rakko/does/not/live/here");

        let Err(error) = resolve(Some(missing.clone()), Path::new(".")) else {
            panic!("expected the run to report an error");
        };

        assert!(error.to_string().contains("/rakko/does/not/live/here"));
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
