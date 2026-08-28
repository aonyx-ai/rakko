use rakko_action::{Finding, Outcome};
use serde::Serialize;

use super::Report;

/// The value that the schema field carries while the shape can still change
///
/// A finding names a file today, and it can name a line and a column in that
/// file. Which granularities a finding will offer is an open question, so the
/// payload promises nothing: a consumer that reads this value knows that the
/// shape can change in any release, and a stable schema replaces it later.
const UNSTABLE: &str = "unstable";

/// The JSON that a run emits for a machine
///
/// The payload is a type of this crate, and it is not a serialization of the
/// types that an action produces. The two therefore change on their own
/// clocks: the contract crate can rearrange a finding without breaking a
/// consumer, and this schema can promise a shape that no type in the contract
/// crate has to hold.
#[derive(Serialize)]
pub(super) struct Payload<'a> {
    /// Whether the shape of this payload is stable
    schema: &'static str,
    /// The name of the action that the run drove
    action: &'a str,
    /// The state that the action returned
    outcome: &'static str,
    /// The reason why the action does not apply, when it does not apply
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
    /// The error that stopped the action, when it stopped
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// The problems that the action found
    findings: Vec<FindingPayload<'a>>,
}

impl<'a> Payload<'a> {
    /// Returns the payload that describes the given report
    pub(super) fn of(report: &'a Report) -> Self {
        let action = report.action.get();

        match &report.outcome {
            Outcome::Passed => Self::new(action, "passed"),
            Outcome::Skipped { reason } => Self {
                reason: Some(reason.get()),
                ..Self::new(action, "skipped")
            },
            Outcome::Errored { source } => Self {
                error: Some(source.to_string()),
                ..Self::new(action, "errored")
            },
            Outcome::Failed { findings } => Self {
                findings: findings.iter().map(FindingPayload::of).collect(),
                ..Self::new(action, "failed")
            },
        }
    }

    /// Returns the payload of an action that reported the given state
    ///
    /// The state decides which of the remaining fields carry a value, so this
    /// constructor leaves all of them empty and each state fills in its own.
    fn new(action: &'a str, outcome: &'static str) -> Self {
        Self {
            schema: UNSTABLE,
            action,
            outcome,
            reason: None,
            error: None,
            findings: Vec::new(),
        }
    }
}

/// One problem that an action found, as a machine reads it
#[derive(Serialize)]
struct FindingPayload<'a> {
    /// The path of the file that the problem is in, relative to the project
    path: String,
    /// The line that the problem is on, when the finding names one
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    /// The column that the problem is at, when the finding names one
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<u32>,
    /// The message that describes the problem
    message: &'a str,
}

impl<'a> FindingPayload<'a> {
    /// Returns the payload that describes the given finding
    fn of(finding: &'a Finding) -> Self {
        let location = finding.location();
        let position = location.position().as_ref();

        Self {
            path: location.path().to_string(),
            line: position.map(|position| position.line().get()),
            column: position.and_then(|position| position.column().map(|column| column.get())),
            message: finding.message().get(),
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{FilePath, Location, Position, SkipReason};

    use super::*;

    /// Returns the JSON that a run of the action `probe` emits
    ///
    /// The assertions compare the JSON as text, and not as a parsed value, so
    /// that they pin the bytes that a consumer reads.
    fn payload(outcome: Outcome) -> String {
        let report = Report::new("probe".parse().expect("the test names an action"), outcome);

        serde_json::to_string(&report).expect("the report serializes")
    }

    // cli[verify report.json]
    #[test]
    fn payload_of_errored_outcome_carries_the_error() {
        let json = payload(Outcome::Errored {
            source: Box::new(std::io::Error::other("failed to read Cargo.toml")),
        });

        assert_eq!(
            json,
            r#"{"schema":"unstable","action":"probe","outcome":"errored","error":"failed to read Cargo.toml","findings":[]}"#
        );
    }

    // cli[verify report.json]
    #[test]
    fn payload_of_failed_outcome_carries_every_finding_with_its_location() {
        let location = Location::builder()
            .path(FilePath::try_from("deny.toml").expect("the test names a relative path"))
            .position(Position::builder().line(3).column(1).build())
            .build();
        let finding = Finding::builder()
            .message("the license is not allowlisted")
            .location(location)
            .build();

        let json = payload(Outcome::Failed {
            findings: vec![finding],
        });

        assert_eq!(
            json,
            r#"{"schema":"unstable","action":"probe","outcome":"failed","findings":[{"path":"deny.toml","line":3,"column":1,"message":"the license is not allowlisted"}]}"#
        );
    }

    // cli[verify report.json]
    #[test]
    fn payload_of_passed_outcome_carries_the_state_alone() {
        let json = payload(Outcome::Passed);

        assert_eq!(
            json,
            r#"{"schema":"unstable","action":"probe","outcome":"passed","findings":[]}"#
        );
    }

    // cli[verify report.json]
    #[test]
    fn payload_of_skipped_outcome_carries_the_reason() {
        let json = payload(Outcome::Skipped {
            reason: SkipReason::new("this project has no TOML file"),
        });

        assert_eq!(
            json,
            r#"{"schema":"unstable","action":"probe","outcome":"skipped","reason":"this project has no TOML file","findings":[]}"#
        );
    }
}
