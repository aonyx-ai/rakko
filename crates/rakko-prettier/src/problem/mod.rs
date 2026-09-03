//! One problem that prettier reported about a file
//!
//! A run of prettier names the files that it could not leave alone, and this
//! module holds one of them together with what prettier knew about it.

/// What prettier reported about a file
mod detail;

use std::path::{Path, PathBuf};

use getset::Getters;
use rakko_action::{FilePath, ProjectRoot};

pub use self::detail::ProblemDetail;

/// One problem that prettier reported about a file
///
/// The path stands as prettier wrote it. Prettier starts in the project root
/// and names a file relative to it, and a caller that reports the problem
/// asks for the [relative][relative] path, which strips a root that arrived
/// in front of it anyway.
///
/// [relative]: PrettierProblem::relative_path
#[derive(Clone, Eq, PartialEq, Debug, Getters)]
pub struct PrettierProblem {
    /// The path of the file, as prettier wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// What prettier reported about the file
    #[getset(get = "pub")]
    detail: ProblemDetail,
}

impl PrettierProblem {
    /// Creates a problem from the path and the detail that prettier reported
    pub fn new(path: PathBuf, detail: ProblemDetail) -> Self {
        Self { path, detail }
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. Prettier
    /// starts in the root and reports what it found below it, so a path that
    /// does not fit points at a report that the caller misread, and the
    /// caller decides what to do about that.
    // prettier[impl path.foreign]
    // prettier[impl path.relative]
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::try_from(strip(&self.path, root)?).ok()
    }
}

/// Returns the path without the project root that prefixes it
///
/// A path that is already relative is the answer itself, because prettier
/// starts in the root and names its files from there. A path that arrives
/// absolute loses the root, and the root of a context can name the same
/// directory through a symbolic link, which is why the canonical root is
/// tried as well.
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

    use super::*;

    /// Returns a problem about the given path
    fn problem(path: &str) -> PrettierProblem {
        PrettierProblem::new(PathBuf::from(path), ProblemDetail::Unformatted)
    }

    // prettier[verify path.foreign]
    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let problem = problem("/home/otter/elsewhere/a.md");

        let path = problem.relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, None);
    }

    // prettier[verify path.relative]
    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let problem = problem("/home/otter/project/sub/a.md");

        let path = problem.relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, FilePath::try_from("sub/a.md").ok());
    }

    // prettier[verify path.relative]
    #[test]
    fn relative_path_that_arrived_relative_stands_as_prettier_wrote_it() {
        let problem = problem("sub/a.md");

        let path = problem.relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, FilePath::try_from("sub/a.md").ok());
    }
}
