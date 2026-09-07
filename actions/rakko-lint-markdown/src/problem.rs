//! One rule that markdownlint reported about a file
//!
//! A run of markdownlint reports one result for each rule that a file broke,
//! and this module holds one of them: where the rule was broken, and what
//! markdownlint said about it.

use std::path::{Path, PathBuf};

use getset::{CopyGetters, Getters};
use rakko_action::{FilePath, ProjectRoot};

/// One rule that markdownlint reported about a file
///
/// The path stands as markdownlint wrote it. Markdownlint starts in the
/// project root and names a file relative to it, and a caller that reports
/// the problem asks for the [relative][relative] path, which strips a root
/// that arrived in front of it anyway.
///
/// Every result of markdownlint names a line, so a problem always has one. A
/// rule that can point at a character in that line names a column as well,
/// and one that speaks about the whole line names none.
///
/// The message is the sentence that markdownlint would have written for a
/// reader: the rule and its aliases, what the rule is about, and what it
/// expected here. A reader of a finding therefore reads the answer of the
/// tool, and not one that this crate wrote about it.
///
/// [relative]: MarkdownlintProblem::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct MarkdownlintProblem {
    /// The path of the file, as markdownlint wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The line that the rule was broken on, starting at 1
    #[getset(get_copy = "pub")]
    line: u32,

    /// The column that the rule points at, starting at 1
    ///
    /// A rule that speaks about the whole line names no column.
    #[getset(get_copy = "pub")]
    column: Option<u32>,

    /// What markdownlint said about the rule and the file
    #[getset(get = "pub")]
    message: String,
}

impl MarkdownlintProblem {
    /// Creates a problem from what markdownlint reported about a file
    pub fn new(path: PathBuf, line: u32, column: Option<u32>, message: String) -> Self {
        Self {
            path,
            line,
            column,
            message,
        }
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. Markdownlint
    /// starts in the root and names what it found below it, so a path that
    /// does not fit points at a report that the caller misread, and the
    /// caller decides what to do about that.
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::try_from(strip(&self.path, root)?).ok()
    }
}

/// Returns the path without the project root that prefixes it
///
/// A path that is already relative is the answer itself, because markdownlint
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
    fn problem(path: &str) -> MarkdownlintProblem {
        MarkdownlintProblem::new(
            PathBuf::from(path),
            1,
            None,
            "MD041/first-line-heading".to_owned(),
        )
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let path = problem("/home/otter/elsewhere/a.md")
            .relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let path = problem("/home/otter/project/sub/a.md")
            .relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, FilePath::try_from("sub/a.md").ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_stands_as_markdownlint_wrote_it() {
        let path = problem("sub/a.md")
            .relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, FilePath::try_from("sub/a.md").ok());
    }
}
