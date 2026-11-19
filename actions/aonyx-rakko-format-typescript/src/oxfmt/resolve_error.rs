//! The error that leaves a run without an oxfmt to start

use rakko_tool::{ResolveToolError, ResolveToolchainError};
use thiserror::Error;

/// An error that occurs when the crate resolves oxfmt and the Node that it
/// runs on
///
/// The variants separate the tool from the toolchain, because each has a
/// repair of its own. The action pins oxfmt, so an install of that version
/// repairs a missing oxfmt. Node is a toolchain of the project, so a pin of
/// the project, or an install of that pin, repairs a missing Node.
#[derive(Debug, Error)]
pub(crate) enum ResolveOxfmtError {
    /// Mise reported no oxfmt at the version that the action declares
    #[error("failed to resolve the oxfmt that the action declares")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },

    /// Mise reported no Node that oxfmt can run on
    #[error("failed to resolve the Node that oxfmt runs on")]
    UnresolvedToolchain {
        /// The cause of the failure
        source: ResolveToolchainError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_oxfmt_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ResolveOxfmtError>();
    }
}
