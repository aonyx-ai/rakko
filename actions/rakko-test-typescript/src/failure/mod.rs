//! One test that failed
//!
//! A run of the test runner reports one result per test. This module holds
//! the ones that failed: which test it was, where the project declares it, and
//! what Node said about the failure. A result that passed carries nothing that
//! a reader needs. So the reading keeps none of them.

mod origin;

use bon::Builder;
use getset::Getters;
use rakko_action::{Finding, Location, Position, ProjectRoot};

pub use self::origin::Origin;

/// One test that failed
///
/// Node names every test that it ran and says whether it passed. A test that
/// failed carries the place where the project declares it and what went wrong.
/// Node leaves either of them out for a failure that it has nothing to say
/// about. An example is a test file that it could not load.
///
/// The name is the name of the test alone, without the tests that hold it, and
/// that is what Node reports. A reader searches the project for it.
///
/// # Examples
///
/// ```
/// use rakko_test_typescript::failure::Failure;
///
/// let failure = Failure::builder()
///     .name("add")
///     .reason("Expected values to be strictly equal: 2 !== 3")
///     .build();
///
/// assert_eq!(*failure.name(), "add");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, Getters)]
pub struct Failure {
    /// The name of the test, as Node wrote it
    #[builder(into)]
    #[getset(get = "pub")]
    name: String,

    /// The place where the project declares the test, or `None` for no place
    #[getset(get = "pub")]
    origin: Option<Origin>,

    /// What Node said about the failure, or `None` when it said nothing
    #[builder(into)]
    #[getset(get = "pub")]
    reason: Option<String>,
}

impl Failure {
    /// Returns the finding that reports the failure
    ///
    /// The finding names the test and carries what Node said. It sits at the
    /// place where the project declares the test, with the path relative to
    /// the project root.
    ///
    /// A failure that Node reported no place for, and one whose place lies
    /// outside the project, get a finding at the level of the project. This is
    /// because there is no path in the project to report them at.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_action::ProjectRoot;
    /// use rakko_test_typescript::failure::{Failure, Origin};
    /// use std::path::PathBuf;
    ///
    /// let failure = Failure::builder()
    ///     .name("add")
    ///     .origin(
    ///         Origin::builder()
    ///             .path(PathBuf::from("/home/otter/project/src/add.test.ts"))
    ///             .line(5)
    ///             .column(1)
    ///             .build(),
    ///     )
    ///     .reason("Expected values to be strictly equal: 2 !== 3")
    ///     .build();
    /// let root = ProjectRoot::new("/home/otter/project".into());
    ///
    /// let finding = failure.finding(&root);
    ///
    /// assert!(finding.message().get().contains("test `add` failed"));
    /// ```
    pub fn finding(&self, root: &ProjectRoot) -> Finding {
        let stated = self
            .reason
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty());

        let message = match stated {
            Some(reason) => format!("test `{}` failed: {reason}", self.name),
            None => format!("test `{}` failed", self.name),
        };

        let location = self
            .origin
            .as_ref()
            .and_then(|origin| {
                let path = origin.relative_path(root)?;
                let position = Position::builder()
                    .line(origin.line())
                    .column(origin.column())
                    .build();

                Some(Location::Position { path, position })
            })
            .unwrap_or(Location::Project);

        Finding::builder()
            .message(message)
            .location(location)
            .build()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko_test_utils::path;

    use super::*;

    /// Returns a failure of a test that the project declares in a file
    fn located() -> Failure {
        Failure::builder()
            .name("counts")
            .origin(
                Origin::builder()
                    .path(path("/home/otter/project/src/add.test.ts"))
                    .line(5)
                    .column(1)
                    .build(),
            )
            .reason("Expected values to be strictly equal: 2 !== 3")
            .build()
    }

    /// The root that the failures of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn failure_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Failure>();
    }

    #[test]
    fn finding_carries_what_node_said() {
        let finding = located().finding(&root());

        assert_eq!(
            finding.message().get(),
            "test `counts` failed: Expected values to be strictly equal: 2 !== 3"
        );
    }

    #[test]
    fn finding_names_the_test() {
        let finding = located().finding(&root());

        assert!(
            finding.message().get().contains("`counts`"),
            "expected the name of the test, got {:?}",
            finding.message()
        );
    }

    /// Shows that a finding of a failure with an empty reason names the test
    /// alone
    ///
    /// Node writes an empty reason for a failure that it has nothing to say
    /// about. A message that ends in a colon tells a reader nothing.
    #[test]
    fn finding_of_a_failure_with_an_empty_reason_names_the_test_alone() {
        let failure = Failure::builder()
            .name("counts")
            .maybe_origin(None)
            .reason("")
            .build();

        let finding = failure.finding(&root());

        assert_eq!(finding.message().get(), "test `counts` failed");
    }

    #[test]
    fn finding_of_a_failure_without_a_place_belongs_to_the_project() {
        let failure = Failure::builder()
            .name("counts")
            .maybe_origin(None)
            .reason("test failed")
            .build();

        let finding = failure.finding(&root());

        assert_eq!(finding.location(), &Location::Project);
    }

    #[test]
    fn finding_of_a_failure_without_a_reason_names_the_test_alone() {
        let failure = Failure::builder()
            .name("counts")
            .maybe_origin(None)
            .maybe_reason(None::<String>)
            .build();

        let finding = failure.finding(&root());

        assert_eq!(finding.message().get(), "test `counts` failed");
    }

    #[test]
    fn finding_of_a_place_outside_the_project_belongs_to_the_project() {
        let failure = Failure::builder()
            .name("counts")
            .origin(
                Origin::builder()
                    .path(path("/elsewhere/add.test.ts"))
                    .line(5)
                    .column(1)
                    .build(),
            )
            .reason("test failed")
            .build();

        let finding = failure.finding(&root());

        assert_eq!(finding.location(), &Location::Project);
    }

    #[test]
    fn finding_sits_where_the_project_declares_the_test() {
        let finding = located().finding(&root());

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "src/add.test.ts".parse().unwrap(),
                position: Position::builder().line(5).column(1).build(),
            }
        );
    }
}
