//! Types for the operations of prettier that an action runs

/// One operation of prettier that an action runs
///
/// Prettier does one job in two ways, and an action wraps one of them. This
/// vocabulary names the ways. So an action states what it wants and never
/// writes a command line of its own.
///
/// The two operations differ in one thing: whether prettier writes. A report
/// names the files that a rewrite would change and leaves the project alone. So
/// a run that a user started in order to look changes nothing. A rewrite
/// formats what it can and says which files it changed.
///
/// A later release can add an operation for another job of prettier. So a
/// match on an operation outside this crate needs an arm for an operation
/// that it does not know.
///
/// # Examples
///
/// An action that only looks asks for the report, which is the default:
///
/// ```
/// use aonyx_rakko_prettier::Operation;
///
/// assert_eq!(Operation::default(), Operation::Report);
/// assert_ne!(Operation::Report, Operation::Rewrite);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[non_exhaustive]
pub enum Operation {
    /// Report the files that prettier would change, and rewrite nothing
    #[default]
    Report,

    /// Rewrite the files that prettier can format
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
