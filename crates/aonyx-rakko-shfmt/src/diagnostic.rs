//! Types for one error that shfmt found in a file

use std::path::{Path, PathBuf};

use getset::{CopyGetters, Getters};
use rakko::{FilePath, ProjectRoot};

/// One error that shfmt found at a position in a file
///
/// Shfmt parses a script before it formats it, and a script that it cannot
/// parse gets this error instead of a format. Shfmt writes the error as one
/// line on its standard error stream, in the form `path:line:column: message`.
///
/// The path is absolute. Shfmt writes it relative to the directory where it
/// starts, which is the project root, and the reading joins the two. A caller
/// that reports the error asks for the path [relative][relative] to that root.
/// That is the name that a reader, a machine, and a code host all recognize.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use aonyx_rakko_shfmt::ShfmtDiagnostic;
///
/// let diagnostic = ShfmtDiagnostic::builder()
///     .path(PathBuf::from("/home/otter/project/run.sh"))
///     .line(1)
///     .column(1)
///     .message("`if` must be followed by a statement list".to_owned())
///     .build();
///
/// assert_eq!(diagnostic.line(), 1);
/// ```
///
/// [relative]: ShfmtDiagnostic::relative_path
#[derive(Clone, Eq, PartialEq, Debug, bon::Builder, CopyGetters, Getters)]
pub struct ShfmtDiagnostic {
    /// The path of the file
    #[getset(get = "pub")]
    path: PathBuf,

    /// The line that the error points at, starting at 1
    #[getset(get_copy = "pub")]
    line: u32,

    /// The column that the error points at, starting at 1
    #[getset(get_copy = "pub")]
    column: u32,

    /// What shfmt said about the position
    #[getset(get = "pub")]
    message: String,
}

#[bon::bon]
impl ShfmtDiagnostic {
    /// Reads one line that shfmt wrote to its standard error stream
    ///
    /// The line has the form `path:line:column: message`. A path can hold a
    /// colon, and so can a message. So the reading takes the first place where
    /// a colon, a number, a colon, a number, a colon, and a space follow each
    /// other. What comes before it is the path, and what comes after it is the
    /// message.
    ///
    /// Returns `None` for a line without that form. Shfmt writes such a line
    /// for an error that has no position, such as a file that it cannot open.
    #[builder]
    pub(crate) fn parse(line: &str, root: &Path) -> Option<Self> {
        line.match_indices(':').find_map(|(index, _)| {
            let (path, rest) = line.split_at(index);
            let (row, column, message) = position(rest)?;

            if path.is_empty() || message.is_empty() {
                return None;
            }

            Some(Self {
                path: root.join(path),
                line: row,
                column,
                message: message.to_owned(),
            })
        })
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. Shfmt starts in
    /// the root and reports what it found below it. So a path that does not fit
    /// points at a report that the caller misread. The caller decides what to
    /// do about that.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::ProjectRoot;
    /// use aonyx_rakko_shfmt::ShfmtDiagnostic;
    ///
    /// let project = std::env::temp_dir();
    /// let diagnostic = ShfmtDiagnostic::builder()
    ///     .path(project.join("run.sh"))
    ///     .line(1)
    ///     .column(1)
    ///     .message("unexpected token".to_owned())
    ///     .build();
    /// let root = ProjectRoot::new(project);
    ///
    /// let relative = diagnostic.relative_path(&root).unwrap();
    /// assert_eq!(relative.get().to_str(), Some("run.sh"));
    /// ```
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::within().path(&self.path).root(root).call()
    }
}

/// Reads the position and the message that follow the path of a line
///
/// The text starts at the colon after the path, in the form
/// `:line:column: message`. Returns `None` when the text does not have that
/// form, or when a number does not fit.
fn position(text: &str) -> Option<(u32, u32, &str)> {
    let text = text.strip_prefix(':')?;
    let (line, text) = text.split_once(':')?;
    let (column, message) = text.split_once(": ")?;

    Some((number(line)?, number(column)?, message))
}

/// Reads a number that holds only decimal digits
fn number(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    text.parse().ok()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko_test_utils::path;

    use super::*;

    /// Returns the root of the project that the tests place their files in
    fn project() -> PathBuf {
        path("/home/otter/project")
    }

    /// Returns a diagnostic about a file of the project
    fn diagnostic(file: &str) -> ShfmtDiagnostic {
        ShfmtDiagnostic::builder()
            .path(project().join(file))
            .line(1)
            .column(1)
            .message("unexpected token".to_owned())
            .build()
    }

    /// Reads a line that shfmt wrote in the project of the tests
    fn parse(line: &str) -> Option<ShfmtDiagnostic> {
        ShfmtDiagnostic::parse().line(line).root(&project()).call()
    }

    #[test]
    fn parse_with_a_colon_in_the_message_keeps_the_message() {
        let parsed = parse("run.sh:2:4: reached EOF without closing quote: '");

        assert_eq!(
            parsed.map(|diagnostic| diagnostic.message),
            Some("reached EOF without closing quote: '".to_owned())
        );
    }

    #[test]
    fn parse_with_a_colon_in_the_path_keeps_the_path() {
        let parsed = parse("a:b.sh:3:7: unexpected token");

        assert_eq!(
            parsed.map(|diagnostic| diagnostic.path),
            Some(project().join("a:b.sh"))
        );
    }

    #[test]
    fn parse_with_a_position_reads_the_line_and_the_column() {
        let parsed = parse("sub/broken.sh:1:1: `if` must be followed by a statement list");

        assert_eq!(
            parsed,
            Some(
                ShfmtDiagnostic::builder()
                    .path(project().join("sub/broken.sh"))
                    .line(1)
                    .column(1)
                    .message("`if` must be followed by a statement list".to_owned())
                    .build()
            )
        );
    }

    #[test]
    fn parse_without_a_message_reads_nothing() {
        let parsed = parse("run.sh:1:1: ");

        assert_eq!(parsed, None);
    }

    #[test]
    fn parse_without_a_position_reads_nothing() {
        let parsed = parse("open locked.sh: permission denied");

        assert_eq!(parsed, None);
    }

    #[test]
    fn relative_path_outside_the_root_names_nothing() {
        let diagnostic = ShfmtDiagnostic::builder()
            .path(path("/home/otter/elsewhere/run.sh"))
            .line(1)
            .column(1)
            .message("unexpected token".to_owned())
            .build();

        let relative = diagnostic.relative_path(&ProjectRoot::new(project()));

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_under_the_root_drops_the_root() {
        let diagnostic = diagnostic("sub/run.sh");

        let relative = diagnostic.relative_path(&ProjectRoot::new(project()));

        assert_eq!(relative, FilePath::try_from(path("sub/run.sh")).ok());
    }

    #[test]
    fn shfmt_diagnostic_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ShfmtDiagnostic>();
    }
}
