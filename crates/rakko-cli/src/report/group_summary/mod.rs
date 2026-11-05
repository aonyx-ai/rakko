//! What a run of a group reports about the members that it drove

mod schema;
mod text;

use std::fmt;

use bon::Builder;
use rakko_action::Name;
use serde::{Serialize, Serializer};

use super::Report;

/// What a run of a group reports about the members that it drove
///
/// A run of a group answers as a run of one action does, with one artifact,
/// and the summary is that artifact. It holds the name of the group and the
/// report of every member, in the order of the list of the group. So a machine
/// that reads the output of the run reads one document, and that document
/// holds all that the members found.
///
/// The text gives one line for each member that did not pass. Then one line
/// counts the members that found problems and the members that stopped. A
/// member passes when it passed, changed the project, or skipped, as a run of
/// that action alone ends clean in these states. The flags that ask for less
/// output hide the report of each member, but not the summary. So such a run
/// that fails still tells the user which members to look at.
///
/// The JSON holds the name of the group, the number of members in each state,
/// and the report of each member. Each report has the shape that a run of that
/// action alone emits.
#[derive(Debug, Builder)]
pub(crate) struct GroupSummary {
    /// The name of the group that the run drove
    group: Name,
    /// The report of each member, in the order of the list of the group
    members: Vec<Report>,
}

impl fmt::Display for GroupSummary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self::text::render(self, formatter)
    }
}

impl Serialize for GroupSummary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self::schema::Payload::of(self).serialize(serializer)
    }
}
