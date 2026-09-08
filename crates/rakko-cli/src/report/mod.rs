/// The JSON that a run emits for a machine
mod schema;
/// The text that a run emits for a reader at a terminal
mod text;

use std::fmt;

use rakko_action::{Name, Outcome};
use serde::{Serialize, Serializer};

/// What a run reports about the action that it drove
///
/// A report owns the name of the action and the outcome that the action
/// returned, and it is what a run hands to the command line. Every way of
/// showing a run reads this one value: [`Display`] writes the text that a
/// reader at a terminal gets, and [`Serialize`] writes the JSON that a
/// machine gets.
///
/// An action produces none of this. It returns an outcome, and the report
/// decides how that outcome reaches a reader, so the output of every project
/// in the fleet has one shape and no action carries code that draws it.
///
/// A command that a harness wrote reports what it wants, and a command that
/// drives actions itself wants exactly this. It creates one report for each
/// action that it ran and writes that report as an artifact of Clawless, so
/// the run shows what a run of each action alone shows, in text and in JSON,
/// and the harness carries no renderer of its own.
///
/// # Examples
///
/// ```
/// use rakko_action::{Outcome, Summary, action_name};
/// use rakko_cli::Report;
///
/// let outcome = Outcome::Passed {
///     summary: Some(Summary::new("checked 3 files")),
/// };
///
/// let report = Report::new(action_name!("format-toml"), outcome);
///
/// assert_eq!(report.to_string(), "format-toml: passed, checked 3 files");
/// ```
///
/// [`Display`]: fmt::Display
// cli[impl report.written]
#[derive(Debug)]
pub struct Report {
    /// The name of the action that the run drove
    action: Name,
    /// What that action returned
    outcome: Outcome,
}

impl Report {
    /// Creates a report from the name of an action and what that action
    /// returned
    // cli[impl report.written]
    #[must_use]
    pub fn new(action: Name, outcome: Outcome) -> Self {
        Self { action, outcome }
    }
}

// cli[impl report.findings]
// cli[impl report.repairs]
// cli[impl report.skipped]
// cli[impl report.errored]
impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self::text::render(self, formatter)
    }
}

// cli[impl report.json]
impl Serialize for Report {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self::schema::Payload::of(self).serialize(serializer)
    }
}
