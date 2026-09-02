//! What cargo reported about a build
//!
//! Cargo writes the diagnostics of a build as one JSON document per line on
//! its standard output when a run asks for that format, and this module
//! reads those lines. The shape of the lines belongs to a version of cargo,
//! and this module is the one place that knows it.

/// One diagnostic of the compiler
mod diagnostic;
/// The error that stops the reading
mod error;
/// How serious the compiler considers a diagnostic
mod level;
/// The source that a diagnostic points at
mod span;

use std::collections::HashSet;
use std::path::PathBuf;

use getset::{CopyGetters, Getters};
use rakko_action::{Position, Span};
use serde::Deserialize;
use serde::de::DeserializeOwned;

pub use self::diagnostic::CargoDiagnostic;
pub use self::error::ReadReportError;
pub use self::level::DiagnosticLevel;
pub use self::span::DiagnosticSpan;

/// The reason of a line that carries a diagnostic of the compiler
const COMPILER_MESSAGE: &str = "compiler-message";

/// The reason of the line that closes the output of a build
const BUILD_FINISHED: &str = "build-finished";

/// The level of a diagnostic that the compiler refused the code for
const ERROR: &str = "error";

/// The level of a diagnostic that the compiler objects to the code with
const WARNING: &str = "warning";

/// What cargo reported about one build
///
/// The report holds the diagnostics of the compiler, each once, and whether
/// cargo said that the build finished with success. A caller that ran cargo
/// with the JSON format reads its standard output into a report and decides
/// what the diagnostics mean for its outcome.
///
/// # Examples
///
/// ```
/// use rakko_cargo::CargoReport;
///
/// let stdout = r#"{"reason":"build-finished","success":true}"#;
///
/// let report = CargoReport::read(stdout)?;
///
/// assert_eq!(report.finished(), Some(true));
/// # Ok::<(), rakko_cargo::ReadReportError>(())
/// ```
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct CargoReport {
    /// The diagnostics of the compiler, each once, in the order of the
    /// output
    #[getset(get = "pub")]
    diagnostics: Vec<CargoDiagnostic>,

    /// Whether cargo said that the build finished with success, or `None`
    /// when cargo did not say
    #[getset(get_copy = "pub")]
    finished: Option<bool>,
}

impl CargoReport {
    /// Reads the report of a build from what cargo wrote to its standard
    /// output
    ///
    /// The reading keeps every compiler message at the level of a warning
    /// or an error, once each, and the line that says whether the build
    /// finished. It ignores every other line: the artifacts that cargo
    /// lists, the notes and the help lines of the compiler, and a line that
    /// is not a record of cargo at all, so the output of a tool that cargo
    /// runs can share the stream.
    ///
    /// # Errors
    ///
    /// Returns [`UnrecognizedRecord`][unrecognized] when a line names a
    /// compiler message or the end of the build and the crate cannot read
    /// its body. The shape belongs to a version of cargo, and a reading that
    /// skipped such a line would let a build pass with its problems unread.
    ///
    /// [unrecognized]: ReadReportError::UnrecognizedRecord
    // cargo[impl diagnostic.finished]
    // cargo[impl diagnostic.ignore]
    // cargo[impl diagnostic.once]
    // cargo[impl diagnostic.read]
    // cargo[impl diagnostic.unrecognized]
    pub fn read(stdout: &str) -> Result<Self, ReadReportError> {
        let mut diagnostics = Vec::new();
        let mut seen = HashSet::new();
        let mut finished = None;

        for line in stdout.lines() {
            let Ok(probe) = serde_json::from_str::<Probe>(line) else {
                continue;
            };

            match probe.reason.as_str() {
                COMPILER_MESSAGE => {
                    let record: CompilerMessage = decode(line)?;

                    if let Some(diagnostic) = diagnostic(record.message)
                        && seen.insert(diagnostic.clone())
                    {
                        diagnostics.push(diagnostic);
                    }
                }
                BUILD_FINISHED => {
                    let record: BuildFinished = decode(line)?;
                    finished = Some(record.success);
                }
                _ => {}
            }
        }

        Ok(Self {
            diagnostics,
            finished,
        })
    }
}

/// The reason of a line of cargo, which says what the line is about
///
/// Every record of cargo carries a reason, and a line without one comes from
/// another tool that shares the stream. Cargo writes more than this field,
/// and the reading ignores the rest, so a field that a new version adds does
/// not break it.
#[derive(Deserialize)]
struct Probe {
    /// What the line is about
    reason: String,
}

/// A line that carries a diagnostic of the compiler
#[derive(Deserialize)]
struct CompilerMessage {
    /// The diagnostic that cargo forwards
    message: Message,
}

/// The line that closes the output of a build
#[derive(Deserialize)]
struct BuildFinished {
    /// Whether the build finished with success
    success: bool,
}

/// Reads a record of cargo from a line whose reason the crate knows
///
/// # Errors
///
/// Returns [`UnrecognizedRecord`][unrecognized] when the line does not have
/// the shape of the record.
///
/// [unrecognized]: ReadReportError::UnrecognizedRecord
// cargo[impl diagnostic.unrecognized]
fn decode<T: DeserializeOwned>(line: &str) -> Result<T, ReadReportError> {
    serde_json::from_str(line).map_err(|source| ReadReportError::UnrecognizedRecord {
        line: line.to_owned(),
        source,
    })
}

/// One diagnostic of the compiler, as cargo forwards it
#[derive(Deserialize)]
struct Message {
    /// The level of the diagnostic
    level: String,

    /// The code that names the lint or the error, when there is one
    code: Option<Code>,

    /// The message of the compiler
    message: String,

    /// The sources that the diagnostic points at
    spans: Vec<SourceSpan>,
}

/// The code of a diagnostic, as cargo forwards it
#[derive(Deserialize)]
struct Code {
    /// The code that names the lint or the error
    code: String,
}

/// One source that a diagnostic points at, as cargo forwards it
#[derive(Deserialize)]
struct SourceSpan {
    /// The file that the source is in, relative to the root that cargo
    /// checked
    file_name: PathBuf,

    /// The line where the source starts
    line_start: u32,

    /// The column where the source starts
    column_start: u32,

    /// The line where the source ends
    line_end: u32,

    /// The column where the source ends
    column_end: u32,

    /// Whether this is the source that the diagnostic is about, and not one
    /// that explains it
    is_primary: bool,
}

/// Returns the diagnostic of a message, when the message names a problem
///
/// A note, a help line, and the line that ends a failed build explain other
/// diagnostics, and they answer `None`. The primary span is the source that
/// the diagnostic is about; a message without one gets the first span it
/// has, and a message without any span gets none.
// cargo[impl diagnostic.ignore]
// cargo[impl diagnostic.read]
fn diagnostic(message: Message) -> Option<CargoDiagnostic> {
    let level = match message.level.as_str() {
        ERROR => DiagnosticLevel::Error,
        WARNING => DiagnosticLevel::Warning,
        _ => return None,
    };

    let span = message
        .spans
        .iter()
        .find(|span| span.is_primary)
        .or_else(|| message.spans.first())
        .map(|span| {
            DiagnosticSpan::new(
                span.file_name.clone(),
                Span::builder()
                    .start(
                        Position::builder()
                            .line(span.line_start)
                            .column(span.column_start)
                            .build(),
                    )
                    .end(
                        Position::builder()
                            .line(span.line_end)
                            .column(span.column_end)
                            .build(),
                    )
                    .build(),
            )
        });

    Some(
        CargoDiagnostic::builder()
            .level(level)
            .maybe_code(message.code.map(|code| code.code))
            .message(message.message)
            .maybe_span(span)
            .build(),
    )
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a report
    // which cargo could have written expects the reading to succeed. A
    // `# Panics` section on every test would repeat that and give the reader
    // no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A line that lists an artifact of the build
    const ARTIFACT: &str = r#"{"reason":"compiler-artifact","target":{"name":"probe"}}"#;

    /// A diagnostic of the compiler about a type that does not fit
    const ERROR: &str = r#"{"reason":"compiler-message","message":{"level":"error","code":{"code":"E0308"},"message":"mismatched types","spans":[{"file_name":"src/broken.rs","line_start":1,"column_start":26,"line_end":1,"column_end":30,"is_primary":true}]}}"#;

    /// The line that closes a build that failed
    const FAILED: &str = r#"{"reason":"build-finished","success":false}"#;

    /// The line that ends a failed build with a pointer to the explanation
    const FAILURE_NOTE: &str = r#"{"reason":"compiler-message","message":{"level":"failure-note","code":null,"message":"For more information about this error, try `rustc --explain E0308`.","spans":[]}}"#;

    /// The line that closes a build that finished
    const FINISHED: &str = r#"{"reason":"build-finished","success":true}"#;

    /// A line of another tool that shares the stream, in the shape of nextest
    const FOREIGN: &str = r#"{"type":"suite","event":"ok","passed":3,"failed":0}"#;

    /// A compiler message in a shape that the crate does not know
    const MISSHAPEN: &str = r#"{"reason":"compiler-message","message":{"level":"warning","text":"a field that the crate does not know"}}"#;

    /// A diagnostic without a source, such as one about the manifest
    const UNPLACED: &str = r#"{"reason":"compiler-message","message":{"level":"warning","code":null,"message":"unused manifest key","spans":[]}}"#;

    /// A diagnostic of clippy, with a secondary span before the primary one
    const WARNING: &str = r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::unwrap_used"},"message":"used `unwrap()` on an `Option` value","spans":[{"file_name":"src/lib.rs","line_start":7,"column_start":1,"line_end":7,"column_end":2,"is_primary":false},{"file_name":"src/lib.rs","line_start":8,"column_start":88,"line_end":8,"column_end":98,"is_primary":true}]}}"#;

    /// Returns the output of a build made of the given lines
    fn output(lines: &[&str]) -> String {
        let mut output = lines.join("\n");
        output.push('\n');

        output
    }

    /// Returns the report of a build made of the given lines
    fn read(lines: &[&str]) -> CargoReport {
        CargoReport::read(&output(lines)).expect("the test reads a report that cargo could write")
    }

    // cargo[verify diagnostic.finished]
    #[test]
    fn read_a_build_that_failed_reports_no_success() {
        let report = read(&[ERROR, FAILED]);

        assert_eq!(report.finished(), Some(false));
    }

    // cargo[verify diagnostic.finished]
    #[test]
    fn read_a_build_that_finished_reports_success() {
        let report = read(&[ARTIFACT, FINISHED]);

        assert_eq!(report.finished(), Some(true));
    }

    // cargo[verify diagnostic.unrecognized]
    #[test]
    fn read_a_compiler_message_it_cannot_read_stops_with_the_line() {
        let report = CargoReport::read(&output(&[MISSHAPEN, FINISHED]));

        assert!(
            matches!(
                &report,
                Err(ReadReportError::UnrecognizedRecord { line, .. }) if line == MISSHAPEN
            ),
            "expected an unrecognized record, got {report:?}"
        );
    }

    // cargo[verify diagnostic.read]
    #[test]
    fn read_a_diagnostic_reads_its_code() {
        let report = read(&[WARNING, FINISHED]);

        assert_eq!(
            report
                .diagnostics()
                .first()
                .and_then(|d| d.code().as_deref()),
            Some("clippy::unwrap_used")
        );
    }

    // cargo[verify diagnostic.read]
    #[test]
    fn read_a_diagnostic_reads_its_level() {
        let report = read(&[ERROR, FAILED]);

        assert_eq!(
            report.diagnostics().first().map(CargoDiagnostic::level),
            Some(DiagnosticLevel::Error)
        );
    }

    // cargo[verify diagnostic.read]
    #[test]
    fn read_a_diagnostic_reads_its_message() {
        let report = read(&[ERROR, FAILED]);

        assert_eq!(
            report.diagnostics().first().map(CargoDiagnostic::message),
            Some(&"mismatched types".to_owned())
        );
    }

    // cargo[verify diagnostic.read]
    #[test]
    fn read_a_diagnostic_reads_its_primary_span() {
        let report = read(&[WARNING, FINISHED]);

        let span = report
            .diagnostics()
            .first()
            .and_then(|diagnostic| diagnostic.span().as_ref());
        assert_eq!(
            span,
            Some(&DiagnosticSpan::new(
                PathBuf::from("src/lib.rs"),
                Span::builder()
                    .start(Position::builder().line(8).column(88).build())
                    .end(Position::builder().line(8).column(98).build())
                    .build(),
            ))
        );
    }

    // cargo[verify diagnostic.once]
    #[test]
    fn read_a_diagnostic_that_cargo_repeats_keeps_one() {
        let report = read(&[WARNING, WARNING, FINISHED]);

        assert_eq!(report.diagnostics().len(), 1);
    }

    // cargo[verify diagnostic.read]
    #[test]
    fn read_a_diagnostic_without_a_span_has_none() {
        let report = read(&[UNPLACED, FINISHED]);

        assert_eq!(
            report.diagnostics().first().map(CargoDiagnostic::span),
            Some(&None)
        );
    }

    // cargo[verify diagnostic.ignore]
    #[test]
    fn read_a_line_of_another_tool_ignores_it() {
        let report = read(&[FOREIGN, FINISHED]);

        assert!(report.diagnostics().is_empty());
    }

    // cargo[verify diagnostic.ignore]
    #[test]
    fn read_a_line_that_is_not_json_ignores_it() {
        let report = read(&["Compiling probe v0.1.0", FINISHED]);

        assert!(report.diagnostics().is_empty());
    }

    // cargo[verify diagnostic.ignore]
    #[test]
    fn read_a_note_that_ends_a_failed_build_ignores_it() {
        let report = read(&[ERROR, FAILURE_NOTE, FAILED]);

        assert_eq!(report.diagnostics().len(), 1);
    }

    // cargo[verify diagnostic.ignore]
    #[test]
    fn read_an_artifact_ignores_it() {
        let report = read(&[ARTIFACT, FINISHED]);

        assert!(report.diagnostics().is_empty());
    }

    // cargo[verify diagnostic.finished]
    #[test]
    fn read_an_output_without_a_closing_line_reports_nothing_about_success() {
        let report = read(&[WARNING]);

        assert_eq!(report.finished(), None);
    }
}
