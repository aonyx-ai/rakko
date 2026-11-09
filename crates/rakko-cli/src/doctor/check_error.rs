//! What `doctor` reports when it cannot check the tool pins

use std::io;
use std::path::PathBuf;

use rakko::Name;
use rakko_tool::{ResolveRequirementError, RunCommandError};
use thiserror::Error;

/// An error that occurs when `doctor` checks the tool pins of a project
///
/// No variant is a problem of the project. A problem of the project is a
/// finding of the report. An error means that `doctor` could not get an
/// answer, so the run stops.
#[derive(Debug, Error)]
pub(crate) enum CheckToolPinsError {
    /// Mise did not run
    #[error("failed to ask mise for the configuration files of the project")]
    MiseUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// The generated file exists, and the file system refused to give its
    /// content
    #[error("failed to read the generated file of the tool pins `{}`", path.display())]
    UnreadableGeneratedFile {
        /// The location of the generated file
        path: PathBuf,

        /// The cause of the failure
        source: io::Error,
    },

    /// Mise ended without success when it listed the configuration files of
    /// the project
    #[error("mise listed no configuration files of the project: {details}")]
    UnlistedConfiguration {
        /// What mise wrote about the failure
        details: String,
    },

    /// Mise listed the configuration files in a form that `doctor` does not
    /// read
    #[error("failed to read the configuration files that mise lists for the project")]
    UnrecognizedConfiguration {
        /// The cause of the failure
        source: serde_json::Error,
    },

    /// The resolution of a toolchain failed for a reason that no pin of the
    /// project repairs
    #[error("failed to check a toolchain that the action `{action}` requires")]
    UnresolvedToolchain {
        /// The action that requires the toolchain
        action: Name,

        /// The cause of the failure, which gives the toolchain
        source: ResolveRequirementError,
    },
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
    fn check_tool_pins_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CheckToolPinsError>();
    }
}
