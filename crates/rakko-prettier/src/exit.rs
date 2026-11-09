//! The way a run of prettier ended

/// The way a run of prettier ended
///
/// A run ends with success or with failure. A report that names files that a
/// rewrite would change ends with failure, so the exit alone does not say
/// that a run went wrong. A caller reads the problems of the observation too.
///
/// # Examples
///
/// ```
/// use rakko_prettier::Exit;
///
/// let exit = Exit::default();
///
/// assert_eq!(exit, Exit::Failed);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Exit {
    /// Prettier ended the run with success
    Succeeded,

    /// Prettier ended the run without success
    #[default]
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Exit>();
    }
}
