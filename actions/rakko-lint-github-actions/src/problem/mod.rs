//! One place that zizmor reported about a workflow
//!
//! A run of zizmor reports one finding for each pattern that it recognized,
//! and a finding names one or more places in the workflow. This module holds
//! one of those places: where it is, how zizmor weighed the pattern, and what
//! zizmor said about the place.

/// The severity that zizmor gave a finding
mod severity;

use std::path::{Component, Path, PathBuf};

use getset::{CopyGetters, Getters};
use rakko_action::{FilePath, ProjectRoot, Span};

pub use self::severity::Severity;

/// The character that opens the severity of a problem in a message
const SEVERITY_OPEN: char = '[';

/// The text that closes the severity of a problem in a message
const SEVERITY_CLOSE: &str = "] ";

/// The text between the audit of a problem and what the audit is about
const AUDIT_CLOSE: &str = ": ";

/// The text that opens what zizmor wrote about the place of a problem
const ANNOTATION_OPEN: &str = " (";

/// The character that closes what zizmor wrote about the place of a problem
const ANNOTATION_CLOSE: char = ')';

/// One place that zizmor reported about a workflow
///
/// A finding of zizmor names one or more places. The first of them is where
/// the finding is, and the others are what a reader needs to read it, such as
/// the step that holds an expression or the job that permissions belong to.
/// Zizmor draws them together in one block of source. A problem holds one of
/// those places on its own, and the audit and the severity say which problems
/// came from the same finding.
///
/// The path stands as zizmor wrote it. A run starts zizmor in the project
/// root and names the root as the place to look, so zizmor reports every path
/// relative to it. A caller that reports the problem asks for the
/// [relative][relative] path, which drops the `./` that zizmor puts in front
/// of it.
///
/// Every place of zizmor has a start and an end, so a problem covers a range
/// and never sits at a single position.
///
/// The audit, the description, and the annotation are the words of zizmor.
/// The audit names the pattern that zizmor looked for, the description says
/// what that pattern is about, and the annotation says what zizmor recognized
/// at this place. A reader of a finding therefore reads the answer of the
/// tool, and not one that this crate wrote about it.
///
/// [relative]: ZizmorProblem::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct ZizmorProblem {
    /// The path of the file, as zizmor wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The range of the file that the place covers
    #[getset(get_copy = "pub")]
    span: Span,

    /// How zizmor weighed the pattern that it recognized
    #[getset(get_copy = "pub")]
    severity: Severity,

    /// The name of the audit that recognized the pattern
    #[getset(get = "pub")]
    audit: String,

    /// What the audit is about, in the words of zizmor
    #[getset(get = "pub")]
    description: String,

    /// What zizmor wrote about this place of the finding
    #[getset(get = "pub")]
    annotation: String,
}

impl ZizmorProblem {
    /// Creates a problem from one place that zizmor reported
    pub fn new(
        path: PathBuf,
        span: Span,
        severity: Severity,
        audit: String,
        description: String,
        annotation: String,
    ) -> Self {
        Self {
            path,
            span,
            severity,
            audit,
            description,
            annotation,
        }
    }

    /// Returns the sentence that zizmor wrote about the problem
    ///
    /// The sentence holds the severity, the audit, and what the audit is
    /// about, in the order that zizmor heads a finding with, and it ends with
    /// what zizmor wrote about this place. A reader therefore sees the two
    /// halves that zizmor draws above and below its block of source, and the
    /// places of one finding read as the same finding.
    // lintgithubactions[impl check.finding]
    // lintgithubactions[impl check.severity]
    pub fn message(&self) -> String {
        format!(
            "{SEVERITY_OPEN}{}{SEVERITY_CLOSE}{}{AUDIT_CLOSE}{}{ANNOTATION_OPEN}{}{ANNOTATION_CLOSE}",
            self.severity, self.audit, self.description, self.annotation
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

/// Returns the path without the prefix that names where zizmor started
///
/// A run names the current directory as the place to look, and zizmor keeps
/// that name in front of every path that it reports, so
/// `./.github/workflows/ci.yml` names the same file as
/// `.github/workflows/ci.yml`. The finding drops the prefix, because a reader
/// and a code host expect the plain path.
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

    use rakko_action::Position;

    use super::*;

    /// Returns a problem about the given path
    fn problem(path: &str) -> ZizmorProblem {
        weighed(path, Severity::High)
    }

    /// Returns a problem of the given severity about the given path
    fn weighed(path: &str, severity: Severity) -> ZizmorProblem {
        ZizmorProblem::new(
            PathBuf::from(path),
            span(),
            severity,
            "template-injection".to_owned(),
            "code injection via template expansion".to_owned(),
            "may expand into attacker-controllable code".to_owned(),
        )
    }

    /// Returns the range that the problems of a test cover
    fn span() -> Span {
        Span::builder()
            .start(Position::builder().line(8).column(24).build())
            .end(Position::builder().line(8).column(48).build())
            .build()
    }

    // lintgithubactions[verify check.finding]
    #[test]
    fn message_holds_the_words_of_zizmor() {
        let message = problem("./.github/workflows/ci.yml").message();

        assert_eq!(
            message,
            "[high] template-injection: code injection via template expansion \
             (may expand into attacker-controllable code)"
        );
    }

    // lintgithubactions[verify check.severity]
    #[test]
    fn message_of_an_informational_finding_names_its_severity() {
        let problem = weighed("./.github/workflows/ci.yml", Severity::Informational);

        let message = problem.message();

        assert!(
            message.starts_with("[informational] "),
            "expected the severity of zizmor, got {message}"
        );
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let path = problem("/home/otter/elsewhere/ci.yml")
            .relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let path = problem("/home/otter/project/.github/workflows/ci.yml")
            .relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, FilePath::try_from(".github/workflows/ci.yml").ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_drops_the_place_of_the_run() {
        let path = problem("./.github/workflows/ci.yml")
            .relative_path(&ProjectRoot::new(PathBuf::from("/home/otter/project")));

        assert_eq!(path, FilePath::try_from(".github/workflows/ci.yml").ok());
    }
}
