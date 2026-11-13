//! Whether crates.io adds an owner at once or invites it

/// Whether crates.io adds an owner at once or invites it
///
/// The public list of the owners of a crate shows only the owners that
/// crates.io added. A user that has an invitation that they did not accept
/// yet is not on the list. So a report about a missing user says that the
/// user possibly did not accept the invitation yet. A report about a missing
/// team does not.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum OwnerKind {
    /// A user, whom crates.io invites
    User,

    /// A team of a GitHub organization, which crates.io adds at once
    Team,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_kind_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<OwnerKind>();
    }
}
