//! One rule that yamllint reported about a file
//!
//! A run of yamllint reports one problem for each rule that a file broke, and
//! this module holds one of them: where the rule was broken, how the project
//! weighed it, and what yamllint said about it.

mod level;

use std::path::PathBuf;

use bon::Builder;
use getset::{CopyGetters, Getters};
use rakko::{FilePath, ProjectRoot};

pub(crate) use self::level::ProblemLevel;

/// The prefix that a message gives the level of a problem
const LEVEL_OPEN: char = '[';

/// The text that closes the level of a problem in a message
const LEVEL_CLOSE: &str = "] ";

/// One rule that yamllint reported about a file
///
/// The path stands as yamllint wrote it. A run starts yamllint in the project
/// root and names the root as the place to look, so yamllint reports every
/// path relative to it. A caller that reports the problem asks for the
/// [relative][relative] path, which drops the `./` that yamllint puts in
/// front of it.
///
/// Every problem of yamllint names a line and a column, so a problem always
/// sits at a position and never on a line alone.
///
/// The description is the sentence that yamllint wrote for a reader, and it
/// holds the rule that the file broke. The level is how the project weighed
/// that rule. A reader of a finding therefore reads the answer of the tool,
/// and not one that this crate wrote about it.
///
/// [relative]: YamllintProblem::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub(crate) struct YamllintProblem {
    /// The path of the file, as yamllint wrote it
    #[getset(get = "pub(crate)")]
    path: PathBuf,

    /// The line that the rule was broken on, starting at 1
    #[getset(get_copy = "pub(crate)")]
    line: u32,

    /// The column that the rule points at, starting at 1
    #[getset(get_copy = "pub(crate)")]
    column: u32,

    /// How the project weighed the rule that the file broke
    #[getset(get_copy = "pub(crate)")]
    level: ProblemLevel,

    /// What yamllint said about the rule and the file
    #[getset(get = "pub(crate)")]
    description: String,
}

impl YamllintProblem {
    /// Returns the sentence that yamllint wrote about the problem
    ///
    /// The sentence holds the level and the description, in the order that
    /// yamllint writes them, so that a finding reads like the line that a
    /// contributor sees when they run yamllint themselves.
    pub(crate) fn message(&self) -> String {
        format!(
            "{LEVEL_OPEN}{}{LEVEL_CLOSE}{}",
            self.level, self.description
        )
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. A run names the
    /// root as the place to look, so a path that does not fit points at a
    /// report that the caller misread, and the caller decides what to do about
    /// that.
    pub(crate) fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
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
    fn problem(path: PathBuf) -> YamllintProblem {
        YamllintProblem::builder()
            .path(path)
            .line(1)
            .column(1)
            .level(ProblemLevel::Warning)
            .description("missing document start \"---\" (document-start)".to_owned())
            .build()
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn message_of_a_problem_reads_like_the_line_of_yamllint() {
        let message = problem(path("./notes.yaml")).message();

        assert_eq!(
            message,
            "[warning] missing document start \"---\" (document-start)"
        );
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let relative = problem(path("/home/otter/elsewhere/a.yaml")).relative_path(&root());

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let relative = problem(path("/home/otter/project/sub/a.yaml")).relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("sub/a.yaml")).ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_drops_the_prefix_of_yamllint() {
        let relative = problem(path("./sub/a.yaml")).relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("sub/a.yaml")).ok());
    }

    #[test]
    fn yamllint_problem_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<YamllintProblem>();
    }
}
