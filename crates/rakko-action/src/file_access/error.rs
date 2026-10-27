use std::io::Error as IoError;
use std::path::PathBuf;

use thiserror::Error;

/// An error that occurs when expanding a declaration into the files of a
/// project
///
/// The expansion lists every directory below the project root. It stops at
/// the first directory that it cannot list, because a list without the files
/// of that directory would miss files that a run reads or writes.
#[derive(Debug, Error)]
pub enum ExpandFilesError {
    /// A directory of the project, or the root itself, cannot be listed
    #[error("failed to list the directory {}", directory.display())]
    UnreadableDirectory {
        /// The directory that cannot be listed
        directory: PathBuf,
        /// The error of the file system
        source: IoError,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::io::ErrorKind;

    use super::*;

    #[test]
    fn unreadable_directory_reports_the_directory() {
        let error = ExpandFilesError::UnreadableDirectory {
            directory: PathBuf::from("src"),
            source: IoError::from(ErrorKind::PermissionDenied),
        };

        let message = error.to_string();

        assert_eq!(message, "failed to list the directory src");
    }
}
