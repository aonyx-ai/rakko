//! The error type for directory path parsing

use std::path::PathBuf;

use thiserror::Error;

/// An error that occurs when parsing a directory path
///
/// A [`DirectoryPath`] accepts only a path that is relative to the project
/// root. This error reports the path that it refused.
///
/// A later release can add a variant, such as for a rule that it adds. So a
/// match on this error outside this crate needs an arm for a variant that it
/// does not know.
///
/// [`DirectoryPath`]: super::DirectoryPath
#[derive(Clone, Eq, PartialEq, Debug, Error)]
#[non_exhaustive]
pub enum ParseDirectoryPathError {
    /// The path starts at the root of the file system
    #[error("path is absolute: {}", path.display())]
    AbsolutePath {
        /// The path that the caller gave
        path: PathBuf,
    },
}
