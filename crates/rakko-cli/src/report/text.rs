use std::fmt;

use rakko_action::{Finding, Location, Outcome, Position};

use super::Report;

/// Writes a report as the text that a reader at a terminal gets
///
/// A finding takes one line, and that line starts with the place that the
/// finding names. One line is what a finding of any granularity can produce,
/// and it is the form that a reader greps and that an editor jumps to.
///
/// A repair takes the same line, because it is the problem that the run took
/// away. A run that repaired part of what it found writes its repairs first.
/// The problems that remain follow them, so that the lines a reader has to act
/// on sit next to the summary.
///
/// A pass shows what the run examined when the action said so, in the way
/// that a skip shows its reason, so a reader can question a pass that
/// examined less than they expect.
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what the
/// report writes.
pub(super) fn render(report: &Report, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let action = &report.action;

    match &report.outcome {
        // cli[impl report.passed]
        Outcome::Passed { summary } => match summary {
            Some(summary) => write!(formatter, "{action}: passed, {summary}"),
            None => write!(formatter, "{action}: passed"),
        },
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
        Outcome::Changed { repairs } => {
            render_findings(repairs, formatter)?;

            write!(formatter, "{action}: ")?;
            render_count(repairs.len(), "repair", formatter)
        }
        Outcome::Failed { findings, repairs } => {
            render_findings(repairs, formatter)?;
            render_findings(findings, formatter)?;

            write!(formatter, "{action}: ")?;
            render_count(findings.len(), "finding", formatter)?;

            if !repairs.is_empty() {
                write!(formatter, ", ")?;
                render_count(repairs.len(), "repair", formatter)?;
            }

            Ok(())
        }
    }
}

/// Writes each finding of a list on a line of its own
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what a
/// finding writes.
fn render_findings(findings: &[Finding], formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for finding in findings {
        render_finding(finding, formatter)?;
    }

    Ok(())
}

/// Writes how many of something a run reported, and the noun that names it
///
/// The noun stands in the singular, and a count of anything other than one
/// gets the plural of that noun.
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what the
/// count writes.
fn render_count(count: usize, noun: &str, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{count} {noun}")?;

    if count != 1 {
        write!(formatter, "s")?;
    }

    Ok(())
}

/// Writes one finding as the line that names where the problem is
///
/// The line names as much of the place as the level of the finding carries. A
/// finding about the project gives the message alone, because it has no path
/// to name. A finding about a directory or a file gives that path. A finding
/// at a position gives the path, the line, and the column that the position
/// carries.
///
/// A finding over a span gives the position where the range starts, and it
/// drops the position where the range ends. A terminal line that reads
/// `path:line:column` is what an editor jumps to, and the end of the range
/// has no place in it.
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what the
/// finding writes.
fn render_finding(finding: &Finding, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match finding.location() {
        Location::Project => {}
        Location::Directory { path } => write!(formatter, "{path}: ")?,
        Location::File { path } => write!(formatter, "{path}: ")?,
        Location::Position { path, position } => {
            write!(formatter, "{path}")?;
            render_position(position, formatter)?;
            write!(formatter, ": ")?;
        }
        Location::Span { path, span } => {
            write!(formatter, "{path}")?;
            render_position(span.start(), formatter)?;
            write!(formatter, ": ")?;
        }
    }

    writeln!(formatter, "{}", finding.message())
}

/// Writes the line, and the column, that a position names
///
/// The column follows the line only when the position carries one, so a tool
/// that reports a line alone does not get a column that nobody measured.
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what the
/// position writes.
fn render_position(position: &Position, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, ":{}", position.line())?;

    if let Some(column) = position.column() {
        write!(formatter, ":{column}")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{DirectoryPath, FilePath, SkipReason, Span, Summary};

    use super::*;

    /// Returns a finding for the given location and message
    fn finding(location: Location, message: &str) -> Finding {
        Finding::builder()
            .message(message)
            .location(location)
            .build()
    }

    /// Returns the path of a file that a test names
    fn file(path: &str) -> FilePath {
        FilePath::try_from(path).expect("the test names a relative path")
    }

    /// Returns the text that a run of the action `probe` reports
    fn rendered(outcome: Outcome) -> String {
        Report::new("probe".parse().expect("the test names an action"), outcome).to_string()
    }

    // cli[verify report.repairs]
    #[test]
    fn render_changed_outcome_reports_every_repair_with_its_location() {
        let text = rendered(Outcome::Changed {
            repairs: vec![
                finding(
                    Location::File {
                        path: file("deny.toml"),
                    },
                    "the file was not formatted",
                ),
                finding(
                    Location::File {
                        path: file("Cargo.toml"),
                    },
                    "the file was not formatted",
                ),
            ],
        });

        assert_eq!(
            text,
            "deny.toml: the file was not formatted\n\
             Cargo.toml: the file was not formatted\n\
             probe: 2 repairs"
        );
    }

    // cli[verify report.repairs]
    #[test]
    fn render_changed_outcome_with_one_repair_reports_it_in_the_singular() {
        let text = rendered(Outcome::Changed {
            repairs: vec![finding(
                Location::File {
                    path: file("deny.toml"),
                },
                "the file was not formatted",
            )],
        });

        assert!(text.ends_with("probe: 1 repair"));
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
                    Location::Position {
                        path: file("deny.toml"),
                        position: Position::builder().line(3).column(1).build(),
                    },
                    "the license is not allowlisted",
                ),
                finding(
                    Location::File {
                        path: file("Cargo.toml"),
                    },
                    "the file is not formatted",
                ),
            ],
            repairs: Vec::new(),
        });

        assert_eq!(
            text,
            "deny.toml:3:1: the license is not allowlisted\n\
             Cargo.toml: the file is not formatted\n\
             probe: 2 findings"
        );
    }

    // cli[verify report.repairs]
    #[test]
    fn render_failed_outcome_reports_the_repairs_before_the_findings() {
        let text = rendered(Outcome::Failed {
            findings: vec![finding(
                Location::File {
                    path: file("Cargo.lock"),
                },
                "the file is not formatted",
            )],
            repairs: vec![finding(
                Location::File {
                    path: file("deny.toml"),
                },
                "the file was not formatted",
            )],
        });

        assert_eq!(
            text,
            "deny.toml: the file was not formatted\n\
             Cargo.lock: the file is not formatted\n\
             probe: 1 finding, 1 repair"
        );
    }

    // cli[verify report.findings]
    #[test]
    fn render_failed_outcome_with_one_finding_reports_it_in_the_singular() {
        let text = rendered(Outcome::Failed {
            findings: vec![finding(
                Location::File {
                    path: file("Cargo.toml"),
                },
                "the file is not formatted",
            )],
            repairs: Vec::new(),
        });

        assert!(text.ends_with("probe: 1 finding"));
    }

    // cli[verify report.findings]
    #[test]
    fn render_finding_over_a_directory_reports_the_directory() {
        let text = rendered(Outcome::Failed {
            findings: vec![finding(
                Location::Directory {
                    path: DirectoryPath::try_from("crates/rakko")
                        .expect("the test names a relative path"),
                },
                "the directory has no specification",
            )],
            repairs: Vec::new(),
        });

        assert!(text.starts_with("crates/rakko: the directory has no specification"));
    }

    // cli[verify report.findings]
    #[test]
    fn render_finding_over_a_span_reports_the_start_of_the_range() {
        let text = rendered(Outcome::Failed {
            findings: vec![finding(
                Location::Span {
                    path: file("src/lib.rs"),
                    span: Span::builder()
                        .start(Position::builder().line(1).column(1).build())
                        .end(Position::builder().line(3).column(2).build())
                        .build(),
                },
                "the block is not formatted",
            )],
            repairs: Vec::new(),
        });

        assert!(text.starts_with("src/lib.rs:1:1: the block is not formatted"));
    }

    // cli[verify report.findings]
    #[test]
    fn render_finding_over_the_project_reports_the_message_alone() {
        let text = rendered(Outcome::Failed {
            findings: vec![finding(Location::Project, "the crate serde is banned")],
            repairs: Vec::new(),
        });

        assert!(text.starts_with("the crate serde is banned"));
    }

    // cli[verify report.findings]
    #[test]
    fn render_finding_without_a_column_reports_the_line() {
        let text = rendered(Outcome::Failed {
            findings: vec![finding(
                Location::Position {
                    path: file("README.md"),
                    position: Position::builder().line(7).build(),
                },
                "the line is too long",
            )],
            repairs: Vec::new(),
        });

        assert!(text.starts_with("README.md:7: the line is too long"));
    }

    #[test]
    fn render_passed_outcome_reports_that_the_action_passed() {
        let text = rendered(Outcome::Passed { summary: None });

        assert_eq!(text, "probe: passed");
    }

    // cli[verify report.passed]
    #[test]
    fn render_passed_outcome_reports_the_summary() {
        let text = rendered(Outcome::Passed {
            summary: Some(Summary::new("checked 0 files")),
        });

        assert_eq!(text, "probe: passed, checked 0 files");
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
