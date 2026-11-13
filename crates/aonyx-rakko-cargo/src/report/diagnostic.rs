//! One diagnostic of the compiler

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko::{Finding, Location, ProjectRoot};

use super::{DiagnosticLevel, DiagnosticSpan};
use crate::root::CargoRoot;

/// One diagnostic that the compiler reported through cargo
///
/// A diagnostic carries what a reader needs to find and understand a
/// problem. It contains the level, the code that names the lint or the error, the
/// message, and the source that it points at. The compiler writes more, such
/// as notes and suggestions below a diagnostic. Those stay behind because a
/// finding carries one message and one location.
///
/// A caller reads a diagnostic as the finding that [`finding`][finding]
/// gives. The parts of the diagnostic stay inside this crate, so a later
/// compatible release can add more of what the compiler writes.
///
/// The value compares by everything it holds, so a report can tell two
/// reports of one diagnostic apart from two diagnostics.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_cargo::CargoReport;
///
/// let line = r#"{"reason":"compiler-message","message":{"level":"warning","code":null,"message":"unused manifest key","spans":[]}}"#;
/// let report = CargoReport::read(line)?;
///
/// assert_eq!(report.diagnostics().len(), 1);
/// # Ok::<(), aonyx_rakko_cargo::ReadReportError>(())
/// ```
///
/// [finding]: CargoDiagnostic::finding
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct CargoDiagnostic {
    /// How serious the compiler considers the diagnostic
    #[getset(get_copy = "pub(crate)")]
    level: DiagnosticLevel,

    /// The code that names the lint or the error, when the compiler assigned
    /// one
    #[getset(get = "pub(crate)")]
    code: Option<String>,

    /// The message of the compiler
    #[getset(get = "pub(crate)")]
    message: String,

    /// The source that the diagnostic points at, when it points at one
    #[getset(get = "pub(crate)")]
    span: Option<DiagnosticSpan>,
}

#[bon]
impl CargoDiagnostic {
    /// Creates a diagnostic from what the compiler reported
    #[builder]
    pub(crate) fn new(
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
    /// The message of the finding equals the message of the compiler. When the
    /// diagnostic has a code, the finding adds it in brackets so a reader can
    /// look the lint up. The location is the range that the diagnostic covers,
    /// with the path relative to the project root.
    ///
    /// A diagnostic without a span gets a finding at the level of the project.
    /// A diagnostic whose file lies outside the project, such as one in a
    /// dependency that a macro pulled in, gets one too. The message then names
    /// the file and the position, so the place is not lost with the level.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::ProjectRoot;
    /// use aonyx_rakko_cargo::{CargoReport, CargoRoot, Documentation};
    ///
    /// let line = r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"unused_variables"},"message":"unused variable: `x`","spans":[{"file_name":"src/lib.rs","line_start":3,"column_start":9,"line_end":3,"column_end":10,"is_primary":true}]}}"#;
    /// let report = CargoReport::read(line)?;
    /// let project = ProjectRoot::new(std::env::temp_dir());
    /// let root = CargoRoot::builder()
    ///     .directory(std::env::temp_dir())
    ///     .documentation(Documentation::Testable)
    ///     .build();
    ///
    /// let finding = report.diagnostics()[0].finding(&root, &project);
    ///
    /// assert_eq!(finding.message().get(), "unused variable: `x` [unused_variables]");
    /// # Ok::<(), aonyx_rakko_cargo::ReadReportError>(())
    /// ```
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::PathBuf;

    use rakko::{Position, Span};

    use super::*;
    use crate::root::Documentation;

    /// Returns the project root that the tests place a root in
    fn project() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns the root of the harness of the project
    fn harness() -> CargoRoot {
        CargoRoot::builder()
            .directory(PathBuf::from("/home/otter/project/tools/harness"))
            .documentation(Documentation::Testable)
            .build()
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
            .span(
                DiagnosticSpan::builder()
                    .path(PathBuf::from(path))
                    .range(range())
                    .build(),
            )
            .build()
    }

    #[test]
    fn cargo_diagnostic_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CargoDiagnostic>();
    }

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

    #[test]
    fn finding_outside_the_project_belongs_to_the_project() {
        let diagnostic = warning("/home/otter/.cargo/registry/dep/src/lib.rs");

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(finding.location(), &Location::Project);
    }

    #[test]
    fn finding_outside_the_project_names_the_place_in_the_message() {
        let diagnostic = warning("/home/otter/.cargo/registry/dep/src/lib.rs");

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(
            finding.message().get(),
            "used `unwrap()` on an `Option` value [clippy::unwrap_used] at /home/otter/.cargo/registry/dep/src/lib.rs:8:88"
        );
    }

    #[test]
    fn finding_puts_the_code_behind_the_message() {
        let diagnostic = warning("src/main.rs");

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(
            finding.message().get(),
            "used `unwrap()` on an `Option` value [clippy::unwrap_used]"
        );
    }

    #[test]
    fn finding_without_a_code_keeps_the_message() {
        let diagnostic = CargoDiagnostic::builder()
            .level(DiagnosticLevel::Warning)
            .message("unused variable")
            .span(
                DiagnosticSpan::builder()
                    .path(PathBuf::from("src/main.rs"))
                    .range(range())
                    .build(),
            )
            .build();

        let finding = diagnostic.finding(&harness(), &project());

        assert_eq!(finding.message().get(), "unused variable");
    }

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
