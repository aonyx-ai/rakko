//! The report that taplo writes about a format run
//!
//! Taplo reports a format run as text on its standard error stream, and this
//! module reads that text. The parse recognizes the lines that carry an
//! answer — a rejected configuration, the count of the files, a file that is
//! not formatted, and a diagnostic with a position — and ignores everything
//! else, so a new log line does not break the parse. What the parse cannot
//! find, the caller treats as a report that it does not recognize.

use std::path::PathBuf;

use getset::{CopyGetters, Getters};

use super::problem::{ProblemDetail, TaploProblem};

/// The marker of the line that reports a rejected configuration file
///
/// Taplo warns about a configuration that it cannot read and then runs with
/// its defaults, so this line is the only trace of the rejection.
const CONFIG_REJECTED: &str = "invalid configuration file";

/// The marker of the line that counts the files of a run
const FILES_FOUND: &str = "collect_files: found files";

/// The field of the count line that holds every file that taplo matched
const TOTAL_FIELD: &str = "total=";

/// The field of the count line that holds the files that the configuration
/// excluded
const EXCLUDED_FIELD: &str = "excluded=";

/// The marker of the line that reports a file that is not formatted
const UNFORMATTED: &str = "the file is not properly formatted";

/// The field that carries the path of a file that is not formatted
const PATH_FIELD: &str = "path=\"";

/// The field that carries the detail of a rejected configuration
const ERROR_FIELD: &str = "error=";

/// The prefix of the header line of a diagnostic
const DIAGNOSTIC: &str = "error: ";

/// The marker of the line that places a diagnostic in a file
const LOCATION_ARROW: &str = "┌─ ";

/// The marker of the line that closes the report of a failed run
///
/// A run that ends without success writes this line last, so a report that
/// ends without it lost its tail.
const FAILURE_SUMMARY: &str = "operation failed";

/// What taplo reported about one run
///
/// The report holds only what the parse found. A field that is `None` means
/// that the corresponding line did not appear, and the caller decides what
/// its absence means for the outcome of the run.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub(super) struct TaploReport {
    /// What taplo said about a configuration file that it rejected
    #[getset(get = "pub(super)")]
    rejected_configuration: Option<String>,

    /// How many files taplo checked, after its configuration excluded
    ///
    /// Taplo counts every file that it matched and the files that the
    /// configuration excluded separately, and the difference is what a run
    /// examined.
    #[getset(get_copy = "pub(super)")]
    checked: Option<u64>,

    /// Whether the report closes with the summary of a failed run
    ///
    /// A run that ends without success sums its failure up on its last
    /// line. A report of such a run without this line lost its tail, and
    /// the problems that it holds can be incomplete.
    #[getset(get_copy = "pub(super)")]
    failure_reported: bool,

    /// The problems that taplo reported, in the order of the report
    #[getset(get = "pub(super)")]
    problems: Vec<TaploProblem>,
}

/// Reads the report of one taplo run from its standard error stream
///
/// The parse walks the report line by line. A diagnostic spans several
/// lines, and the parse looks ahead from its header without consuming the
/// block, because the lines of a block match no other rule.
pub(super) fn parse(stderr: &str) -> TaploReport {
    let mut report = TaploReport {
        rejected_configuration: None,
        checked: None,
        failure_reported: false,
        problems: Vec::new(),
    };
    let lines: Vec<&str> = stderr.lines().collect();

    for (index, line) in lines.iter().enumerate() {
        if line.contains(CONFIG_REJECTED) {
            if report.rejected_configuration.is_none() {
                report.rejected_configuration = Some(rejection(line));
            }
        } else if line.contains(FAILURE_SUMMARY) {
            report.failure_reported = true;
        } else if line.contains(FILES_FOUND) {
            report.checked = checked(line);
        } else if line.contains(UNFORMATTED) {
            if let Some(path) = quoted_path(line) {
                report
                    .problems
                    .push(TaploProblem::new(path, ProblemDetail::Unformatted));
            }
        } else if let Some(header) = line.strip_prefix(DIAGNOSTIC)
            && let Some(problem) = diagnostic(header, &lines[index + 1..])
        {
            report.problems.push(problem);
        }
    }

    report
}

/// Returns the label that a caret line of a diagnostic carries
///
/// The excerpt of a diagnostic draws the offending text and then a line of
/// carets with a label, such as `^ unexpected EOF`. The caret line starts
/// with the gutter of the excerpt and no line number, which is how it
/// differs from a line of the offending text itself.
fn caret_label(line: &str) -> Option<String> {
    let content = line.trim_start().strip_prefix('│')?;
    let start = content.find('^')?;
    let label = content[start..].trim_start_matches('^').trim();

    (!label.is_empty()).then(|| label.to_owned())
}

/// Returns how many files a run checked, from the line that counts them
fn checked(line: &str) -> Option<u64> {
    let total = field(line, TOTAL_FIELD)?;
    let excluded = field(line, EXCLUDED_FIELD)?;

    total.checked_sub(excluded)
}

/// Returns the coordinates of a diagnostic, from the text after its arrow
///
/// The text reads `path:line:column`, and the path can itself hold the
/// separator, so the parse takes the two numbers from the right.
fn coordinates(location: &str) -> Option<(PathBuf, u32, u32)> {
    let mut parts = location.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    let path = parts.next()?;

    (!path.is_empty()).then(|| (PathBuf::from(path), line, column))
}

/// Returns the problem that a diagnostic block describes
///
/// The header carries the message, the arrow line that follows carries the
/// position, and the caret line of the excerpt can carry a label that makes
/// the message precise. A block without a position describes no file, and
/// the parse drops it, because the caller reports what it cannot read
/// through the exit status of the run.
fn diagnostic(header: &str, following: &[&str]) -> Option<TaploProblem> {
    let arrow = following
        .iter()
        .take(2)
        .find(|line| line.contains(LOCATION_ARROW))?;
    let location = arrow.split_once(LOCATION_ARROW)?.1.trim();
    let (path, line, column) = coordinates(location)?;

    let label = following
        .iter()
        .take_while(|line| !line.trim().is_empty())
        .find_map(|line| caret_label(line));
    let message = match label {
        Some(label) => format!("{}: {}", header.trim(), label),
        None => header.trim().to_owned(),
    };

    Some(TaploProblem::new(
        path,
        ProblemDetail::Diagnostic {
            line,
            column,
            message,
        },
    ))
}

/// Returns the number that a field of the count line carries
fn field(line: &str, name: &str) -> Option<u64> {
    let start = line.find(name)? + name.len();
    let rest = &line[start..];
    let end = rest
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(rest.len());

    rest[..end].parse().ok()
}

/// Returns the path that a report line quotes
fn quoted_path(line: &str) -> Option<PathBuf> {
    let start = line.find(PATH_FIELD)? + PATH_FIELD.len();
    let rest = &line[start..];
    let end = rest.rfind('"')?;
    let path = &rest[..end];

    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Returns what a rejection line says about the configuration file
///
/// The line carries the diagnosis of taplo behind its `error=` field, and
/// the whole line stands in when the field is missing, so the caller always
/// has something to show.
fn rejection(line: &str) -> String {
    line.split_once(ERROR_FIELD).map_or_else(
        || line.trim().to_owned(),
        |(_, detail)| detail.trim().to_owned(),
    )
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use PathBuf;

    use super::*;

    /// The report of a run that checked two files and found nothing
    const CLEAN: &str = " INFO taplo:format_files:collect_files: found files total=3 excluded=1 files=[\"/home/otter/project/a.toml\"] cwd=\"/home/otter/project\"\n";

    /// The report of a run over a file that taplo cannot parse
    const INVALID: &str = "error: invalid TOML\n  \u{250c}\u{2500} /home/otter/project/broken.toml:2:1\n  \u{2502}\n2 \u{2502} \n  \u{2502} ^ unexpected EOF\n\nERROR operation failed error=some files were not properly formatted\n";

    /// The report of a run whose configuration taplo rejected
    const REJECTED: &str = " WARN taplo:format_files:load_config: invalid configuration file error=TOML parse error at line 1, column 5\n";

    /// The report of a run over a file that is not formatted
    const UNFORMATTED: &str = "ERROR taplo:format_files: the file is not properly formatted path=\"/home/otter/project/sub/messy.toml\"\nERROR operation failed error=some files were not properly formatted\n";

    #[test]
    fn parse_clean_report_counts_the_checked_files() {
        let report = parse(CLEAN);

        assert_eq!(report.checked(), Some(2));
    }

    #[test]
    fn parse_clean_report_finds_no_problem() {
        let report = parse(CLEAN);

        assert!(report.problems().is_empty());
    }

    #[test]
    fn parse_diagnostic_reads_the_position() {
        let report = parse(INVALID);

        assert_eq!(
            report.problems(),
            &vec![TaploProblem::new(
                PathBuf::from("/home/otter/project/broken.toml"),
                ProblemDetail::Diagnostic {
                    line: 2,
                    column: 1,
                    message: "invalid TOML: unexpected EOF".to_owned(),
                },
            )]
        );
    }

    #[test]
    fn parse_diagnostic_without_a_label_keeps_the_header() {
        let report = parse("error: invalid TOML\n  \u{250c}\u{2500} /p/broken.toml:2:1\n");

        let Some(problem) = report.problems().first() else {
            panic!("expected a diagnostic");
        };
        let ProblemDetail::Diagnostic { message, .. } = problem.detail() else {
            panic!("expected a diagnostic, got {problem:?}");
        };
        assert_eq!(message, "invalid TOML");
    }

    #[test]
    fn parse_empty_report_finds_nothing() {
        let report = parse("");

        assert_eq!(
            report,
            TaploReport {
                rejected_configuration: None,
                checked: None,
                failure_reported: false,
                problems: Vec::new(),
            }
        );
    }

    #[test]
    fn parse_failed_run_sees_the_summary_of_the_failure() {
        let report = parse(UNFORMATTED);

        assert!(report.failure_reported());
    }

    #[test]
    fn parse_incomplete_report_sees_no_summary_of_a_failure() {
        let report = parse(
            "ERROR taplo:format_files: the file is not properly formatted path=\"/p/messy.toml\"\n",
        );

        assert!(!report.failure_reported());
    }

    #[test]
    fn parse_rejected_configuration_keeps_the_diagnosis() {
        let report = parse(REJECTED);

        assert_eq!(
            report.rejected_configuration().as_deref(),
            Some("TOML parse error at line 1, column 5")
        );
    }

    #[test]
    fn parse_unformatted_file_reads_the_path() {
        let report = parse(UNFORMATTED);

        assert_eq!(
            report.problems(),
            &vec![TaploProblem::new(
                PathBuf::from("/home/otter/project/sub/messy.toml"),
                ProblemDetail::Unformatted,
            )]
        );
    }

    #[test]
    fn parse_unrecognized_failure_finds_nothing() {
        let report = parse(
            " INFO taplo:format_files:collect_files: found files total=1 excluded=0 files=[] cwd=\"/p\"\nERROR operation failed error=Permission denied (os error 13)\n",
        );

        assert!(report.problems().is_empty());
    }
}
