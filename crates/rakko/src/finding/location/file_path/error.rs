//! The error type for file path parsing

use std::path::PathBuf;

use thiserror::Error;

/// An error that occurs when parsing a file path
///
/// A [`FilePath`] accepts only a path that is relative to the
/// project root. This error reports the path that it refused.
///
/// A later release can add a variant, such as for a rule that it adds. So a
/// match on this error outside this crate needs an arm for a variant that it
/// does not know.
///
/// [`FilePath`]: super::FilePath
///
/// # Examples
///
/// Handle an error when parsing an absolute path.
///
/// ```
/// use rakko::ParseFilePathError;
///
/// let absolute = std::env::temp_dir().join("file.txt");
/// let error = ParseFilePathError::AbsolutePath {
///     path: absolute.clone(),
/// };
///
/// assert_eq!(error.to_string(), format!("path is absolute: {}", absolute.display()));
/// ```
#[derive(Clone, Eq, PartialEq, Debug, Error)]
#[non_exhaustive]
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
