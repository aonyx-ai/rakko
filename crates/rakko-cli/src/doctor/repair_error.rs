//! What `doctor --fix` reports when it cannot repair the tool pins

use std::io;
use std::path::PathBuf;

use rakko_tool::RunCommandError;
use thiserror::Error;

use super::check_error::CheckToolPinsError;

/// An error that occurs when `doctor --fix` repairs the tool pins of a
/// project
///
/// The repairs that happened before the error stay on disk. A second run
/// finds them done, and it continues with the rest.
#[derive(Debug, Error)]
pub(crate) enum RepairToolPinsError {
    /// Mise did not run when the repair asked it to lock the tools
    #[error("failed to ask mise to lock the tools of the project")]
    MiseUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// The check that comes before the repair failed
    #[error("failed to check the tool pins of the project")]
    UncheckedPins {
        /// The cause of the failure
        source: CheckToolPinsError,
    },

    /// Mise ended without success when it locked the tools of the project
    #[error("mise did not lock the tools of the project: {details}")]
    UnlockedTools {
        /// What mise wrote about the failure
        details: String,
    },

    /// A configuration file of the project is not valid TOML
    #[error("failed to read the configuration file `{}` as TOML", path.display())]
    UnparsableConfiguration {
        /// The location of the configuration file
        path: PathBuf,

        /// The cause of the failure
        source: toml_edit::TomlError,
    },

    /// The file system refused to give the content of a configuration file,
    /// or to list a directory of them
    #[error("failed to read the configuration of mise at `{}`", path.display())]
    UnreadableConfiguration {
        /// The location of the configuration file or of the directory
        path: PathBuf,

        /// The cause of the failure
        source: io::Error,
    },

    /// The file system refused to write or to remove a file, or to create its
    /// directory
    #[error("failed to write `{}`", path.display())]
    UnwritableFile {
        /// The location of the file
        path: PathBuf,

        /// The cause of the failure
        source: io::Error,
    },
}

impl From<CheckToolPinsError> for RepairToolPinsError {
    fn from(source: CheckToolPinsError) -> Self {
        Self::UncheckedPins { source }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// The run puts the error in an outcome, and that outcome holds an error
    /// that another thread can read.
    #[test]
    fn repair_tool_pins_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<RepairToolPinsError>();
    }
}
