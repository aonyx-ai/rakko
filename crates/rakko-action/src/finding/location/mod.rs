/// The path of a file that a problem is in
mod file_path;
/// The position of a problem in a file
mod position;

pub use file_path::FilePath;
pub use position::{ColumnNumber, LineNumber, Position};

use bon::bon;
use getset::Getters;

/// The location of a problem in a project
///
/// A location tells where a problem is in a project. It always names a file,
/// and it can add a position in that file.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Location {
    /// The path of the file that the problem is in
    #[getset(get = "pub")]
    path: FilePath,
    /// The position of the problem in that file, or `None` when the location
    /// does not name a position
    #[getset(get = "pub")]
    position: Option<Position>,
}

#[bon]
impl Location {
    /// Creates a location from a file path and an optional position
    // action[impl location.path]
    // action[impl location.position]
    #[builder]
    pub fn new(
        #[builder(into)] path: FilePath,
        #[builder(into)] position: Option<Position>,
    ) -> Self {
        Self { path, position }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    // action[verify location.path]
    #[test]
    fn path_returns_given_path() {
        let location = Location::builder().path("src/main.rs").build();

        assert_eq!(location.path().get(), &PathBuf::from("src/main.rs"));
    }

    // action[verify location.position]
    #[test]
    fn position_returns_given_position() {
        let position = Position::builder().line(10).column(3).build();
        let location = Location::builder()
            .path("src/main.rs")
            .position(position)
            .build();

        assert_eq!(location.position(), &Some(position));
    }

    // action[verify location.position]
    #[test]
    fn position_without_value_returns_none() {
        let location = Location::builder().path("src/main.rs").build();

        assert_eq!(location.position(), &None);
    }
}
