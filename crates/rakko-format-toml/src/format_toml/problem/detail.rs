/// What taplo reported about a file, at the level that taplo could name
#[derive(Clone, Eq, PartialEq, Debug)]
pub(in crate::format_toml) enum ProblemDetail {
    /// The file is not formatted
    ///
    /// Taplo names the file and nothing in it, so the problem has no
    /// position.
    Unformatted,

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
}
