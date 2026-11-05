//! How serious the compiler considers a diagnostic

/// How serious the compiler considers a diagnostic
///
/// The compiler writes notes and help lines as well, but those explain a
/// diagnostic above them, and a reader wants the diagnostic. Only the two
/// levels that name a problem of the project travel in a report.
///
/// # Examples
///
/// ```
/// use rakko_cargo::{CargoReport, DiagnosticLevel};
///
/// let stdout = r#"{"reason":"compiler-message","message":{"level":"warning","code":null,"message":"unused variable","spans":[]}}"#;
/// let report = CargoReport::read(stdout)?;
///
/// assert_eq!(report.diagnostics()[0].level(), DiagnosticLevel::Warning);
/// # Ok::<(), rakko_cargo::ReadReportError>(())
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum DiagnosticLevel {
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
