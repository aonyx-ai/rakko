//! The reading of the report that tsc wrote
//!
//! Tsc writes one line per diagnostic, and it indents the lines that explain a
//! diagnostic underneath it. This module turns that text into diagnostics, and
//! it judges nothing: the caller decides what a diagnostic means for the
//! outcome of a run.

/// The error that leaves a caller without the diagnostics of a run
mod error;

use std::path::PathBuf;

pub use self::error::ReadReportError;
use crate::diagnostic::{Category, Diagnostic, Origin};

/// The text that closes the place of a diagnostic
///
/// Tsc writes the place as the path, the line, and the column in brackets, and
/// it closes the whole of it with this text before it writes the kind. A path
/// can hold a bracket of its own, so the first occurrence is the one that
/// closes the place.
const PLACE_CLOSE: &str = "): ";

/// The character that opens the line and the column of a place
const POSITION_OPEN: char = '(';

/// The character that separates the line of a place from its column
const POSITION_SEPARATOR: char = ',';

/// The text that separates the number of a diagnostic from what tsc said
const NUMBER_CLOSE: &str = ": ";

/// The character that separates the kind of a diagnostic from its number
const KIND_CLOSE: char = ' ';

/// The text that joins an explaining line to the text of its diagnostic
///
/// Tsc writes the explanation of a diagnostic as indented lines underneath it,
/// and each of them ends in a full stop. A message carries no formatting, so
/// the lines join into one sentence with the separator that a reader expects
/// between two sentences.
const JOIN: char = ' ';

/// Returns the diagnostics that tsc reported
///
/// Tsc writes one line per diagnostic, and it indents the lines that explain
/// one underneath it. An explaining line joins the text of the diagnostic
/// above it, so a caller gets one diagnostic however many lines tsc wrote
/// about it.
///
/// A run that reported nothing produces no diagnostic. The caller decides what
/// that means, because tsc writes nothing both for a project that it checked
/// and found clean and for a run that it never started.
///
/// # Errors
///
/// Returns [`UnreadableLine`][unreadable] for a line that is neither the start
/// of a diagnostic nor an explanation of the one above it.
///
/// [unreadable]: ReadReportError::UnreadableLine
// checktypescript[impl check.elaboration]
// checktypescript[impl check.diagnostic]
// checktypescript[impl check.project]
// checktypescript[impl check.unreadable]
pub fn read(report: &str) -> Result<Vec<Diagnostic>, ReadReportError> {
    let mut diagnostics = Vec::new();
    let mut open: Option<Pending> = None;

    for line in report.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if line.starts_with(char::is_whitespace) {
            let Some(pending) = open.as_mut() else {
                return Err(unreadable(line));
            };

            pending.explain(line.trim());
            continue;
        }

        let Some(started) = start(line) else {
            return Err(unreadable(line));
        };

        if let Some(previous) = open.replace(started) {
            diagnostics.push(previous.finish());
        }
    }

    if let Some(pending) = open {
        diagnostics.push(pending.finish());
    }

    Ok(diagnostics)
}

/// One diagnostic whose explaining lines the reading still collects
///
/// Tsc writes the explanation of a diagnostic underneath it, so a diagnostic
/// is complete only once the next one starts or the report ends. This holds
/// the parts until then.
struct Pending {
    /// The place that the diagnostic points at, or `None` for no place
    origin: Option<Origin>,

    /// The kind that tsc gave the diagnostic
    category: Category,

    /// The rule of TypeScript, as tsc numbers it
    number: String,

    /// What tsc said about the diagnostic, so far
    text: String,
}

impl Pending {
    /// Adds a line that tsc wrote to explain the diagnostic
    // checktypescript[impl check.elaboration]
    fn explain(&mut self, line: &str) {
        self.text.push(JOIN);
        self.text.push_str(line);
    }

    /// Returns the diagnostic that the collected lines describe
    fn finish(self) -> Diagnostic {
        Diagnostic::builder()
            .maybe_origin(self.origin)
            .category(self.category)
            .number(self.number)
            .text(self.text)
            .build()
    }
}

/// Returns the parts of a diagnostic that tsc reported about a place
///
/// Returns `None` when the text in front of the kind is no place, which
/// happens for a message of tsc that holds the text that closes a place.
fn located(place: &str, rest: &str) -> Option<Pending> {
    let (path, position) = place.rsplit_once(POSITION_OPEN)?;
    let (line, column) = position.split_once(POSITION_SEPARATOR)?;

    let origin = Origin::builder()
        .path(PathBuf::from(path))
        .line(line.parse().ok()?)
        .column(column.parse().ok()?)
        .build();

    stated(Some(origin), rest)
}

/// Returns the parts of a diagnostic that the given text states
///
/// The text holds the kind, the number, and what tsc said, in that order.
/// Returns `None` when it holds something else, which means that the line
/// starts no diagnostic.
fn stated(origin: Option<Origin>, text: &str) -> Option<Pending> {
    let (kind, rest) = text.split_once(KIND_CLOSE)?;
    let category = Category::parse(kind)?;
    let (number, said) = rest.split_once(NUMBER_CLOSE)?;

    Some(Pending {
        origin,
        category,
        number: number.to_owned(),
        text: said.to_owned(),
    })
}

/// Returns the diagnostic that the given line starts
///
/// A diagnostic that tsc found in a file names the place in front of the kind,
/// and one about the run itself names none. A path can hold the text that
/// closes a place, so a line whose place does not read as one is read as a
/// diagnostic without a place.
///
/// Returns `None` when the line starts no diagnostic at all.
// checktypescript[impl check.diagnostic]
// checktypescript[impl check.project]
fn start(line: &str) -> Option<Pending> {
    if let Some((place, rest)) = line.split_once(PLACE_CLOSE)
        && let Some(pending) = located(place, rest)
    {
        return Some(pending);
    }

    stated(None, line)
}

/// Returns the error of a line that is no part of a diagnostic
// checktypescript[impl check.unreadable]
fn unreadable(line: &str) -> ReadReportError {
    ReadReportError::UnreadableLine {
        line: line.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;

    /// What tsc writes about a project whose code breaks two rules
    const TWO: &str = "src/index.ts(2,14): error TS2322: Type 'string' is not assignable to type \
                       'number'.\nsrc/other.ts(1,1): error TS2307: Cannot find module './nowhere' \
                       or its corresponding type declarations.\n";

    /// What tsc writes about a diagnostic that it explains further
    const EXPLAINED: &str = "src/index.ts(2,14): error TS2322: Type '(a: number) => void' is not \
                             assignable to type 'F'.\n  Types of parameters 'a' and 'a' are \
                             incompatible.\n    Type 'string' is not assignable to type \
                             'number'.\n";

    /// What tsc writes about a project that it found no configuration for
    const UNCONFIGURED: &str = "error TS5081: Cannot find a tsconfig.json file at the current \
                                directory: /home/otter/project/tsconfig.json.\n";

    #[test]
    fn read_of_a_diagnostic_that_names_a_file_reports_its_place() {
        let diagnostics = read(TWO).unwrap();

        assert_eq!(
            diagnostics[0].origin(),
            &Some(
                Origin::builder()
                    .path(path("src/index.ts"))
                    .line(2)
                    .column(14)
                    .build()
            )
        );
    }

    #[test]
    fn read_of_a_diagnostic_that_names_no_file_reports_no_place() {
        let diagnostics = read(UNCONFIGURED).unwrap();

        assert_eq!(diagnostics[0].origin(), &None);
    }

    #[test]
    fn read_of_a_diagnostic_that_tsc_explains_joins_the_explanation() {
        let diagnostics = read(EXPLAINED).unwrap();

        assert_eq!(
            diagnostics[0].text(),
            "Type '(a: number) => void' is not assignable to type 'F'. Types of parameters 'a' \
             and 'a' are incompatible. Type 'string' is not assignable to type 'number'."
        );
    }

    #[test]
    fn read_of_a_diagnostic_that_tsc_explains_reports_one_diagnostic() {
        let diagnostics = read(EXPLAINED).unwrap();

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn read_of_a_line_that_starts_no_diagnostic_fails() {
        let error = read("tsc: The TypeScript Compiler - Version 7.0.2\n").unwrap_err();

        assert_eq!(
            error,
            ReadReportError::UnreadableLine {
                line: "tsc: The TypeScript Compiler - Version 7.0.2".to_owned(),
            }
        );
    }

    #[test]
    fn read_of_an_empty_report_finds_no_diagnostic() {
        let diagnostics = read("").unwrap();

        assert_eq!(diagnostics, Vec::new());
    }

    #[test]
    fn read_of_an_explanation_without_a_diagnostic_fails() {
        let error = read("  Types of parameters 'a' and 'a' are incompatible.\n").unwrap_err();

        assert!(
            matches!(error, ReadReportError::UnreadableLine { .. }),
            "expected the line to be unreadable, got {error:?}"
        );
    }

    #[test]
    fn read_of_two_diagnostics_reports_both() {
        let diagnostics = read(TWO).unwrap();

        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn read_reports_the_kind_that_tsc_gave_a_diagnostic() {
        let diagnostics = read(TWO).unwrap();

        assert_eq!(diagnostics[0].category(), &Category::Error);
    }

    #[test]
    fn read_reports_the_number_that_tsc_gave_a_diagnostic() {
        let diagnostics = read(TWO).unwrap();

        assert_eq!(diagnostics[0].number(), "TS2322");
    }

    #[test]
    fn read_reports_what_tsc_said_about_a_diagnostic() {
        let diagnostics = read(TWO).unwrap();

        assert_eq!(
            diagnostics[0].text(),
            "Type 'string' is not assignable to type 'number'."
        );
    }
}
