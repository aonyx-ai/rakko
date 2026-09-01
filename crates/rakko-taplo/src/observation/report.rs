use std::collections::HashSet;
use std::path::PathBuf;

use super::Observation;
use crate::problem::{ProblemDetail, TaploProblem};

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

/// The marker of the line that reports a file that taplo refused
///
/// The marker carries the field that follows it, so that the line of a
/// rejected configuration file, which reads almost the same, stays out.
const INVALID_FILE: &str = "invalid file error=";

/// The field that carries the path of a file that a line reports about
const PATH_FIELD: &str = "path=\"";

/// The field that carries the detail of a failure
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

/// Reads what a run of taplo produced
///
/// The reading walks the report line by line. A diagnostic spans several
/// lines, and the reading looks ahead from its header without consuming the
/// block, because the lines of a block match no other rule.
pub(super) fn read(stderr: &str, succeeded: bool) -> Observation {
    let mut observation = Observation {
        checked: None,
        failure_reported: false,
        problems: Vec::new(),
        rejected_configuration: None,
        stderr: stderr.to_owned(),
        succeeded,
    };
    let lines: Vec<&str> = observation.stderr.lines().collect();

    for (index, line) in lines.iter().enumerate() {
        // taplo[impl report.configuration]
        if line.contains(CONFIG_REJECTED) {
            if observation.rejected_configuration.is_none() {
                observation.rejected_configuration = Some(rejection(line));
            }
        // taplo[impl report.failure]
        } else if line.contains(FAILURE_SUMMARY) {
            observation.failure_reported = true;
        // taplo[impl report.checked]
        } else if line.contains(FILES_FOUND) {
            observation.checked = checked(line);
        // taplo[impl report.unformatted]
        } else if line.contains(UNFORMATTED) {
            if let Some(path) = quoted_path(line) {
                observation
                    .problems
                    .push(TaploProblem::new(path, ProblemDetail::Unformatted));
            }
        // taplo[impl report.invalid]
        } else if line.contains(INVALID_FILE) {
            if let Some(path) = quoted_path(line) {
                let detail = ProblemDetail::Invalid {
                    reason: reason(line),
                };

                observation.problems.push(TaploProblem::new(path, detail));
            }
        // taplo[impl report.diagnostic]
        } else if let Some(header) = line.strip_prefix(DIAGNOSTIC)
            && let Some(problem) = diagnostic(header, &lines[index + 1..])
        {
            observation.problems.push(problem);
        }
    }

    summarized(&mut observation.problems);

    observation
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
/// separator, so the reading takes the two numbers from the right.
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
/// the reading drops it, because the caller reports what it cannot read
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

/// Returns what a line about a refused file says about the file
///
/// The reason stands between the field that carries it and the path that
/// closes the line, and the whole line stands in when the reading finds no
/// such text, so the caller always has something to show.
fn reason(line: &str) -> String {
    let detail = line
        .split_once(INVALID_FILE)
        .map(|(_, detail)| detail)
        .and_then(|detail| detail.split_once(PATH_FIELD))
        .map(|(reason, _)| reason.trim());

    detail
        .filter(|reason| !reason.is_empty())
        .unwrap_or_else(|| line.trim())
        .to_owned()
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

/// Drops the summary of a file that a diagnostic already describes
///
/// Taplo closes the diagnostics of a file with a line that names the file
/// and sums up why it refused it. The summary repeats what the diagnostics
/// said, at a level that says less, so it survives only for a file that got
/// no diagnostic at all — a file that taplo could not open, where the
/// summary is the whole answer.
// taplo[impl report.summarized]
fn summarized(problems: &mut Vec<TaploProblem>) {
    let positioned: HashSet<PathBuf> = problems
        .iter()
        .filter(|problem| matches!(problem.detail(), ProblemDetail::Diagnostic { .. }))
        .map(|problem| problem.path().clone())
        .collect();

    problems.retain(|problem| {
        !matches!(problem.detail(), ProblemDetail::Invalid { .. })
            || !positioned.contains(problem.path())
    });
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// The report of a run that examined two files and found nothing
    const CLEAN: &str = " INFO taplo:lint_files:collect_files: found files total=3 excluded=1 files=[\"/home/otter/project/a.toml\"] cwd=\"/home/otter/project\"\n";

    /// The report of a run over a file that taplo cannot parse
    const INVALID: &str = "error: invalid TOML\n  \u{250c}\u{2500} /home/otter/project/broken.toml:2:1\n  \u{2502}\n2 \u{2502} \n  \u{2502} ^ unexpected EOF\n\nERROR taplo:lint_files: invalid file error=syntax errors found path=\"/home/otter/project/broken.toml\"\nERROR operation failed error=some files were not valid\n";

    /// The report of a run over a file that taplo cannot open
    const REFUSED: &str = "ERROR taplo:lint_files: invalid file error=Permission denied (os error 13) path=\"/home/otter/project/secret.toml\"\nERROR operation failed error=some files were not valid\n";

    /// The report of a run whose configuration taplo rejected
    const REJECTED: &str = " WARN taplo:format_files:load_config: invalid configuration file error=TOML parse error at line 1, column 5\n";

    /// The report of a run over a file that is not formatted
    const UNFORMATTED: &str = "ERROR taplo:format_files: the file is not properly formatted path=\"/home/otter/project/sub/messy.toml\"\nERROR operation failed error=some files were not properly formatted\n";

    /// Returns the detail of the only problem of a report
    fn detail(stderr: &str) -> ProblemDetail {
        let observation = read(stderr, false);

        let [problem] = observation.problems().as_slice() else {
            panic!("expected one problem, got {:?}", observation.problems());
        };

        problem.detail().clone()
    }

    // taplo[verify report.checked]
    #[test]
    fn read_clean_report_counts_the_examined_files() {
        let observation = read(CLEAN, true);

        assert_eq!(observation.checked(), Some(2));
    }

    #[test]
    fn read_clean_report_finds_no_problem() {
        let observation = read(CLEAN, true);

        assert!(observation.problems().is_empty());
    }

    // taplo[verify report.diagnostic]
    #[test]
    fn read_diagnostic_reads_the_position() {
        let observation = read(INVALID, false);

        assert_eq!(
            observation.problems().first().map(TaploProblem::detail),
            Some(&ProblemDetail::Diagnostic {
                line: 2,
                column: 1,
                message: "invalid TOML: unexpected EOF".to_owned(),
            })
        );
    }

    // taplo[verify report.diagnostic]
    #[test]
    fn read_diagnostic_reads_the_path() {
        let observation = read(INVALID, false);

        assert_eq!(
            observation.problems().first().map(TaploProblem::path),
            Some(&PathBuf::from("/home/otter/project/broken.toml"))
        );
    }

    #[test]
    fn read_diagnostic_without_a_label_keeps_the_header() {
        let detail = detail("error: invalid TOML\n  \u{250c}\u{2500} /p/broken.toml:2:1\n");

        let ProblemDetail::Diagnostic { message, .. } = detail else {
            panic!("expected a diagnostic, got {detail:?}");
        };
        assert_eq!(message, "invalid TOML");
    }

    #[test]
    fn read_empty_report_finds_nothing() {
        let observation = read("", true);

        assert_eq!(
            observation,
            Observation {
                checked: None,
                failure_reported: false,
                problems: Vec::new(),
                rejected_configuration: None,
                stderr: String::new(),
                succeeded: true,
            }
        );
    }

    // taplo[verify report.failure]
    #[test]
    fn read_failed_run_sees_the_summary_of_the_failure() {
        let observation = read(UNFORMATTED, false);

        assert!(observation.failure_reported());
    }

    // taplo[verify report.failure]
    #[test]
    fn read_incomplete_report_sees_no_summary_of_a_failure() {
        let observation = read(
            "ERROR taplo:format_files: the file is not properly formatted path=\"/p/messy.toml\"\n",
            false,
        );

        assert!(!observation.failure_reported());
    }

    // taplo[verify report.invalid]
    #[test]
    fn read_refused_file_keeps_the_reason_of_taplo() {
        let detail = detail(REFUSED);

        assert_eq!(
            detail,
            ProblemDetail::Invalid {
                reason: "Permission denied (os error 13)".to_owned(),
            }
        );
    }

    // taplo[verify report.invalid]
    #[test]
    fn read_refused_file_reads_the_path() {
        let observation = read(REFUSED, false);

        assert_eq!(
            observation.problems().first().map(TaploProblem::path),
            Some(&PathBuf::from("/home/otter/project/secret.toml"))
        );
    }

    // taplo[verify report.configuration]
    #[test]
    fn read_rejected_configuration_keeps_the_diagnosis() {
        let observation = read(REJECTED, true);

        assert_eq!(
            observation.rejected_configuration().as_deref(),
            Some("TOML parse error at line 1, column 5")
        );
    }

    // taplo[verify report.summarized]
    #[test]
    fn read_summary_of_a_file_with_a_diagnostic_drops_out() {
        let observation = read(INVALID, false);

        assert_eq!(observation.problems().len(), 1);
    }

    // taplo[verify report.unformatted]
    #[test]
    fn read_unformatted_file_reads_the_path() {
        let observation = read(UNFORMATTED, false);

        assert_eq!(
            observation.problems(),
            &vec![TaploProblem::new(
                PathBuf::from("/home/otter/project/sub/messy.toml"),
                ProblemDetail::Unformatted,
            )]
        );
    }

    #[test]
    fn read_unrecognized_failure_finds_nothing() {
        let observation = read(
            " INFO taplo:format_files:collect_files: found files total=1 excluded=0 files=[] cwd=\"/p\"\nERROR operation failed error=Permission denied (os error 13)\n",
            false,
        );

        assert!(observation.problems().is_empty());
    }
}
