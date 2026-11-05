//! The error type for directory path parsing

use std::path::PathBuf;

use thiserror::Error;

/// An error that occurs when parsing a directory path
///
/// A [`DirectoryPath`] accepts only a path that is relative to the project
/// root. This error reports the path that it refused.
///
/// [`DirectoryPath`]: super::DirectoryPath
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum ParseDirectoryPathError {
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
    fn parse_directory_path_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ParseDirectoryPathError>();
    }
}
