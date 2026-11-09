//! Whether the pattern of a run of prettier matched a file

/// Whether the pattern of a run of prettier matched a file
///
/// Prettier treats a pattern that matches nothing as an error. This type
/// tells that case apart from a run that failed for another reason.
///
/// # Examples
///
/// ```
/// use rakko_prettier::Matching;
///
/// let matching = Matching::default();
///
/// assert_eq!(matching, Matching::Matched);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Matching {
    /// The pattern matched at least one file
    #[default]
    Matched,

    /// Prettier refused the pattern because nothing matched it
    Unmatched,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Matching>();
    }
}
