//! The error that leaves an action without a tool to run on Node

use rakko_tool::{ResolveToolError, ResolveToolchainError};
use thiserror::Error;

/// An error that occurs when the crate resolves a tool and the Node that it
/// runs on
///
/// The variants separate the tool from Node, because each has a repair of its
/// own. The source of each variant gives the repair. The crate that wraps a
/// tool pins it, so an install of that version repairs a missing tool. Node is
/// a toolchain of the project, so a pin of the project, or an install of that
/// pin, repairs a missing Node.
///
/// Nothing started, so there is no exit status and no output to read.
///
/// A later release can add a variant for another reason why the lookup fails.
/// So a match on this error outside this crate needs an arm for a variant that
/// it does not know.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_node::ResolveNodeToolError;
/// use rakko::MiseTool;
/// use rakko_tool::ResolveToolError;
///
/// let error = ResolveNodeToolError::UnresolvedTool {
///     source: ResolveToolError::UndeclaredTool {
///         tool: MiseTool::new("npm:prettier"),
///     },
/// };
///
/// assert_eq!(error.to_string(), "failed to resolve the tool that runs on Node");
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveNodeToolError {
    /// Mise reported no Node that the tool can run on
    ///
    /// The project pins no Node that the action accepts, mise did not install
    /// the pinned Node, or mise did not run. The directory of that Node can
    /// also be one that cannot be an entry of `PATH`. A tool without the
    /// pinned Node runs on the Node that comes first on the path. So the
    /// lookup stops here.
    #[error("failed to resolve the Node that the project pins")]
    UnresolvedNode {
        /// The cause of the failure
        source: ResolveToolchainError,
    },

    /// Mise reported no program of the tool at the version that the action
    /// declares
    ///
    /// The action does not declare the tool, mise did not run, or nothing
    /// installed that version of the tool. The tool can also have no program
    /// by that name. Rakko installs nothing, so the lookup stops here.
    #[error("failed to resolve the tool that runs on Node")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_node_tool_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ResolveNodeToolError>();
    }
}
