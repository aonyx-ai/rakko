//! One rule that yamllint reported about a file
//!
//! A run of yamllint reports one problem for each rule that a file broke, and
//! this module holds one of them: where the rule was broken, how the project
//! weighed it, and what yamllint said about it.

/// The level that yamllint gave a problem
mod level;

use std::path::{Component, Path, PathBuf};

use getset::{CopyGetters, Getters};
use rakko_action::{FilePath, ProjectRoot};

pub use self::level::ProblemLevel;

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
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct YamllintProblem {
    /// The path of the file, as yamllint wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The line that the rule was broken on, starting at 1
    #[getset(get_copy = "pub")]
    line: u32,

    /// The column that the rule points at, starting at 1
    #[getset(get_copy = "pub")]
    column: u32,

    /// How the project weighed the rule that the file broke
    #[getset(get_copy = "pub")]
    level: ProblemLevel,

    /// What yamllint said about the rule and the file
    #[getset(get = "pub")]
    description: String,
}

impl YamllintProblem {
    /// Creates a problem from what yamllint reported about a file
    pub fn new(
        path: PathBuf,
        line: u32,
        column: u32,
        level: ProblemLevel,
        description: String,
    ) -> Self {
        Self {
            path,
            line,
            column,
            level,
            description,
        }
    }

    /// Returns the sentence that yamllint wrote about the problem
    ///
    /// The sentence holds the level and the description, in the order that
    /// yamllint writes them, so that a finding reads like the line that a
    /// contributor sees when they run yamllint themselves.
    pub fn message(&self) -> String {
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
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::try_from(strip(&self.path, root)?).ok()
    }
}

/// Returns the path without the prefix that names where yamllint started
///
/// A run names the current directory as the place to look, and yamllint keeps
/// that name in front of every path that it reports, so `./sub/a.yaml` names
/// the same file as `sub/a.yaml`. The finding drops the prefix, because a
/// reader and a code host expect the plain path.
///
/// A path that arrives absolute loses the project root instead. The root of a
/// context can name the same directory through a symbolic link, which is why
/// the canonical root is tried as well.
fn strip(path: &Path, root: &ProjectRoot) -> Option<PathBuf> {
    if path.is_relative() {
        let plain: PathBuf = path
            .components()
            .filter(|component| *component != Component::CurDir)
            .collect();

        return Some(plain);
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
    fn problem(path: &str) -> YamllintProblem {
        YamllintProblem::new(
            PathBuf::from(path),
            1,
            1,
            ProblemLevel::Warning,
            "missing document start \"---\" (document-start)".to_owned(),
        )
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    // lintyaml[verify check.problem]
    #[test]
    fn message_of_a_problem_reads_like_the_line_of_yamllint() {
        let message = problem("./notes.yaml").message();

        assert_eq!(
            message,
            "[warning] missing document start \"---\" (document-start)"
        );
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let path = problem("/home/otter/elsewhere/a.yaml").relative_path(&root());

        assert_eq!(path, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let path = problem("/home/otter/project/sub/a.yaml").relative_path(&root());

        assert_eq!(path, FilePath::try_from("sub/a.yaml").ok());
    }

    // lintyaml[verify check.problem]
    #[test]
    fn relative_path_that_arrived_relative_drops_the_prefix_of_yamllint() {
        let path = problem("./sub/a.yaml").relative_path(&root());

        assert_eq!(path, FilePath::try_from("sub/a.yaml").ok());
    }
}
