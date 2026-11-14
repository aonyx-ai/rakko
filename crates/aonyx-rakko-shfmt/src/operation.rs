//! Types for the operations of shfmt that an action runs

/// One operation of shfmt that an action runs
///
/// Shfmt does more than one job, and an action wraps one of them. This
/// vocabulary names the jobs that Rakko uses, so that an action states what
/// it wants and never writes a command line of its own.
///
/// Every operation walks the project from its root, in the way that shfmt
/// walks a directory that it receives as an argument. Shfmt reads the
/// `.editorconfig` files of the project on that walk, and it leaves out a
/// file that such a file marks with `ignore = true`. So the three operations
/// agree on which files are the shell scripts of the project.
///
/// The two formatting operations differ in one thing: whether shfmt writes. A
/// check reports the files that it would rewrite and leaves the project alone.
/// So a run that a user started in order to look changes nothing.
///
/// A later release can add an operation for another job of shfmt. So a match
/// on an operation outside this crate needs an arm for an operation that it
/// does not know.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_shfmt::Operation;
///
/// let operation = Operation::CheckFormat;
///
/// assert_ne!(operation, Operation::Format);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[non_exhaustive]
pub enum Operation {
    /// Report the files that shfmt would format, and rewrite nothing
    CheckFormat,

    /// List the shell scripts of the project, and read none of them
    Find,

    /// Rewrite the files that shfmt can format, and report the files that it
    /// rewrote
    Format,
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
