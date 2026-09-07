//! The reports that tracey writes as data
//!
//! Tracey answers a validation and a coverage query as JSON, and this module
//! holds the shape of each answer. The shapes belong to a version of tracey,
//! so one place reads them, and a version that writes something else breaks
//! that one place instead of every caller of it.
//!
//! Each shape names only the fields that a run reads. Tracey writes more about
//! a problem than a finding carries, such as the rule that a stale reference
//! points at, and a field that nothing reads is a field that a new version may
//! rename without breaking a run.

use serde::Deserialize;

use crate::problem::TraceyProblem;

/// What tracey reported about one specification and one implementation of it
///
/// Tracey validates each pair of a specification and an implementation on its
/// own, and reports the problems of that pair together with the counts that
/// sort them into warnings and errors.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Validation {
    /// The problems that tracey found in this pair
    #[serde(default)]
    errors: Vec<Diagnostic>,
}

impl Validation {
    /// Returns the problems that tracey found, in the shape of the crate
    pub(super) fn problems(&self) -> impl Iterator<Item = TraceyProblem> + '_ {
        self.errors.iter().map(Diagnostic::problem)
    }
}

/// One problem that a validation of tracey reported
///
/// Tracey places a problem in a file, at a line and a column, whether the
/// problem is a reference that names nothing, a reference to a version that
/// moved, an identifier that two requirements share, or a file that tracey
/// could not read.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    /// What tracey wrote about the problem
    message: String,

    /// The path of the file that the problem is in
    file: String,

    /// The line of the file that the problem is on
    line: u32,

    /// The column of the line that the problem is at
    column: u32,
}

impl Diagnostic {
    /// Returns the problem that this diagnostic describes
    fn problem(&self) -> TraceyProblem {
        TraceyProblem::builder()
            .path(self.file.clone())
            .line(self.line)
            .column(self.column)
            .message(self.message.clone())
            .build()
    }
}

/// What tracey reported about the coverage of the specifications of a project
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Status {
    /// The counts of each pair of a specification and an implementation
    #[serde(default)]
    impls: Vec<ImplCoverage>,
}

impl Status {
    /// Returns the coverage of every specification of the project together
    pub(super) fn coverage(&self) -> Coverage {
        self.impls
            .iter()
            .fold(Coverage::default(), |total, one| Coverage {
                total: total.total + one.total_rules,
                covered: total.covered + one.covered_rules,
                verified: total.verified + one.verified_rules,
            })
    }
}

/// The counts of one pair of a specification and an implementation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImplCoverage {
    /// How many requirements the specification holds
    total_rules: u64,

    /// How many of them the code implements
    covered_rules: u64,

    /// How many of them a test verifies
    verified_rules: u64,
}

/// How much of the specifications of a project the code answers for
///
/// The counts add every specification of the project together, because a
/// summary of a run says what the run examined and not how each specification
/// fared. The report of tracey holds the detail per specification.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Coverage {
    /// How many requirements the project holds
    pub total: u64,

    /// How many of them the code implements
    pub covered: u64,

    /// How many of them a test verifies
    pub verified: u64,
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What tracey wrote about a project with one broken reference
    const VALIDATION: &str = r#"[
        {
            "spec": "probe",
            "implName": "rust",
            "errors": [
                {
                    "code": "unknown_requirement",
                    "message": "Unknown rule reference probe[impl missing]",
                    "file": "src/lib.rs",
                    "line": 4,
                    "column": 3,
                    "relatedRules": [],
                    "referenceRuleId": null,
                    "referenceText": "probe[impl missing]"
                }
            ],
            "warningCount": 0,
            "errorCount": 1
        }
    ]"#;

    /// What tracey wrote about the coverage of a project with two specs
    const STATUS: &str = r#"{
        "impls": [
            {
                "spec": "one",
                "implName": "rust",
                "totalRules": 10,
                "coveredRules": 9,
                "staleRules": 0,
                "verifiedRules": 8
            },
            {
                "spec": "two",
                "implName": "rust",
                "totalRules": 5,
                "coveredRules": 5,
                "staleRules": 0,
                "verifiedRules": 4
            }
        ]
    }"#;

    #[test]
    fn validation_reports_the_message_of_tracey() {
        let validations: Vec<Validation> =
            serde_json::from_str(VALIDATION).expect("the test reads a report of tracey");

        let problems: Vec<_> = validations.iter().flat_map(Validation::problems).collect();

        assert_eq!(
            problems[0].message(),
            "Unknown rule reference probe[impl missing]"
        );
    }

    #[test]
    fn validation_reports_the_place_of_a_problem() {
        let validations: Vec<Validation> =
            serde_json::from_str(VALIDATION).expect("the test reads a report of tracey");

        let problems: Vec<_> = validations.iter().flat_map(Validation::problems).collect();

        assert_eq!(
            (
                problems[0].path().as_str(),
                *problems[0].line(),
                *problems[0].column()
            ),
            ("src/lib.rs", 4, 3)
        );
    }

    #[test]
    fn status_adds_every_specification_together() {
        let status: Status =
            serde_json::from_str(STATUS).expect("the test reads a report of tracey");

        let coverage = status.coverage();

        assert_eq!(
            coverage,
            Coverage {
                total: 15,
                covered: 14,
                verified: 12
            }
        );
    }
}
