//! What a run of the action does with the files that it examines

/// What a run of the action does with the files that it examines
///
/// A run reports by default. The `fix` argument of the action asks for a
/// rewrite instead. Mise lock changes the project only in a rewrite.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_check_mise_lock::Operation;
///
/// let operation = Operation::default();
///
/// assert_eq!(operation, Operation::Report);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Operation {
    /// Report the lockfiles that mise lock would change, and write nothing
    #[default]
    Report,

    /// Write the lockfiles that mise lock would change
    Rewrite,
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
