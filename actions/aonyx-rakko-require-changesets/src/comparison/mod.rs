//! What a pull request changes, as Git compares its base with its head

use std::path::PathBuf;

use getset::Getters;

use crate::change::Change;

/// The files that a pull request changes, and where the project lies in its
/// repository
///
/// Git and Otto write each path relative to the top of the repository. A
/// finding gives its location relative to the root of the project, which can
/// be a directory below the top. The prefix is the path from the top to that
/// root, and it is empty when the project is the whole repository.
#[derive(Clone, Eq, PartialEq, Debug, Default, bon::Builder, Getters)]
pub(crate) struct Comparison {
    /// The path from the top of the repository to the root of the project
    #[builder(into, default)]
    #[getset(get = "pub(crate)")]
    prefix: PathBuf,

    /// The files that the pull request changes, in the order that Git lists
    /// them
    #[builder(default)]
    #[getset(get = "pub(crate)")]
    changes: Vec<Change>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Comparison>();
    }
}
