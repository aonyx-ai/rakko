//! Where the head branch of a pull request lives

/// The repository that holds the head branch of a pull request
///
/// A pull request from a fork has its head branch in another repository.
/// Anyone can create a branch `release/<unit>` there, so only a release branch
/// in the repository of the pull request marks a release of Otto.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum Origin {
    /// The head branch is in the repository of the pull request
    BaseRepository,

    /// The head branch is in another repository, such as a fork, or in a
    /// repository that GitHub no longer has
    OtherRepository,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Origin>();
    }
}
