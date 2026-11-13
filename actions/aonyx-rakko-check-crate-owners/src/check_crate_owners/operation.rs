//! What a run of the action does about a missing owner

/// What a run of the action does about a missing owner
///
/// A run reports by default. The `fix` argument of the action asks for a
/// repair instead, which adds each missing owner to the crate. A repair needs
/// the credentials of a user owner of each crate. So a run in CI reports.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_check_crate_owners::Operation;
///
/// let operation = Operation::default();
///
/// assert_eq!(operation, Operation::Report);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Operation {
    /// Report each missing owner, and change nothing on crates.io
    #[default]
    Report,

    /// Add each missing owner on crates.io
    Repair,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Operation>();
    }
}
