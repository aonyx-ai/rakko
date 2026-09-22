//! One problem that oxfmt reported about a project
//!
//! A run of oxfmt names the files that it could not leave alone and the
//! things that it could not do, and this module holds one of them together
//! with what oxfmt knew about it.

use std::path::{Path, PathBuf};

use rakko_action::{FilePath, ProjectRoot};

/// One problem that oxfmt reported about a project
///
/// Oxfmt names a problem as precisely as the operation allows, and a variant
/// carries one of those levels. A list of the files that a rewrite would
/// change knows the file and nothing in it. A file that oxfmt read and could
/// not parse points at the character that broke it. A file that oxfmt could
/// not read at all, or could not write back, has a sentence and no place,
/// because oxfmt never got far enough to have one.
///
/// A path stands as oxfmt wrote it. A run starts oxfmt in the project root, so
/// oxfmt reports a path relative to it, and a caller that reports the problem
/// asks [`relative_path`] for the path that a finding names.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum OxfmtProblem {
    /// Oxfmt could not format the file, and it named the place that stopped it
    Invalid {
        /// The path of the file, as oxfmt wrote it
        path: PathBuf,

        /// The line that oxfmt pointed at, starting at 1
        line: u32,

        /// The column that oxfmt pointed at, starting at 1
        column: u32,

        /// What oxfmt said about the file
        message: String,
    },

    /// The file differs from what oxfmt would write
    ///
    /// Oxfmt names the file and nothing in it, so the problem has no position.
    Unformatted {
        /// The path of the file, as oxfmt wrote it
        path: PathBuf,
    },

    /// Oxfmt could not do something, and it named no place for the failure
    ///
    /// A file that oxfmt cannot open and a file that it cannot write back both
    /// end this way. Oxfmt writes the path of the file into the sentence
    /// instead of reporting it as a place, so the message carries it and the
    /// problem belongs to the project.
    Unplaced {
        /// What oxfmt said about the failure
        message: String,
    },
}

impl OxfmtProblem {
    /// Returns the path of the file that the problem is about
    ///
    /// Returns `None` for a problem that oxfmt placed in no file.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Invalid { path, .. } | Self::Unformatted { path } => Some(path),
            Self::Unplaced { .. } => None,
        }
    }
}

/// Returns the path of a file, relative to the project root
///
/// Returns `None` when the root does not contain the file. A run names the root
/// as the place to look, so a path that does not fit points at a report that
/// the caller misread, and the caller decides what to do about that.
pub fn relative_path(path: &Path, root: &ProjectRoot) -> Option<FilePath> {
    FilePath::try_from(strip(path, root)?).ok()
}

/// Returns the path without the prefix that names where oxfmt started
///
/// A run names the current directory as the place to look, so oxfmt reports a
/// path that is already relative to the project root.
///
/// A path that arrives absolute loses the project root instead. The root of a
/// context can name the same directory through a symbolic link, which is why
/// the canonical root is tried as well.
fn strip(path: &Path, root: &ProjectRoot) -> Option<PathBuf> {
    if path.is_relative() {
        return Some(path.to_path_buf());
    }

    if let Ok(stripped) = path.strip_prefix(root.get()) {
        return Some(stripped.to_path_buf());
    }

    let canonical = root.get().canonicalize().ok()?;

    path.strip_prefix(canonical).ok().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn path_of_a_failure_without_a_place_names_no_file() {
        let problem = OxfmtProblem::Unplaced {
            message: String::from("Failed to read file: /home/otter/project/src/index.ts"),
        };

        assert_eq!(problem.path(), None);
    }

    #[test]
    fn path_of_an_unformatted_file_stands_as_oxfmt_wrote_it() {
        let problem = OxfmtProblem::Unformatted {
            path: path("src/index.ts"),
        };

        assert_eq!(problem.path(), Some(path("src/index.ts").as_path()));
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let relative = relative_path(&path("/home/otter/elsewhere/index.ts"), &root());

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let relative = relative_path(&path("/home/otter/project/src/index.ts"), &root());

        assert_eq!(relative, FilePath::try_from(path("src/index.ts")).ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_names_the_file() {
        let relative = relative_path(&path("src/index.ts"), &root());

        assert_eq!(relative, FilePath::try_from(path("src/index.ts")).ok());
    }
}
