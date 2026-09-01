/// What taplo reported about a file, at the level that taplo could name
///
/// Taplo names a problem as precisely as its operation allows. A formatting
/// run knows only that a file differs from what it would write, a lint run
/// that cannot read a file knows only the reason, and a run that parsed a
/// file points at the character that broke it. Each variant carries one of
/// those levels, so a caller reports what taplo knew and invents nothing.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ProblemDetail {
    /// Taplo reported a diagnostic at a position in the file
    Diagnostic {
        /// The line that the diagnostic points at, starting at 1
        line: u32,

        /// The column that the diagnostic points at, starting at 1
        column: u32,

        /// The message of taplo, with the label of the diagnostic when it
        /// carries one
        message: String,
    },

    /// Taplo refused the file and named no position in it
    ///
    /// A file that taplo cannot open ends this way: taplo has the reason
    /// and nothing else, because it never read a character of the file.
    Invalid {
        /// What taplo said about the file
        reason: String,
    },

    /// The file is not formatted
    ///
    /// Taplo names the file and nothing in it, so the problem has no
    /// position.
    Unformatted,
}
