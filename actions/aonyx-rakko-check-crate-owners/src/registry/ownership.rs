//! What the registry knows about the owners of one crate

use crate::crate_owner::CrateOwner;

/// What the registry knows about the owners of one crate
///
/// A crate that the registry does not have is not an error. A new crate
/// reaches crates.io only with its first publish, and a binary can stay off
/// crates.io for good. So a run skips such a crate with a notice.
#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) enum Ownership {
    /// The registry has the crate, and these are its owners
    Published {
        /// The owners that the registry shows for the crate
        owners: Vec<CrateOwner>,
    },

    /// The registry does not have the crate
    Unpublished,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ownership_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Ownership>();
    }
}
