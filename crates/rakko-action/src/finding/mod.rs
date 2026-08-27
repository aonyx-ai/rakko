/// The location of a problem in a project
mod location;
/// The message that describes a problem
mod message;

pub use location::{ColumnNumber, FilePath, LineNumber, Location, Position};
pub use message::FindingMessage;

use bon::bon;
use getset::Getters;

/// One problem that an action found in a project
///
/// A finding says what the problem is and where it is. Findings travel in the
/// outcome of an action run.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Finding {
    /// A message that describes the problem
    #[getset(get = "pub")]
    message: FindingMessage,
    /// The location of the problem in the project
    #[getset(get = "pub")]
    location: Location,
}

#[bon]
impl Finding {
    /// Creates a finding from a message and a location
    // action[impl finding.message]
    // action[impl finding.location]
    #[builder]
    pub fn new(
        #[builder(into)] message: FindingMessage,
        #[builder(into)] location: Location,
    ) -> Self {
        Self { message, location }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // action[verify finding.location]
    #[test]
    fn location_returns_given_location() {
        let location = Location::builder().path("src/main.rs").build();
        let finding = Finding::builder()
            .message("missing semicolon")
            .location(location.clone())
            .build();

        assert_eq!(finding.location(), &location);
    }

    // action[verify finding.message]
    #[test]
    fn message_returns_given_message() {
        let location = Location::builder().path("src/main.rs").build();
        let finding = Finding::builder()
            .message("missing semicolon")
            .location(location)
            .build();

        assert_eq!(finding.message().get(), "missing semicolon");
    }
}
