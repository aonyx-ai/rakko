//! One rule that oxlint reported about a file
//!
//! A run of oxlint reports one diagnostic for each rule that a file broke,
//! and this module holds one of them: where the rule was broken, how the
//! project weighs it, and what oxlint said about it.

/// The severity that oxlint gave a diagnostic
mod severity;

use std::path::PathBuf;

use bon::Builder;
use getset::{CopyGetters, Getters};
use rakko_action::{FilePath, ProjectRoot};

pub use self::severity::Severity;

/// The character that opens the severity of a diagnostic in a message
const SEVERITY_OPEN: char = '[';

/// The text that closes the severity of a diagnostic in a message
const SEVERITY_CLOSE: &str = "] ";

/// The text that separates the rule of a diagnostic from its description
const RULE_CLOSE: &str = ": ";

/// The text that introduces what oxlint suggests about a diagnostic
///
/// Oxlint writes the suggestion behind this word in its own report, and a
/// message that carries one reads the same way.
const HELP_OPEN: &str = " help: ";

/// One rule that oxlint reported about a file
///
/// The path stands as oxlint wrote it. A run starts oxlint in the project root
/// and names the root as the place to look, so oxlint reports every path
/// relative to it. A caller that reports the diagnostic asks for the
/// [relative][relative] path.
///
/// A diagnostic marks one or more places of a file, and this value carries the
/// first of them, because oxlint reports one diagnostic however many places it
/// marks.
///
/// The description is the sentence that oxlint wrote for a reader, the code is
/// the rule that the file broke, and the severity is how the project weighs
/// that rule. Oxlint suggests what to do about most of its rules, and the
/// suggestion travels where oxlint wrote one. A reader of a finding therefore
/// reads the answer of the tool, and not one that this crate wrote about it.
///
/// [relative]: OxlintProblem::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct OxlintProblem {
    /// The path of the file, as oxlint wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The line that the rule was broken on, starting at 1
    #[getset(get_copy = "pub")]
    line: u32,

    /// The column that the rule points at, starting at 1
    #[getset(get_copy = "pub")]
    column: u32,

    /// How the project weighs the rule that the file broke
    #[getset(get_copy = "pub")]
    severity: Severity,

    /// The rule that the file broke, as oxlint names it
    #[builder(into)]
    #[getset(get = "pub")]
    code: String,

    /// What oxlint said about the rule and the file
    #[builder(into)]
    #[getset(get = "pub")]
    description: String,

    /// What oxlint suggests about the diagnostic, where it suggests anything
    #[builder(into)]
    #[getset(get = "pub")]
    help: Option<String>,
}

impl OxlintProblem {
    /// Returns the sentence that oxlint wrote about the diagnostic
    ///
    /// The sentence holds the severity, the rule, the description, and the
    /// suggestion, in the order that oxlint writes them, so that a finding
    /// reads like the line that a contributor sees when they run oxlint
    /// themselves.
    pub fn message(&self) -> String {
        let help = match &self.help {
            Some(help) => format!("{HELP_OPEN}{help}"),
            None => String::new(),
        };

        format!(
            "{SEVERITY_OPEN}{}{SEVERITY_CLOSE}{}{RULE_CLOSE}{}{help}",
            self.severity, self.code, self.description
        )
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. A run names the
    /// root as the place to look, so a path that does not fit points at a
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

    use std::path::PathBuf;

    use rakko_action::{FilePath, ProjectRoot};
    use rakko_test_utils::path;

    use super::*;

    /// Returns a diagnostic about the given path, with a sentence about it
    fn problem(path: PathBuf) -> OxlintProblem {
        OxlintProblem::builder()
            .path(path)
            .line(2)
            .column(3)
            .severity(Severity::Warning)
            .code("eslint(no-debugger)")
            .description("`debugger` statement is not allowed")
            .help("Remove the debugger statement")
            .build()
    }

    /// The root that the diagnostics of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn message_of_a_diagnostic_reads_like_the_line_of_oxlint() {
        let message = problem(path("index.ts")).message();

        assert_eq!(
            message,
            "[warning] eslint(no-debugger): `debugger` statement is not allowed \
             help: Remove the debugger statement"
        );
    }

    #[test]
    fn message_of_a_diagnostic_without_a_sentence_holds_the_rule_alone() {
        let problem = OxlintProblem::builder()
            .path(path("index.ts"))
            .line(1)
            .column(7)
            .severity(Severity::Error)
            .code("eslint(id-length)")
            .description("Identifier name `x` is too short")
            .build();

        assert_eq!(
            problem.message(),
            "[error] eslint(id-length): Identifier name `x` is too short"
        );
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let relative = problem(path("/home/otter/elsewhere/index.ts")).relative_path(&root());

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let relative = problem(path("/home/otter/project/src/index.ts")).relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("src/index.ts")).ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_names_the_file() {
        let relative = problem(path("src/index.ts")).relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("src/index.ts")).ok());
    }
}
