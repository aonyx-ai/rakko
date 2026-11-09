//! What a run of the action does with the files that it examines

/// What a run of the action does with the files that it examines
///
/// A run reports by default. The `fix` argument of the action asks for a
/// rewrite instead. Rustfmt changes the project only in a rewrite.
///
/// # Examples
///
/// ```
/// use rakko_format_rust::Operation;
///
/// let operation = Operation::default();
///
/// assert_eq!(operation, Operation::Report);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Operation {
    /// Report the files that rustfmt would change, and rewrite nothing
    #[default]
    Report,

    /// Rewrite the files that rustfmt can format
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
