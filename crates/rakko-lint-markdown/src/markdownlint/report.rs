use std::path::PathBuf;

use serde::Deserialize;

use crate::problem::MarkdownlintProblem;

/// The separator between the name of a rule and its aliases
///
/// Markdownlint joins them this way when it writes its report for a reader,
/// and a message that a contributor can search for reads the same either way.
const ALIAS_SEPARATOR: &str = "/";

/// One result of the report that markdownlint wrote
///
/// Markdownlint writes more than these fields, and the reading ignores the
/// rest, so a field that a new version adds does not break it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    /// The path of the file, relative to where markdownlint started
    file_name: PathBuf,

    /// The line that the rule was broken on, starting at 1
    line_number: u32,

    /// The name of the rule and its aliases, the name first
    rule_names: Vec<String>,

    /// What the rule is about, in the words of markdownlint
    rule_description: String,

    /// The text that broke the rule, when the rule quotes it
    error_context: Option<String>,

    /// What the rule expected here, when the rule says so
    error_detail: Option<String>,

    /// The column of the rule and the length of what it points at
    ///
    /// A rule that speaks about the whole line reports nothing here.
    error_range: Option<Vec<u32>>,
}

impl Record {
    /// Returns the problem that this result describes
    fn into_problem(self) -> MarkdownlintProblem {
        let message = message(&self);
        let column = self
            .error_range
            .as_ref()
            .and_then(|range| range.first().copied())
            .filter(|column| *column > 0);

        MarkdownlintProblem::new(self.file_name, self.line_number, column, message)
    }
}

/// Returns the sentence that markdownlint would have written for a reader
///
/// Markdownlint builds it from the rule, what the rule is about, what it
/// expected, and the text that broke it, and it leaves out the parts that a
/// rule does not report. Building the same sentence here means that a finding
/// says what a contributor sees when they run markdownlint themselves.
fn message(record: &Record) -> String {
    let mut message = record.rule_names.join(ALIAS_SEPARATOR);

    message.push(' ');
    message.push_str(&record.rule_description);

    if let Some(detail) = &record.error_detail {
        message.push_str(" [");
        message.push_str(detail);
        message.push(']');
    }

    if let Some(context) = &record.error_context {
        message.push_str(" [Context: \"");
        message.push_str(context);
        message.push_str("\"]");
    }

    message
}

/// Returns the problems that a report of markdownlint holds
///
/// A run that reported nothing writes nothing, and an empty report holds no
/// problem. Everything else is the JSON array that markdownlint wrote.
///
/// # Errors
///
/// Returns the error of a report that is not the array that markdownlint
/// writes. Markdownlint answers a file that it cannot open by ending the run
/// with a stack trace and no report, so a reading that tolerated such a
/// report would let a run pass with an unknown part of the project unread.
pub(super) fn problems(report: &str) -> Result<Vec<MarkdownlintProblem>, serde_json::Error> {
    let text = report.trim();

    if text.is_empty() {
        return Ok(Vec::new());
    }

    let records: Vec<Record> = serde_json::from_str(text)?;

    Ok(records.into_iter().map(Record::into_problem).collect())
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a report
    // which markdownlint could have written expects the reading to succeed. A
    // `# Panics` section on every test would repeat that and give the reader
    // no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A result of a rule that points at a character in its line
    const RANGED: &str = r#"[{"fileName":"sub/notes.md","lineNumber":3,"ruleNames":["MD030","list-marker-space"],"ruleDescription":"Spaces after list markers","ruleInformation":"https://example.invalid/md030.md","errorDetail":"Expected: 1; Actual: 2","errorContext":null,"errorRange":[1,3],"fixInfo":{"editColumn":2},"severity":"error"}]"#;

    /// A result of a rule that speaks about the whole line
    const UNRANGED: &str = r##"[{"fileName":"notes.md","lineNumber":1,"ruleNames":["MD041","first-line-heading","first-line-h1"],"ruleDescription":"First line in a file should be a top-level heading","ruleInformation":"https://example.invalid/md041.md","errorDetail":null,"errorContext":"#Bad","errorRange":null,"fixInfo":null,"severity":"error"}]"##;

    /// What markdownlint writes when it ends without a report
    const STACK_TRACE: &str = "Error: EACCES: permission denied, open 'locked.md'\n    at Object.readFileSync (node:fs:484:20)\n";

    /// Returns the single problem of a report that holds one
    fn problem(report: &str) -> MarkdownlintProblem {
        let mut problems =
            problems(report).expect("the test reads a report that markdownlint could write");

        problems
            .pop()
            .expect("the test reads a report that holds one problem")
    }

    // lintmarkdown[verify check.column]
    #[test]
    fn problems_of_a_ranged_result_take_the_column_of_the_range() {
        let problem = problem(RANGED);

        assert_eq!(problem.column(), Some(1));
    }

    // lintmarkdown[verify check.violation]
    #[test]
    fn problems_of_a_result_name_the_file() {
        let problem = problem(RANGED);

        assert_eq!(problem.path(), &PathBuf::from("sub/notes.md"));
    }

    // lintmarkdown[verify check.violation]
    #[test]
    fn problems_of_a_result_sit_on_the_line_of_markdownlint() {
        let problem = problem(RANGED);

        assert_eq!(problem.line(), 3);
    }

    // lintmarkdown[verify check.violation]
    #[test]
    fn problems_of_a_result_with_a_detail_read_like_markdownlint() {
        let problem = problem(RANGED);

        assert_eq!(
            problem.message(),
            "MD030/list-marker-space Spaces after list markers [Expected: 1; Actual: 2]"
        );
    }

    // lintmarkdown[verify check.violation]
    #[test]
    fn problems_of_a_result_with_a_context_quote_it_like_markdownlint() {
        let problem = problem(UNRANGED);

        assert_eq!(
            problem.message(),
            "MD041/first-line-heading/first-line-h1 First line in a file should be a top-level heading [Context: \"#Bad\"]"
        );
    }

    // lintmarkdown[verify check.column]
    #[test]
    fn problems_of_an_unranged_result_name_no_column() {
        let problem = problem(UNRANGED);

        assert_eq!(problem.column(), None);
    }

    // lintmarkdown[verify check.passed]
    #[test]
    fn problems_of_an_empty_report_hold_nothing() {
        let problems = problems("  \n").expect("the test reads the report of a run that passed");

        assert!(problems.is_empty(), "expected no problem, got {problems:?}");
    }

    // lintmarkdown[verify check.unreadable]
    #[test]
    fn problems_of_a_report_that_is_not_a_report_stop_the_reading() {
        let problems = problems(STACK_TRACE);

        assert!(
            problems.is_err(),
            "expected the reading to stop, got {problems:?}"
        );
    }
}
