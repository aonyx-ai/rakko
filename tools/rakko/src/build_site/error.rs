//! The errors that stop a build of the website

use rakko_tool::{ResolveRequirementError, RunCommandError};
use thiserror::Error;

/// An error that stops a run of the build-site action
///
/// A build that Zola ends without success is no error. The run reports it as
/// a finding instead.
#[derive(Debug, Error)]
pub(crate) enum BuildSiteError {
    /// Mise found no Zola that the project pins in the range of the action
    #[error("failed to resolve zola")]
    UnresolvedZola {
        /// The cause of the failure, with the tool and the version
        source: ResolveRequirementError,
    },

    /// Zola did not start, so nothing was built
    #[error("failed to start zola")]
    ZolaUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },
}
