//! How serious the compiler considers a diagnostic

/// How serious the compiler considers a diagnostic
///
/// The compiler writes notes and help lines as well, but those explain a
/// diagnostic above them, and a reader wants the diagnostic. Only the two
/// levels that name a problem of the project travel in a report.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum DiagnosticLevel {
    /// The compiler refused the code, and the build did not finish
    Error,

    /// The compiler accepted the code and objects to it
    Warning,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_level_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<DiagnosticLevel>();
    }
}
