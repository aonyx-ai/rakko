//! The error that stops a run of the action

use std::path::PathBuf;

use rakko_tool::{ResolveToolchainError, RunCommandError};
use thiserror::Error;

use crate::terraform::ObserveTerraformError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the formatting. A file that is not formatted
/// travels as a finding in the outcome of the run. The variants of this error
/// describe a run whose answer cannot be trusted, and such a run stops instead
/// of reporting one.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_format_terraform::FormatTerraformError;
///
/// let error = FormatTerraformError::UnreadableConfiguration {
///     diagnostics: "Error: Unclosed configuration block".to_owned(),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "Terraform stopped before it read the whole project:\nError: Unclosed configuration block"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FormatTerraformError {
    /// Terraform reported a path that the project root does not contain
    ///
    /// A finding gives its path relative to the project root, and a path
    /// outside the root has no such form. Terraform walks the root, so this
    /// points at a report that the action misread.
    #[error("Terraform reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that Terraform reported
        path: PathBuf,
    },

    /// Terraform did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run Terraform")]
    TerraformUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Terraform stopped before it read the whole project
    ///
    /// Terraform stops at the first file that it cannot parse, and the files
    /// after it stay unread. A release of Terraform also stops at an option of
    /// the formatter that it does not know. An outcome built on such a run
    /// would describe a part of the project as though it were the whole.
    #[error("Terraform stopped before it read the whole project:\n{diagnostics}")]
    UnreadableConfiguration {
        /// The diagnostics that Terraform wrote
        diagnostics: String,
    },

    /// Terraform wrote a report that the action does not recognize
    ///
    /// A check that found a file that is not formatted lists it. A check that
    /// ends with the status of such a file and lists none leaves the run
    /// unexplained, so the run stops instead of passing.
    #[error("Terraform wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What Terraform wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no Terraform that the project pins
    ///
    /// The project pins no Terraform, or nothing installed the version that it
    /// pins. The action installs nothing, so the run stops here.
    #[error("failed to resolve Terraform")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolchainError,
    },
}

impl From<ObserveTerraformError> for FormatTerraformError {
    /// Turns the failure of a Terraform run into the error of the action
    ///
    /// The machinery that runs Terraform gives the same three conditions that
    /// the action reports, so the conversion renames them and adds nothing.
    fn from(error: ObserveTerraformError) -> Self {
        match error {
            ObserveTerraformError::TerraformUnavailable { source } => {
                Self::TerraformUnavailable { source }
            }
            ObserveTerraformError::UnreadableConfiguration { diagnostics } => {
                Self::UnreadableConfiguration { diagnostics }
            }
            ObserveTerraformError::UnrecognizedReport { stderr } => {
                Self::UnrecognizedReport { stderr }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// What Terraform wrote about a file that it could not parse
    const DIAGNOSTICS: &str = "Error: Unclosed configuration block";

    #[test]
    fn error_of_an_unreadable_configuration_holds_the_diagnostics() {
        let error = FormatTerraformError::from(ObserveTerraformError::UnreadableConfiguration {
            diagnostics: DIAGNOSTICS.to_owned(),
        });

        assert!(
            matches!(&error, FormatTerraformError::UnreadableConfiguration { diagnostics } if diagnostics == DIAGNOSTICS),
            "expected the diagnostics of Terraform, got {error:?}"
        );
    }

    #[test]
    fn format_terraform_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<FormatTerraformError>();
    }
}
