//! What a run of the action does with the files that it examines

/// What a run of the action does with the files that it examines
///
/// A run reports by default. The `fix` argument of the action asks for a
/// rewrite instead. Shfmt changes the project only in a rewrite.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_format_shell::FormatMode;
///
/// let format_mode = FormatMode::default();
///
/// assert_eq!(format_mode, FormatMode::Report);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum FormatMode {
    /// Report the files that shfmt would change, and rewrite nothing
    #[default]
    Report,

    /// Rewrite the files that shfmt can format
    Rewrite,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_mode_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<FormatMode>();
    }
}
