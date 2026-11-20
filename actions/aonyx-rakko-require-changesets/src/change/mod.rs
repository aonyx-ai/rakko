//! A file that a pull request changes

mod kind;

use std::path::PathBuf;

use getset::{CopyGetters, Getters};

pub(crate) use self::kind::ChangeKind;

/// A file that a pull request changes between its base and its head
///
/// The path is relative to the top of the repository, as Git and Otto write
/// their paths. The kind tells an added file from any other change. Only an
/// added file in the changeset directory of a unit counts as a new changeset
/// of that unit.
#[derive(
    Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, bon::Builder, CopyGetters, Getters,
)]
pub(crate) struct Change {
    /// The file, relative to the top of the repository
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    path: PathBuf,

    /// What the pull request did to the file
    #[getset(get_copy = "pub(crate)")]
    kind: ChangeKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Change>();
    }
}
