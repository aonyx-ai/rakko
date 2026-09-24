/// What taplo reported about a file
mod detail;

use std::path::PathBuf;

use getset::Getters;
use rakko_action::{FilePath, ProjectRoot};

pub use self::detail::ProblemDetail;

/// One problem that taplo reported about a file
///
/// The path stands as taplo wrote it, which is absolute, because taplo
/// starts in the project root. A caller that reports the problem asks for
/// the path [relative][relative] to that root, which is the name that a
/// reader, a machine, and a code host all recognize.
///
/// [relative]: TaploProblem::relative_path
#[derive(Clone, Eq, PartialEq, Debug, Getters)]
pub struct TaploProblem {
    /// The path of the file, as taplo wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// What taplo reported about the file
    #[getset(get = "pub")]
    detail: ProblemDetail,
}

impl TaploProblem {
    /// Creates a problem from the path and the detail that taplo reported
    pub fn new(path: PathBuf, detail: ProblemDetail) -> Self {
        Self { path, detail }
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. Taplo starts
    /// in the root and reports what it found below it, so a path that does
    /// not fit points at a report that the caller misread, and the caller
    /// decides what to do about that.
    // taplo[impl path.relative]
    // taplo[impl path.foreign]
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::within(&self.path, root)
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns a problem about the given path
    fn problem(path: &str) -> TaploProblem {
        TaploProblem::new(PathBuf::from(path), ProblemDetail::Unformatted)
    }

    // taplo[verify path.foreign]
    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let problem = problem("/home/otter/elsewhere/a.toml");

        let path = problem.relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, None);
    }

    // taplo[verify path.relative]
    #[test]
    fn relative_path_under_the_root_drops_the_root() {
        let problem = problem("/home/otter/project/sub/a.toml");

        let path = problem.relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, FilePath::try_from("sub/a.toml").ok());
    }
}
