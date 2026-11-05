//! One problem that prettier reported about a file
//!
//! A run of prettier names the files that it could not leave alone, and this
//! module holds one of them together with what prettier knew about it.

mod detail;

use std::path::PathBuf;

use bon::bon;
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
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use rakko_prettier::problem::ProblemDetail;
/// use rakko_prettier::PrettierProblem;
///
/// let problem = PrettierProblem::builder()
///     .path(PathBuf::from("src/main.rs"))
///     .detail(ProblemDetail::Unformatted)
///     .build();
///
/// assert_eq!(problem.path(), &PathBuf::from("src/main.rs"));
/// ```
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

#[bon]
impl PrettierProblem {
    /// Creates a problem from the path and the detail that prettier reported
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use rakko_prettier::problem::ProblemDetail;
    /// use rakko_prettier::PrettierProblem;
    ///
    /// let problem = PrettierProblem::builder()
    ///     .path(PathBuf::from("src/main.rs"))
    ///     .detail(ProblemDetail::Unformatted)
    ///     .build();
    ///
    /// assert_eq!(problem.detail(), &ProblemDetail::Unformatted);
    /// ```
    #[builder]
    pub fn new(path: PathBuf, detail: ProblemDetail) -> Self {
        Self { path, detail }
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. Prettier
    /// starts in the root and reports what it found below it, so a path that
    /// does not fit points at a report that the caller misread, and the
    /// caller decides what to do about that.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use rakko_action::ProjectRoot;
    /// use rakko_prettier::problem::ProblemDetail;
    /// use rakko_prettier::PrettierProblem;
    ///
    /// let problem = PrettierProblem::builder()
    ///     .path(PathBuf::from("src/main.rs"))
    ///     .detail(ProblemDetail::Unformatted)
    ///     .build();
    ///
    /// let root = ProjectRoot::new(PathBuf::from("/home/otter/project"));
    /// let relative = problem.relative_path(&root).unwrap();
    ///
    /// assert_eq!(relative.get().to_str(), Some("src/main.rs"));
    /// ```
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::within().path(&self.path).root(root).call()
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;

    /// Returns a problem about the given path
    fn problem(path: PathBuf) -> PrettierProblem {
        PrettierProblem::builder()
            .path(path)
            .detail(ProblemDetail::Unformatted)
            .build()
    }

    #[test]
    fn prettier_problem_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<PrettierProblem>();
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let problem = problem(path("/home/otter/elsewhere/a.md"));

        let relative = problem.relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let problem = problem(path("/home/otter/project/sub/a.md"));

        let relative = problem.relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, FilePath::try_from(path("sub/a.md")).ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_stands_as_prettier_wrote_it() {
        let problem = problem(path("sub/a.md"));

        let relative = problem.relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, FilePath::try_from(path("sub/a.md")).ok());
    }
}
