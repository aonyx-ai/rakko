use std::path::PathBuf;

use rakko_action::{Position, Span};
use serde::Deserialize;

use crate::problem::{Severity, ZizmorProblem};

/// The distance between a point of zizmor and a position of a finding
///
/// Zizmor counts rows and columns from zero, and a position of Rakko counts
/// lines and columns from one. The report of zizmor and the block that it
/// draws for a reader therefore disagree by this much, and the reading closes
/// the gap so that a finding names the line that an editor shows.
const ORIGIN: u32 = 1;

/// One finding of the report that zizmor wrote
///
/// Zizmor writes more than these fields, and the reading ignores the rest, so
/// a field that a new version adds does not break it.
#[derive(Deserialize)]
struct Record {
    /// The name of the audit that recognized the pattern
    ident: String,

    /// What the audit is about, in the words of zizmor
    desc: String,

    /// How zizmor weighed the pattern, and how sure it is about it
    determinations: Determinations,

    /// The places of the workflow that the finding is about
    locations: Vec<Location>,
}

impl Record {
    /// Returns one problem for each place that the finding names
    ///
    /// Zizmor draws every place of a finding in one block of source, which an
    /// outcome of Rakko has no place for, so each place becomes a problem of
    /// its own. The audit and the severity travel with each of them, so that
    /// a reader sees which problems came from the same finding.
    // lintgithubactions[impl check.finding]
    fn into_problems(self) -> Vec<ZizmorProblem> {
        let severity = self.determinations.severity;

        self.locations
            .into_iter()
            .map(|location| {
                ZizmorProblem::new(
                    location.symbolic.key.local.verbatim_path,
                    span(&location.concrete.location),
                    severity,
                    self.ident.clone(),
                    self.desc.clone(),
                    location.symbolic.annotation,
                )
            })
            .collect()
    }
}

/// How zizmor weighed a finding
#[derive(Deserialize)]
struct Determinations {
    /// How much the finding matters
    severity: Severity,
}

/// One place of a workflow that a finding is about
#[derive(Deserialize)]
struct Location {
    /// The file of the place, and what zizmor recognized in it
    symbolic: Symbolic,

    /// Where in the file the place is
    concrete: Concrete,
}

/// The file of a place, and what zizmor recognized in it
#[derive(Deserialize)]
struct Symbolic {
    /// The file that the place is in
    key: Key,

    /// What zizmor wrote about this place of the finding
    annotation: String,
}

/// The file that a place is in
///
/// Zizmor audits a repository of a code host as well as a checkout on disk,
/// and it names the file of a remote repository differently. A run of this
/// action names a directory of the machine, so every file arrives as a local
/// one, and a report that names another kind stops the reading.
#[derive(Deserialize)]
struct Key {
    /// The file below the place that the run named
    #[serde(rename = "Local")]
    local: LocalKey,
}

/// The file of a place, below the place that the run named
#[derive(Deserialize)]
struct LocalKey {
    /// The path of the file, as zizmor wrote it
    verbatim_path: PathBuf,
}

/// Where in the file a place is
#[derive(Deserialize)]
struct Concrete {
    /// The range of the file that the place covers
    location: Range,
}

/// The range of a file that a place covers
#[derive(Deserialize)]
struct Range {
    /// The point where the range starts
    start_point: Point,

    /// The point where the range ends
    end_point: Point,
}

/// One point of a file, counted from zero
#[derive(Deserialize)]
struct Point {
    /// The row of the point, starting at 0
    row: u32,

    /// The column of the point, starting at 0
    column: u32,
}

impl Point {
    /// Returns the position of a finding that this point names
    fn position(&self) -> Position {
        Position::builder()
            .line(self.row.saturating_add(ORIGIN))
            .column(self.column.saturating_add(ORIGIN))
            .build()
    }
}

/// Returns the problems that a report of zizmor holds
///
/// A run that reported nothing writes an empty array, and a run that stopped
/// before it audited anything writes nothing at all. Both hold no problem.
///
/// # Errors
///
/// Returns the error of a report that is not the array that zizmor writes. A
/// reading that tolerated such a report would let a run pass while the
/// findings of the run went unread.
// lintgithubactions[impl check.finding]
pub(super) fn problems(report: &str) -> Result<Vec<ZizmorProblem>, serde_json::Error> {
    let text = report.trim();

    if text.is_empty() {
        return Ok(Vec::new());
    }

    let records: Vec<Record> = serde_json::from_str(text)?;

    Ok(records
        .into_iter()
        .flat_map(Record::into_problems)
        .collect())
}

/// Returns the range of a finding that a range of zizmor covers
fn span(range: &Range) -> Span {
    Span::builder()
        .start(range.start_point.position())
        .end(range.end_point.position())
        .build()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a report
    // which zizmor could have written expects the reading to succeed. A
    // `# Panics` section on every test would repeat that and give the reader
    // no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A finding that names one place of a workflow
    const SINGLE: &str = r#"[{"ident":"unpinned-uses","desc":"unpinned action reference","url":"https://example.invalid/#unpinned-uses","determinations":{"confidence":"High","severity":"High","persona":"Regular"},"locations":[{"symbolic":{"key":{"Local":{"prefix":"/home/otter/project","given_path":".","verbatim_path":"./.github/workflows/ci.yml"}},"annotation":"action is not pinned to a hash","route":{"route":[]},"feature_kind":"Normal","kind":"Primary"},"concrete":{"location":{"start_point":{"row":6,"column":14},"end_point":{"row":6,"column":33},"offset_span":{"start":88,"end":107}},"feature":"actions/checkout@v4","comments":[]}}],"ignored":false,"fixes":[]}]"#;

    /// A finding that names two places of a workflow
    const PAIRED: &str = r#"[{"ident":"template-injection","desc":"code injection via template expansion","url":"https://example.invalid/#template-injection","determinations":{"confidence":"High","severity":"High","persona":"Regular"},"locations":[{"symbolic":{"key":{"Local":{"verbatim_path":"./.github/workflows/ci.yml"}},"annotation":"this step","route":{"route":[]},"feature_kind":"Normal","kind":"Hidden"},"concrete":{"location":{"start_point":{"row":7,"column":8},"end_point":{"row":8,"column":0},"offset_span":{"start":116,"end":160}},"feature":"run: echo","comments":[]}},{"symbolic":{"key":{"Local":{"verbatim_path":"./.github/workflows/ci.yml"}},"annotation":"may expand into attacker-controllable code","route":{"route":[]},"feature_kind":"Normal","kind":"Primary"},"concrete":{"location":{"start_point":{"row":7,"column":23},"end_point":{"row":7,"column":47},"offset_span":{"start":131,"end":155}},"feature":"echo","comments":[]}}],"ignored":false,"fixes":[]}]"#;

    /// A finding whose file lives in a repository of a code host
    const REMOTE: &str = r#"[{"ident":"unpinned-uses","desc":"unpinned action reference","url":"https://example.invalid/#unpinned-uses","determinations":{"confidence":"High","severity":"High","persona":"Regular"},"locations":[{"symbolic":{"key":{"Remote":{"slug":"otter/project","path":".github/workflows/ci.yml"}},"annotation":"action is not pinned to a hash","route":{"route":[]},"feature_kind":"Normal","kind":"Primary"},"concrete":{"location":{"start_point":{"row":6,"column":14},"end_point":{"row":6,"column":33},"offset_span":{"start":88,"end":107}},"feature":"actions/checkout@v4","comments":[]}}],"ignored":false,"fixes":[]}]"#;

    /// What zizmor writes when it ends without a report
    const DIAGNOSIS: &str = "error: configuration error in .";

    /// Returns the single problem of a report that holds one
    fn problem(report: &str) -> ZizmorProblem {
        let mut problems =
            problems(report).expect("the test reads a report that zizmor could write");

        problems
            .pop()
            .expect("the test reads a report that holds one problem")
    }

    // lintgithubactions[verify check.finding]
    #[test]
    fn problems_of_a_finding_carry_the_words_of_zizmor() {
        let problem = problem(SINGLE);

        assert_eq!(
            problem.message(),
            "[high] unpinned-uses: unpinned action reference \
             (action is not pinned to a hash)"
        );
    }

    // lintgithubactions[verify check.finding]
    #[test]
    fn problems_of_a_finding_cover_the_range_of_zizmor() {
        let problem = problem(SINGLE);

        assert_eq!(
            problem.span(),
            Span::builder()
                .start(Position::builder().line(7).column(15).build())
                .end(Position::builder().line(7).column(34).build())
                .build()
        );
    }

    // lintgithubactions[verify check.finding]
    #[test]
    fn problems_of_a_finding_name_the_file() {
        let problem = problem(SINGLE);

        assert_eq!(problem.path(), &PathBuf::from("./.github/workflows/ci.yml"));
    }

    // lintgithubactions[verify check.finding]
    #[test]
    fn problems_of_a_finding_with_two_places_hold_one_problem_each() {
        let problems = problems(PAIRED).expect("the test reads a report that zizmor could write");

        assert_eq!(
            problems
                .iter()
                .map(ZizmorProblem::annotation)
                .collect::<Vec<_>>(),
            ["this step", "may expand into attacker-controllable code"]
        );
    }

    // lintgithubactions[verify check.passed]
    #[test]
    fn problems_of_an_empty_report_hold_nothing() {
        let problems = problems("[]").expect("the test reads the report of a run that passed");

        assert!(problems.is_empty(), "expected no problem, got {problems:?}");
    }

    // lintgithubactions[verify check.unreadable]
    #[test]
    fn problems_of_a_report_that_is_not_a_report_stop_the_reading() {
        let problems = problems(DIAGNOSIS);

        assert!(
            problems.is_err(),
            "expected the reading to stop, got {problems:?}"
        );
    }

    // lintgithubactions[verify check.unreadable]
    #[test]
    fn problems_of_a_finding_in_a_remote_repository_stop_the_reading() {
        let problems = problems(REMOTE);

        assert!(
            problems.is_err(),
            "expected the reading to stop, got {problems:?}"
        );
    }
}
