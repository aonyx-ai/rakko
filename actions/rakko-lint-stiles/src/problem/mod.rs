//! One diagnostic that stiles reported about a file
//!
//! A run of stiles reports one diagnostic for each place where a rule of the
//! project fired. This module holds one of those diagnostics: where it is, how
//! the rule weighed it, and what the rule said about it.

/// The severity that stiles gave a diagnostic
mod severity;

use std::path::PathBuf;

use getset::{CopyGetters, Getters};
use rakko_action::{FilePath, Position, ProjectRoot};

pub use self::severity::Severity;

/// The character that opens the severity of a problem in a message
const SEVERITY_OPEN: char = '[';

/// The text that closes the severity of a problem in a message
const SEVERITY_CLOSE: &str = "] ";

/// The text between the rule of a problem and what the rule said
const RULE_CLOSE: &str = ": ";

/// One diagnostic that stiles reported about a file
///
/// The path stands as stiles wrote it. A run starts stiles in the project
/// root and names the root as the place to check, so stiles reports every
/// path relative to it. A caller that reports the problem asks for the
/// [relative][relative] path, which drops the `./` that stiles can put in
/// front of it.
///
/// Stiles reports the line and the column where a diagnostic starts, and the
/// byte offset where it ends. A byte offset is no position that an editor
/// shows, so a problem sits at the position where it starts.
///
/// The rule and the message are the words of stiles. The rule names the check
/// that fired, and the message says what it found. A reader of a finding
/// therefore reads the answer of the tool, and not one that this crate wrote
/// about it.
///
/// [relative]: StilesProblem::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct StilesProblem {
    /// The path of the file, as stiles wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The position where the diagnostic starts
    #[getset(get_copy = "pub")]
    position: Position,

    /// How the rule weighed the diagnostic
    #[getset(get_copy = "pub")]
    severity: Severity,

    /// The identifier of the rule that fired, such as `lint.todo-comment`
    #[getset(get = "pub")]
    rule: String,

    /// What the rule said about the code, in the words of stiles
    #[getset(get = "pub")]
    text: String,
}

impl StilesProblem {
    /// Creates a problem from one diagnostic that stiles reported
    pub fn new(
        path: PathBuf,
        position: Position,
        severity: Severity,
        rule: String,
        text: String,
    ) -> Self {
        Self {
            path,
            position,
            severity,
            rule,
            text,
        }
    }

    /// Returns the sentence that stiles wrote about the problem
    ///
    /// The sentence holds the severity, the rule, and what the rule said, so a
    /// reader sees which rule to turn off or to fix the code for.
    pub fn message(&self) -> String {
        format!(
            "{SEVERITY_OPEN}{}{SEVERITY_CLOSE}{}{RULE_CLOSE}{}",
            self.severity, self.rule, self.text
        )
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. A run names the
    /// root as the place to check, so a path that does not fit points at a
    /// report that the caller misread, and the caller decides what to do about
    /// that.
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::within(&self.path, root)
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
    fn problem(path: PathBuf) -> StilesProblem {
        StilesProblem::new(
            path,
            Position::builder().line(7).column(5).build(),
            Severity::Warning,
            "lint.todo-comment".to_owned(),
            "a TODO comment marks unfinished work".to_owned(),
        )
    }

    #[test]
    fn message_holds_the_words_of_stiles() {
        let message = problem(path("src/lib.rs")).message();

        assert_eq!(
            message,
            "[warning] lint.todo-comment: a TODO comment marks unfinished work"
        );
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let relative = problem(path("/home/otter/elsewhere/lib.rs"))
            .relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let relative = problem(path("/home/otter/project/src/lib.rs"))
            .relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, FilePath::try_from(path("src/lib.rs")).ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_drops_the_place_of_the_run() {
        let relative = problem(path("./src/lib.rs"))
            .relative_path(&ProjectRoot::new(path("/home/otter/project")));

        assert_eq!(relative, FilePath::try_from(path("src/lib.rs")).ok());
    }
}
