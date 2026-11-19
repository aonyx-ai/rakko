//! The error that leaves an action without a prettier to run

use rakko_tool::{ResolveToolError, ResolveToolchainError};
use thiserror::Error;

/// An error that occurs when the crate resolves prettier and the Node that it
/// runs on
///
/// The variants separate the tool from the toolchain, because each has a
/// repair of its own. The crate pins prettier, so an install of that version
/// repairs a missing prettier. Node is a toolchain of the project, so a pin
/// of the project, or an install of that pin, repairs a missing Node. The
/// source of each variant gives the repair.
///
/// Nothing started, so there is no exit status and no output to read.
///
/// A later release can add a variant for another reason why prettier cannot
/// be resolved. So a match on this error outside this crate needs an arm for a
/// variant that it does not know.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_prettier::ResolvePrettierError;
/// use rakko::MiseTool;
/// use rakko_tool::ResolveToolError;
///
/// let error = ResolvePrettierError::UnresolvedTool {
///     source: ResolveToolError::UndeclaredTool {
///         tool: MiseTool::new("npm:prettier"),
///     },
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "failed to resolve the prettier that the crate pins"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolvePrettierError {
    /// Mise reported no prettier at the version that the crate pins
    ///
    /// The action does not declare the tools of the crate, mise did not run,
    /// or nothing installed that version of prettier. Rakko installs nothing,
    /// so the run stops here.
    #[error("failed to resolve the prettier that the crate pins")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },

    /// Mise reported no Node that prettier can run on
    ///
    /// The project pins no Node that the action accepts, mise did not install
    /// the pinned Node, or mise did not run. The directory of that Node can
    /// also be one that cannot be an entry of `PATH`. Without the pinned Node,
    /// prettier would run on whatever Node comes first on the path, so the
    /// run stops here.
    #[error("failed to resolve the Node that prettier runs on")]
    UnresolvedToolchain {
        /// The cause of the failure
        source: ResolveToolchainError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_prettier_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ResolvePrettierError>();
    }
}
