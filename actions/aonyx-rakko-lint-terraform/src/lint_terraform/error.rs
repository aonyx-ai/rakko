//! The error that stops a run of the action

use std::path::PathBuf;

use rakko_tool::{ResolveRequirementError, RunCommandError};
use thiserror::Error;

use crate::tflint::ObserveTflintError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem that a rule found. An issue travels as a finding
/// in the outcome of the run. The variants of this error describe a run whose
/// answer cannot be trusted, and such a run stops instead of reporting one.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_lint_terraform::LintTerraformError;
///
/// let error = LintTerraformError::IncompleteInspection {
///     messages: "Failed to load the root module".to_owned(),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "TFLint could not inspect every module:\nFailed to load the root module"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LintTerraformError {
    /// TFLint reported a path that the project root does not contain
    ///
    /// A finding gives its path relative to the project root, and a path
    /// outside the root has no such form. TFLint walks the root, so this
    /// points at a report that the action misread.
    #[error("TFLint reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that TFLint reported
        path: PathBuf,
    },

    /// TFLint could not inspect every module
    ///
    /// TFLint reports an error for a module that it cannot load, such as a
    /// module with a file that Terraform cannot parse. An outcome built on
    /// such a run would describe a part of the project as though it were the
    /// whole.
    #[error("TFLint could not inspect every module:\n{messages}")]
    IncompleteInspection {
        /// What TFLint wrote about each module that it could not inspect
        messages: String,
    },

    /// TFLint did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run TFLint")]
    TflintUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// TFLint wrote a report that the action does not recognize
    ///
    /// The report is not the JSON that TFLint writes, or it does not explain
    /// a run that failed. The findings of such a run would be the ones that
    /// the reading happened to understand.
    #[error("TFLint wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What TFLint wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no TFLint at the version that the action declares
    ///
    /// Mise did not run, or nothing installed that version of TFLint yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve TFLint")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveRequirementError,
    },
}

impl From<ObserveTflintError> for LintTerraformError {
    /// Turns the failure of a TFLint run into the error of the action
    ///
    /// The machinery that runs TFLint gives the same three conditions that the
    /// action reports, so the conversion renames them and adds nothing.
    fn from(error: ObserveTflintError) -> Self {
        match error {
            ObserveTflintError::IncompleteInspection { messages } => {
                Self::IncompleteInspection { messages }
            }
            ObserveTflintError::TflintUnavailable { source } => Self::TflintUnavailable { source },
            ObserveTflintError::UnrecognizedReport { stderr } => {
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

    /// What TFLint wrote about a module that it could not load
    const MESSAGES: &str = "Failed to load the root module";

    #[test]
    fn error_of_an_incomplete_inspection_holds_what_tflint_wrote() {
        let error = LintTerraformError::from(ObserveTflintError::IncompleteInspection {
            messages: MESSAGES.to_owned(),
        });

        assert!(
            matches!(&error, LintTerraformError::IncompleteInspection { messages } if messages == MESSAGES),
            "expected what TFLint wrote, got {error:?}"
        );
    }

    #[test]
    fn lint_terraform_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<LintTerraformError>();
    }
}
