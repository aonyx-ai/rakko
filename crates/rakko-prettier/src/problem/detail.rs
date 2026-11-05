//! What prettier reported about a file

/// What prettier reported about a file, at the level that prettier could name
///
/// Prettier names a problem as precisely as its operation allows. A report
/// knows only that a file differs from what prettier would write. A run that
/// could not open a file knows only the reason. A run that parsed a file points
/// at the character that broke it. Each variant carries one of those levels, so
/// a caller reports what prettier knew and invents nothing.
///
/// # Examples
///
/// ```
/// use rakko_prettier::problem::ProblemDetail;
/// use rakko_prettier::PrettierProblem;
///
/// let problem = PrettierProblem::builder()
///     .path("a.md".into())
///     .detail(ProblemDetail::Diagnostic {
///         line: 1,
///         column: 10,
///         message: "SyntaxError: Unexpected token".to_owned(),
///     })
///     .build();
///
/// let ProblemDetail::Diagnostic { line, column, .. } = problem.detail() else {
///     panic!("expected a diagnostic");
/// };
/// assert_eq!((*line, *column), (1, 10));
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_detail_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ProblemDetail>();
    }
}
