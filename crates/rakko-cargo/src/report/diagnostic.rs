use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_action::{Finding, Location, ProjectRoot};

use super::{DiagnosticLevel, DiagnosticSpan};
use crate::root::CargoRoot;

/// One diagnostic that the compiler reported through cargo
///
/// A diagnostic carries what a reader needs to find and understand a
/// problem: the level, the code that names the lint or the error, the
/// message, and the source that it points at. The compiler writes more, such
/// as the notes and the suggestions below a diagnostic, and those stay
/// behind, because a finding carries one message and one location.
///
/// The value compares by everything it holds, so a report can tell two
/// reports of one diagnostic apart from two diagnostics.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct CargoDiagnostic {
    /// How serious the compiler considers the diagnostic
    #[getset(get_copy = "pub")]
    level: DiagnosticLevel,

    /// The code that names the lint or the error, when the compiler assigned
    /// one
    #[getset(get = "pub")]
    code: Option<String>,

    /// The message of the compiler
    #[getset(get = "pub")]
    message: String,

    /// The source that the diagnostic points at, when it points at one
    #[getset(get = "pub")]
    span: Option<DiagnosticSpan>,
}

#[bon]
impl CargoDiagnostic {
    /// Creates a diagnostic from what the compiler reported
    #[builder]
    pub fn new(
        level: DiagnosticLevel,
        #[builder(into)] code: Option<String>,
        #[builder(into)] message: String,
        span: Option<DiagnosticSpan>,
    ) -> Self {
        Self {
            level,
            code,
            message,
            span,
        }
    }

    /// Returns the finding that reports this diagnostic
    ///
    /// The message of the finding is the message of the compiler, followed
    /// by the code in brackets when the diagnostic has one, so a reader can
    /// look the lint up. The location is the range that the diagnostic
    /// covers, with the path relative to the project root.
    ///
    /// A diagnostic without a span, and a diagnostic whose file lies outside
    /// the project, such as one in a dependency that a macro pulled in, get
    /// a finding at the level of the project. The message then names the
    /// file and the position, so the place is not lost with the level.
    // cargo[impl diagnostic.finding]
    // cargo[impl diagnostic.foreign]
    pub fn finding(&self, root: &CargoRoot, project: &ProjectRoot) -> Finding {
        let message = match &self.code {
            Some(code) => format!("{} [{code}]", self.message),
            None => self.message.clone(),
        };

        let Some(span) = &self.span else {
            return Finding::builder()
                .message(message)
                .location(Location::Project)
                .build();
        };

        match root.relative_path(span.path(), project) {
            Some(path) => Finding::builder()
                .message(message)
                .location(Location::Span {
                    path,
                    span: *span.range(),
                })
                .build(),
            None => {
                let start = span.range().start();
                let column = start
                    .column()
                    .map_or_else(String::new, |column| format!(":{column}"));

                Finding::builder()
                    .message(format!(
                        "{message} at {}:{}{column}",
                        span.path().display(),
                        start.line()
                    ))
                    .location(Location::Project)
                    .build()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use rakko_action::{Position, Span};

    use super::*;

    /// Returns the project root that the tests place a root in
    fn project() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns the root of the harness of the project
    fn harness() -> CargoRoot {
        CargoRoot::new(PathBuf::from("/home/otter/project/tools/harness"))
    }

    /// Returns the range that the diagnostics of the tests cover
    fn range() -> Span {
        Span::builder()
            .start(Position::builder().line(8).column(88).build())
            .end(Position::builder().line(8).column(98).build())
            .build()
    }

    /// Returns a warning of clippy about the given file
    fn warning(path: &str) -> CargoDiagnostic {
        CargoDiagnostic::builder()
            .level(DiagnosticLevel::Warning)
            .code("clippy::unwrap_used")
            .message("used `unwrap()` on an `Option` value")
            .span(DiagnosticSpan::new(PathBuf::from(path), range()))
            .build()
    }

    // cargo[verify diagnostic.finding]
    #[test]
    fn finding_covers_the_range_of_the_span() {
        let diagnostic = warning("src/main.rs");

        let finding = diagnostic.finding(&harness(), &project());

        assert!(
            matches!(finding.location(), Location::Span { span, .. } if *span == range()),
            "expected the range of the diagnostic, got {:?}",
            finding.location()
        );
    }

    // cargo[verify diagnostic.finding]
    #[test]
    fn finding_names_the_path_relative_to_the_project() {
        let diagnostic = warning("src/main.rs");

        let finding = diagnostic.finding(&harness(), &project());

        assert!(
            matches!(
                finding.location(),
                Location::Span { path, .. } if path.get() == std::path::Path::new("tools/harness/src/main.rs")
            ),
            "expected a path relative to the project, got {:?}",
            finding.location()
        );
    }

    // cargo[verify diagnostic.foreign]
    #[test]
    fn finding_outside_the_project_belongs_to_the_project() {
        let diagnostic = warning("/home/otter/.cargo/registry/dep/src/lib.rs");

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(finding.location(), &Location::Project);
    }

    // cargo[verify diagnostic.foreign]
    #[test]
    fn finding_outside_the_project_names_the_place_in_the_message() {
        let diagnostic = warning("/home/otter/.cargo/registry/dep/src/lib.rs");

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(
            finding.message().get(),
            "used `unwrap()` on an `Option` value [clippy::unwrap_used] at /home/otter/.cargo/registry/dep/src/lib.rs:8:88"
        );
    }

    // cargo[verify diagnostic.finding]
    #[test]
    fn finding_puts_the_code_behind_the_message() {
        let diagnostic = warning("src/main.rs");

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(
            finding.message().get(),
            "used `unwrap()` on an `Option` value [clippy::unwrap_used]"
        );
    }

    // cargo[verify diagnostic.finding]
    #[test]
    fn finding_without_a_code_keeps_the_message() {
        let diagnostic = CargoDiagnostic::builder()
            .level(DiagnosticLevel::Warning)
            .message("unused variable")
            .span(DiagnosticSpan::new(PathBuf::from("src/main.rs"), range()))
            .build();

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(finding.message().get(), "unused variable");
    }

    // cargo[verify diagnostic.foreign]
    #[test]
    fn finding_without_a_span_belongs_to_the_project() {
        let diagnostic = CargoDiagnostic::builder()
            .level(DiagnosticLevel::Error)
            .code("E0601")
            .message("`main` function not found")
            .build();

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(finding.location(), &Location::Project);
    }
}
