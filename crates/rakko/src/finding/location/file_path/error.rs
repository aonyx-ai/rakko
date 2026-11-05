use std::path::PathBuf;

use thiserror::Error;

/// An error that occurs when parsing a file path
///
/// A [`FilePath`](super::FilePath) accepts only a path that is relative to the
/// project root. This error reports the path that it refused.
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum ParseFilePathError {
    /// The path starts at the root of the file system
    #[error("path is absolute: {}", path.display())]
    AbsolutePath {
        /// The path that the caller gave
        path: PathBuf,
    },
}
