use rakko_action::ParseFilePathError;
use rakko_tool::ResolveToolError;
use thiserror::Error;

use crate::comparison::PrepareComparisonError;
use crate::tracey::ObserveTraceyError;

/// An error that stops a run of the action
///
/// A broken link between a specification and the code is no error: it travels
/// as a finding, and the outcome of the run reports it. The variants here
/// describe a run that reached no answer about the project at all.
#[derive(Debug, Error)]
pub enum CheckSpecsError {
    /// Tracey reported a problem in a file outside the project
    ///
    /// Tracey starts in the root of the project and names a file below it, so
    /// a path that the root does not contain points at a tracey that reports
    /// something else than the crate expects.
    #[error("tracey reported a problem in {path}, which is not a file of the project")]
    ForeignPath {
        /// The path that tracey reported
        path: String,

        /// The cause of the failure
        source: ParseFilePathError,
    },

    /// Tracey answered no question about the project
    #[error("failed to ask tracey about the specifications of the project")]
    UnansweredQuestion {
        /// The cause of the failure
        source: ObserveTraceyError,
    },

    /// The comparison that a pull request needs could not be built
    #[error("failed to compare the pull request with its base branch")]
    UnpreparedComparison {
        /// The cause of the failure
        source: PrepareComparisonError,
    },

    /// Mise reported no tracey for the project
    ///
    /// Provisioning is the job of mise, and the action installs nothing, so a
    /// tracey that nobody installed stops the run.
    #[error("failed to resolve tracey")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveTraceyError> for CheckSpecsError {
    fn from(source: ObserveTraceyError) -> Self {
        Self::UnansweredQuestion { source }
    }
}

impl From<PrepareComparisonError> for CheckSpecsError {
    fn from(source: PrepareComparisonError) -> Self {
        Self::UnpreparedComparison { source }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read.
    #[test]
    fn check_specs_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<CheckSpecsError>();
    }
}
