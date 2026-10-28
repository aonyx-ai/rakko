use std::path::PathBuf;

use rakko_action::Position;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::problem::{Severity, StilesProblem};

/// One diagnostic of the report that stiles wrote
///
/// Stiles writes more than these fields, such as the byte offsets where the
/// diagnostic starts and ends, and the reading ignores the rest, so a field
/// that a new version adds does not break it.
#[derive(Deserialize)]
struct Record {
    /// The identifier of the rule that fired
    rule: String,

    /// How the rule weighed the diagnostic
    severity: Severity,

    /// What the rule said about the code
    message: String,

    /// The file of the diagnostic, as stiles found it
    file: PathBuf,

    /// The line where the diagnostic starts, counting from 1
    line: u32,

    /// The column where the diagnostic starts, in characters, counting from 1
    column: u32,
}

impl Record {
    /// Returns the problem that this diagnostic describes
    fn into_problem(self) -> StilesProblem {
        StilesProblem::new(
            self.file,
            Position::builder()
                .line(self.line)
                .column(self.column)
                .build(),
            self.severity,
            self.rule,
            self.message,
        )
    }
}

/// One rule of the listing that stiles wrote
///
/// Stiles also names the languages of the rule, and the reading ignores them,
/// because only whether the project runs the rule decides anything here.
#[derive(Deserialize)]
struct ListedRule {
    /// The identifier of the rule
    rule: String,

    /// Whether the configuration of the project lets the rule report
    enabled: bool,
}

/// Returns the problems that a report of stiles holds
///
/// Stiles writes one object per line, one for each diagnostic, and a check
/// that reported nothing writes nothing at all.
///
/// # Errors
///
/// Returns the error of a line that is not a diagnostic of stiles. A reading
/// that tolerated such a line would let a run pass while a diagnostic of the
/// run went unread.
pub(super) fn problems(report: &str) -> Result<Vec<StilesProblem>, serde_json::Error> {
    let records: Vec<Record> = objects(report)?;

    Ok(records.into_iter().map(Record::into_problem).collect())
}

/// Returns the identifiers of the rules that a listing of stiles turns on
///
/// Stiles writes one object per line, one for each rule that the configured
/// lints declare, and it marks a rule that the configuration of the project
/// turns off. A project that configures no lint gets an empty listing.
///
/// # Errors
///
/// Returns the error of a line that is not a rule of stiles.
pub(super) fn enabled_rules(listing: &str) -> Result<Vec<String>, serde_json::Error> {
    let rules: Vec<ListedRule> = objects(listing)?;

    Ok(rules
        .into_iter()
        .filter(|rule| rule.enabled)
        .map(|rule| rule.rule)
        .collect())
}

/// Returns the objects of a text that holds one JSON object per line
///
/// A blank line holds no object, so the end of the last line and an empty
/// text read as nothing.
///
/// # Errors
///
/// Returns the error of the first line that is not an object of the given
/// shape.
fn objects<T: DeserializeOwned>(text: &str) -> Result<Vec<T>, serde_json::Error> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(serde_json::from_str)
        .collect()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a report
    // which stiles could have written expects the reading to succeed. A
    // `# Panics` section on every test would repeat that and give the reader
    // no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A diagnostic that a rule weighs as a warning
    const WARNING: &str = r#"{"rule":"lint.todo-comment","severity":"warning","message":"a TODO comment marks unfinished work","file":"./src/lib.rs","start":120,"end":124,"line":7,"column":5}"#;

    /// A diagnostic that a rule weighs as an error
    const ERROR: &str = r#"{"rule":"lint.wildcard-match-arm","severity":"error","message":"name the variants","file":"./src/lib.rs","start":200,"end":201,"line":9,"column":9}"#;

    /// A listing with one rule that the project runs and one that it turned off
    const LISTING: &str = "{\"rule\":\"lint.todo-comment\",\"languages\":[\"rust\"],\"enabled\":true}\n\
                           {\"rule\":\"lint.no-inline-comments\",\"languages\":[\"rust\"],\"enabled\":false}\n";

    /// Returns the problems of a report that the test expects to read
    fn read(report: &str) -> Vec<StilesProblem> {
        problems(report).expect("the test reads a report that stiles could have written")
    }

    #[test]
    fn enabled_rules_of_a_listing_leave_out_a_rule_that_is_off() {
        let rules = enabled_rules(LISTING).expect("the test reads a listing of stiles");

        assert_eq!(rules, ["lint.todo-comment"]);
    }

    #[test]
    fn enabled_rules_of_an_empty_listing_name_nothing() {
        let rules = enabled_rules("").expect("the test reads an empty listing");

        assert!(rules.is_empty());
    }

    #[test]
    fn enabled_rules_of_text_that_is_no_listing_fail() {
        let result = enabled_rules("lint.todo-comment (rust)\n");

        assert!(result.is_err());
    }

    #[test]
    fn problems_of_a_diagnostic_carry_its_position() {
        let problems = read(WARNING);

        assert_eq!(
            problems[0].position(),
            Position::builder().line(7).column(5).build()
        );
    }

    #[test]
    fn problems_of_a_diagnostic_carry_its_severity() {
        let problems = read(ERROR);

        assert_eq!(problems[0].severity(), Severity::Error);
    }

    #[test]
    fn problems_of_a_report_keep_its_order() {
        let problems = read(&format!("{WARNING}\n{ERROR}\n"));

        assert_eq!(
            problems
                .iter()
                .map(|problem| problem.rule().as_str())
                .collect::<Vec<_>>(),
            ["lint.todo-comment", "lint.wildcard-match-arm"]
        );
    }

    #[test]
    fn problems_of_a_severity_that_stiles_does_not_write_fail() {
        let result = problems(&WARNING.replace("\"warning\"", "\"fatal\""));

        assert!(result.is_err());
    }

    #[test]
    fn problems_of_an_empty_report_are_none() {
        let problems = read("");

        assert!(problems.is_empty());
    }

    #[test]
    fn problems_of_text_that_is_no_report_fail() {
        let result = problems("warning: something\n");

        assert!(result.is_err());
    }
}
