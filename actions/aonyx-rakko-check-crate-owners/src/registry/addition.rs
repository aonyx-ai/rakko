//! The answer of the registry to the addition of an owner

/// The answer of the registry to the addition of an owner
///
/// An addition that fails does not stop a run. The run reports it as a
/// finding of its crate, and it goes on with the other owners and crates. So
/// the report of a run with `fix` shows every owner that is still missing.
#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) enum Addition {
    /// The registry added the owner, or invited the user
    Added,

    /// The registry refused the addition
    ///
    /// crates.io refuses an addition when the user that asks for it is not a
    /// user owner of the crate. Only a user owner can add owners.
    Refused {
        /// What the tool that asked the registry wrote about the refusal
        details: String,
    },

    /// The addition did not reach the registry, or the registry answered
    /// with another error
    Failed {
        /// What the tool that asked the registry wrote about the failure
        details: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Addition>();
    }
}
