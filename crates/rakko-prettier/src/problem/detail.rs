/// What prettier reported about a file, at the level that prettier could name
///
/// Prettier names a problem as precisely as its operation allows. A report
/// knows only that a file differs from what prettier would write, a run that
/// could not open a file knows only the reason, and a run that parsed a file
/// points at the character that broke it. Each variant carries one of those
/// levels, so a caller reports what prettier knew and invents nothing.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ProblemDetail {
    /// Prettier reported a diagnostic at a position in the file
    Diagnostic {
        /// The line that the diagnostic points at, starting at 1
        line: u32,

        /// The column that the diagnostic points at, starting at 1
        column: u32,

        /// The message of prettier, without the position that follows it
        message: String,
    },

    /// The file differs from what prettier would write
    ///
    /// Prettier names the file and nothing in it, so the problem has no
    /// position.
    Unformatted,

    /// Prettier could not read the file and named no position in it
    ///
    /// A file that prettier cannot open ends this way: prettier has the
    /// reason and nothing else, because it never read a character of the
    /// file.
    Unreadable {
        /// What prettier said about the file
        reason: String,
    },
}
