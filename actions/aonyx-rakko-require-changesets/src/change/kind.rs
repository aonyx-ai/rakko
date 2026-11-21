//! What a pull request did to a file

/// What a pull request did to a file between its base and its head
///
/// The check needs only one difference. A file that the base does not hold
/// and the head holds is added. Every other change, such as an edit or a
/// deletion, changes the unit of the file but adds no changeset.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum ChangeKind {
    /// The head holds the file, and the base does not
    Added,

    /// The pull request edited or deleted the file, or changed its type
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_kind_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ChangeKind>();
    }
}
