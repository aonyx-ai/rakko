use rakko_action::{Finding, Location, Outcome, Position};
use serde::Serialize;

use super::Report;

/// The value that the schema field carries while the shape can still change
///
/// The payload promises nothing yet: a consumer that reads this value knows
/// that the shape can change in any release, and a stable schema replaces it
/// later.
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
///
/// The level names how precisely the action could place the problem, and it
/// decides which of the remaining fields carry a value. A consumer reads the
/// level first and then knows what it can rely on, so it never has to guess
/// why a field is absent.
#[derive(Serialize)]
struct FindingPayload<'a> {
    /// How precisely the action could place the problem
    level: &'static str,
    /// The path that the problem is at, relative to the project root
    ///
    /// Every level except `project` carries a path.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    /// The line that the problem starts on
    ///
    /// The `position` and the `span` level carry a line.
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    /// The column that the problem starts at
    ///
    /// A tool that reports a line without a column leaves this field out.
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<u32>,
    /// The line that the problem ends on
    ///
    /// The `span` level carries this field, and no other level does.
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<u32>,
    /// The column that the problem ends at
    ///
    /// A span whose end names a line without a column leaves this field out.
    #[serde(skip_serializing_if = "Option::is_none")]
    end_column: Option<u32>,
    /// The message that describes the problem
    message: &'a str,
}

impl<'a> FindingPayload<'a> {
    /// Returns the payload that describes the given finding
    fn of(finding: &'a Finding) -> Self {
        let message = finding.message().get();

        match finding.location() {
            Location::Project => Self::new("project", message),
            Location::Directory { path } => Self {
                path: Some(path.to_string()),
                ..Self::new("directory", message)
            },
            Location::File { path } => Self {
                path: Some(path.to_string()),
                ..Self::new("file", message)
            },
            Location::Position { path, position } => Self {
                path: Some(path.to_string()),
                line: Some(position.line().get()),
                column: column_of(position),
                ..Self::new("position", message)
            },
            Location::Span { path, span } => Self {
                path: Some(path.to_string()),
                line: Some(span.start().line().get()),
                column: column_of(span.start()),
                end_line: Some(span.end().line().get()),
                end_column: column_of(span.end()),
                ..Self::new("span", message)
            },
        }
    }

    /// Returns the payload of a finding at the given level
    ///
    /// The level decides which of the remaining fields carry a value, so this
    /// constructor leaves all of them empty and each level fills in its own.
    fn new(level: &'static str, message: &'a str) -> Self {
        Self {
            level,
            path: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
            message,
        }
    }
}

/// Returns the column of a position, when the position names one
fn column_of(position: &Position) -> Option<u32> {
    position.column().map(|column| column.get())
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{DirectoryPath, FilePath, SkipReason, Span};

    use super::*;

    /// Returns a failed outcome that holds one finding
    fn failure(location: Location, message: &str) -> Outcome {
        Outcome::Failed {
            findings: vec![
                Finding::builder()
                    .message(message)
                    .location(location)
                    .build(),
            ],
        }
    }

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
    fn payload_of_directory_finding_carries_the_level_and_the_path() {
        let json = payload(failure(
            Location::Directory {
                path: DirectoryPath::try_from("crates/rakko")
                    .expect("the test names a relative path"),
            },
            "the directory has no specification",
        ));

        assert!(json.contains(
            r#"{"level":"directory","path":"crates/rakko","message":"the directory has no specification"}"#
        ));
    }

    // cli[verify report.json]
    #[test]
    fn payload_of_failed_outcome_carries_every_finding_with_its_location() {
        let json = payload(failure(
            Location::Position {
                path: FilePath::try_from("deny.toml").expect("the test names a relative path"),
                position: Position::builder().line(3).column(1).build(),
            },
            "the license is not allowlisted",
        ));

        assert_eq!(
            json,
            r#"{"schema":"unstable","action":"probe","outcome":"failed","findings":[{"level":"position","path":"deny.toml","line":3,"column":1,"message":"the license is not allowlisted"}]}"#
        );
    }

    // cli[verify report.json]
    #[test]
    fn payload_of_file_finding_carries_the_level_and_the_path() {
        let json = payload(failure(
            Location::File {
                path: FilePath::try_from("Cargo.toml").expect("the test names a relative path"),
            },
            "the file is not formatted",
        ));

        assert!(json.contains(
            r#"{"level":"file","path":"Cargo.toml","message":"the file is not formatted"}"#
        ));
    }

    // cli[verify report.json]
    #[test]
    fn payload_of_project_finding_carries_the_level_alone() {
        let json = payload(failure(Location::Project, "the crate serde is banned"));

        assert!(json.contains(r#"{"level":"project","message":"the crate serde is banned"}"#));
    }

    // cli[verify report.json]
    #[test]
    fn payload_of_span_finding_carries_both_ends_of_the_range() {
        let json = payload(failure(
            Location::Span {
                path: FilePath::try_from("src/lib.rs").expect("the test names a relative path"),
                span: Span::builder()
                    .start(Position::builder().line(1).column(1).build())
                    .end(Position::builder().line(3).build())
                    .build(),
            },
            "the block is not formatted",
        ));

        assert!(json.contains(
            r#"{"level":"span","path":"src/lib.rs","line":1,"column":1,"end_line":3,"message":"the block is not formatted"}"#
        ));
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
