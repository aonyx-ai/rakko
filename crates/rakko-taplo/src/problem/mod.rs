//! One problem that taplo reported about a file

mod detail;

use std::path::PathBuf;

use bon::bon;
use getset::Getters;
use rakko_action::{FilePath, ProjectRoot};

pub use self::detail::ProblemDetail;

/// One problem that taplo reported about a file
///
/// The path stands as taplo wrote it, which is absolute, because taplo starts
/// in the project root. A caller that reports the problem asks for the path
/// [relative][relative] to that root. That is the name that a reader, a
/// machine, and a code host all recognize.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use rakko_taplo::{TaploProblem, ProblemDetail};
///
/// let problem = TaploProblem::builder()
///     .path(PathBuf::from("/home/otter/project/a.toml"))
///     .detail(ProblemDetail::Unformatted)
///     .build();
///
/// assert_eq!(problem.detail(), &ProblemDetail::Unformatted);
/// ```
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

#[bon]
impl TaploProblem {
    /// Creates a problem from the path and the detail that taplo reported
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use rakko_taplo::{TaploProblem, ProblemDetail};
    ///
    /// let problem = TaploProblem::builder()
    ///     .path(PathBuf::from("/home/otter/project/a.toml"))
    ///     .detail(ProblemDetail::Unformatted)
    ///     .build();
    /// assert_eq!(problem.path(), &PathBuf::from("/home/otter/project/a.toml"));
    /// assert_eq!(problem.detail(), &ProblemDetail::Unformatted);
    /// assert_eq!(problem.detail(), &ProblemDetail::Unformatted);
    /// ```
    #[builder]
    pub fn new(path: PathBuf, detail: ProblemDetail) -> Self {
        Self { path, detail }
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. Taplo starts in
    /// the root and reports what it found below it. So a path that does not fit
    /// points at a report that the caller misread. The caller decides what to
    /// do about that.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_action::ProjectRoot;
    /// use rakko_taplo::{TaploProblem, ProblemDetail};
    /// use rakko_test_utils::path;
    ///
    /// let problem = TaploProblem::builder()
    ///     .path(path("/home/otter/project/a.toml"))
    ///     .detail(ProblemDetail::Unformatted)
    ///     .build();
    /// let root = ProjectRoot::new(path("/home/otter/project"));
    ///
    /// let relative = problem.relative_path(&root).unwrap();
    /// assert_eq!(relative.get().to_str(), Some("a.toml"));
    /// ```
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::within().path(&self.path).root(root).call()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko_test_utils::path;

    use super::*;

    /// Returns a problem about the path that a literal names on this platform
    fn problem(literal: &str) -> TaploProblem {
        TaploProblem::builder()
            .path(path(literal))
            .detail(ProblemDetail::Unformatted)
            .build()
    }

    /// Returns the root of the project that the tests place their problems in
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let problem = problem("/home/otter/elsewhere/a.toml");

        let relative = problem.relative_path(&root());

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_under_the_root_drops_the_root() {
        let problem = problem("/home/otter/project/sub/a.toml");

        let relative = problem.relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("sub/a.toml")).ok());
    }

    #[test]
    fn taplo_problem_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<TaploProblem>();
    }
}
