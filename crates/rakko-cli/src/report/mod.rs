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
/// [`Display`]: fmt::Display
#[derive(Debug)]
pub(crate) struct Report {
    /// The name of the action that the run drove
    action: Name,
    /// What that action returned
    outcome: Outcome,
}

impl Report {
    /// Creates a report from the name of an action and what that action
    /// returned
    pub(crate) fn new(action: Name, outcome: Outcome) -> Self {
        Self { action, outcome }
    }
}

// cli[impl report.findings]
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
