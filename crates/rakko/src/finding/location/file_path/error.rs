//! The error type for file path parsing

use std::path::PathBuf;

use thiserror::Error;

/// An error that occurs when parsing a file path
///
/// A [`FilePath`] accepts only a path that is relative to the
/// project root. This error reports the path that it refused.
///
/// [`FilePath`]: super::FilePath
///
/// # Examples
///
/// Handle an error when parsing an absolute path.
///
/// ```
/// use std::path::PathBuf;
/// use rakko::finding::ParseFilePathError;
///
/// let error = ParseFilePathError::AbsolutePath {
///     path: PathBuf::from("/tmp/project/file.txt"),
/// };
///
/// assert_eq!(error.to_string(), "path is absolute: /tmp/project/file.txt");
/// ```
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum ParseFilePathError {
    /// The path starts at the root of the file system
    #[error("path is absolute: {}", path.display())]
    AbsolutePath {
        /// The path that the caller gave
        path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_path_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ParseFilePathError>();
    }
}
