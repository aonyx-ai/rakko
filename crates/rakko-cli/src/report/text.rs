use std::fmt;

use rakko_action::{Finding, Outcome};

use super::Report;

/// Writes a report as the text that a reader at a terminal gets
///
/// A finding takes one line, and that line starts with the location that the
/// finding names. One line is what a finding of any granularity can produce,
/// and it is the form that a reader greps and that an editor jumps to.
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what the
/// report writes.
pub(super) fn render(report: &Report, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let action = &report.action;

    match &report.outcome {
        Outcome::Passed => write!(formatter, "{action}: passed"),
        Outcome::Skipped { reason } => write!(formatter, "{action}: skipped, {reason}"),
        Outcome::Errored { source } => {
            write!(formatter, "{action}: {source}")?;

            // The Display of an error reports its own layer only, and the
            // layer that a reader needs is usually the innermost one, so the
            // line carries the whole chain.
            let mut cause = source.source();
            while let Some(error) = cause {
                write!(formatter, ": {error}")?;
                cause = error.source();
            }

            Ok(())
        }
        Outcome::Failed { findings } => {
            for finding in findings {
                render_finding(finding, formatter)?;
            }

            let count = findings.len();
            let noun = if count == 1 { "finding" } else { "findings" };

            write!(formatter, "{action}: {count} {noun}")
        }
    }
}

/// Writes one finding as the line that names where the problem is
///
/// The line names as much of the location as the finding carries. A finding
/// that names no position gives the path alone, and a finding that names a
/// line without a column gives the line alone.
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what the
/// finding writes.
fn render_finding(finding: &Finding, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let location = finding.location();
    let message = finding.message();

    write!(formatter, "{}", location.path())?;

    if let Some(position) = location.position() {
        write!(formatter, ":{}", position.line())?;

        if let Some(column) = position.column() {
            write!(formatter, ":{column}")?;
        }
    }

    writeln!(formatter, ": {message}")
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{FilePath, Location, Position, SkipReason};

    use super::*;

    /// Returns a finding for the given path, position, and message
    fn finding(path: &str, position: Option<Position>, message: &str) -> Finding {
        let location = Location::builder()
            .path(FilePath::try_from(path).expect("the test names a relative path"))
            .maybe_position(position)
            .build();

        Finding::builder()
            .message(message)
            .location(location)
            .build()
    }

    /// Returns the text that a run of the action `probe` reports
    fn rendered(outcome: Outcome) -> String {
        Report::new("probe".parse().expect("the test names an action"), outcome).to_string()
    }

    // cli[verify report.errored]
    #[test]
    fn render_errored_outcome_reports_the_error() {
        let text = rendered(Outcome::Errored {
            source: Box::new(std::io::Error::other("failed to read Cargo.toml")),
        });

        assert_eq!(text, "probe: failed to read Cargo.toml");
    }

    // cli[verify report.findings]
    #[test]
    fn render_failed_outcome_reports_every_finding_with_its_location() {
        let text = rendered(Outcome::Failed {
            findings: vec![
                finding(
                    "deny.toml",
                    Some(Position::builder().line(3).column(1).build()),
                    "the license is not allowlisted",
                ),
                finding("Cargo.toml", None, "the file is not formatted"),
            ],
        });

        assert_eq!(
            text,
            "deny.toml:3:1: the license is not allowlisted\n\
             Cargo.toml: the file is not formatted\n\
             probe: 2 findings"
        );
    }

    // cli[verify report.findings]
    #[test]
    fn render_failed_outcome_with_one_finding_reports_it_in_the_singular() {
        let text = rendered(Outcome::Failed {
            findings: vec![finding("Cargo.toml", None, "the file is not formatted")],
        });

        assert!(text.ends_with("probe: 1 finding"));
    }

    // cli[verify report.findings]
    #[test]
    fn render_finding_without_a_column_reports_the_line() {
        let text = rendered(Outcome::Failed {
            findings: vec![finding(
                "README.md",
                Some(Position::builder().line(7).build()),
                "the line is too long",
            )],
        });

        assert!(text.starts_with("README.md:7: the line is too long"));
    }

    #[test]
    fn render_passed_outcome_reports_that_the_action_passed() {
        let text = rendered(Outcome::Passed);

        assert_eq!(text, "probe: passed");
    }

    // cli[verify report.skipped]
    #[test]
    fn render_skipped_outcome_reports_the_reason() {
        let text = rendered(Outcome::Skipped {
            reason: SkipReason::new("this project has no TOML file"),
        });

        assert_eq!(text, "probe: skipped, this project has no TOML file");
    }
}
