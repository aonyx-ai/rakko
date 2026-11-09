//! What `mise lock` did to a part

/// What `mise lock` did to one part of a lockfile
///
/// The variant says whether the part was missing before the run, is missing
/// after it, or is there on both sides with other content.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Change {
    /// `mise lock` wrote a part that the lockfile did not hold
    Added,

    /// `mise lock` wrote other content for a part that the lockfile held
    Changed,

    /// `mise lock` took away a part that the lockfile held
    Removed,
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
