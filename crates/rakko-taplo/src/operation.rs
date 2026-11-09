//! Types for the operations of taplo that an action runs

/// One operation of taplo that an action runs
///
/// Taplo does more than one job, and an action wraps one of them. This
/// vocabulary names the jobs that Rakko uses, so that an action states what
/// it wants and never writes a command line of its own.
///
/// The two formatting operations differ in one thing: whether taplo writes.
/// A check reports the files that it would rewrite and leaves the project
/// alone, so a run that a user started in order to look changes nothing.
///
/// A later release can add an operation for another job of taplo. So a
/// match on an operation outside this crate needs an arm for an operation
/// that it does not know.
///
/// # Examples
///
/// ```
/// use rakko_taplo::Operation;
///
/// let operation = Operation::CheckFormat;
///
/// assert_ne!(operation, Operation::Format);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[non_exhaustive]
pub enum Operation {
    /// Report the files that taplo would format, and rewrite nothing
    CheckFormat,

    /// Rewrite the files that taplo can format
    Format,

    /// Report the files that taplo cannot read, parse, or validate
    Lint,
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
