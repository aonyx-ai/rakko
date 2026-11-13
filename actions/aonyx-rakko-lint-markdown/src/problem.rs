//! One rule that markdownlint reported about a file
//!
//! A run of markdownlint reports one result for each rule that a file broke.
//! This module holds one of them: where the rule was broken, and what
//! markdownlint said about it.

use std::path::PathBuf;

use bon::Builder;
use getset::{CopyGetters, Getters};
use rakko::{FilePath, ProjectRoot};

/// One rule that markdownlint reported about a file
///
/// The path stands as markdownlint wrote it.
/// Markdownlint starts in the project root and names a file relative to it.
/// A caller that reports the problem asks for the [relative][relative] path.
/// This path strips a root that arrived in front of it anyway.
///
/// Every result of markdownlint names a line, so a problem always has one.
/// A rule that can point at a character in that line names a column as well.
/// One that speaks about the whole line names none.
///
/// The message is the sentence markdownlint would have written for a reader.
/// It includes the rule, its aliases, what the rule is about, and what it
/// expected here.
/// A reader of a finding therefore reads the answer of the tool, and not one
/// that this crate wrote about it.
///
/// [relative]: MarkdownlintProblem::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub(crate) struct MarkdownlintProblem {
    /// The path of the file, as markdownlint wrote it
    #[getset(get = "pub(crate)")]
    path: PathBuf,

    /// The line that the rule was broken on, starting at 1
    #[getset(get_copy = "pub(crate)")]
    line: u32,

    /// The column that the rule points at, starting at 1
    ///
    /// A rule that speaks about the whole line names no column.
    #[getset(get_copy = "pub(crate)")]
    column: Option<u32>,

    /// What markdownlint said about the rule and the file
    #[getset(get = "pub(crate)")]
    message: String,
}

impl MarkdownlintProblem {
    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. Markdownlint
    /// starts in the root and names what it found below it. So a path that does
    /// not fit points at a report that the caller misread. The caller decides
    /// what to do about that.
    pub(crate) fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
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

    /// Returns a problem about the given path
    fn problem(path: PathBuf) -> MarkdownlintProblem {
        MarkdownlintProblem::builder()
            .path(path)
            .line(1)
            .message("MD041/first-line-heading".to_owned())
            .build()
    }

    #[test]
    fn markdownlint_problem_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<MarkdownlintProblem>();
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let relative = problem(path("/home/otter/elsewhere/a.md"))
            .relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let relative = problem(path("/home/otter/project/sub/a.md"))
            .relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, FilePath::try_from(path("sub/a.md")).ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_stands_as_markdownlint_wrote_it() {
        let relative =
            problem(path("sub/a.md")).relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, FilePath::try_from(path("sub/a.md")).ok());
    }
}
