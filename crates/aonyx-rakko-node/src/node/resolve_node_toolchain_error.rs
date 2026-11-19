//! The error that leaves an action without a toolchain to run on Node

use rakko_tool::ResolveToolchainError;
use thiserror::Error;

/// An error that occurs when the crate resolves a toolchain of the project and
/// the Node that it runs on
///
/// Some toolchains of a project are packages for Node, such as TypeScript. The
/// variants separate that toolchain from Node, because each has a repair of
/// its own. Both belong to the project, so a pin of the project, or an install
/// of that pin, repairs either one. The variant says which of the two to
/// repair, and its source gives the cause.
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
/// use aonyx_rakko_node::ResolveNodeToolchainError;
/// use rakko::MiseTool;
/// use rakko_tool::ResolveToolchainError;
///
/// let error = ResolveNodeToolchainError::UnresolvedToolchain {
///     source: ResolveToolchainError::UnpinnedToolchain {
///         toolchain: MiseTool::new("npm:typescript"),
///         details: "no configuration inside the project pins a version".to_owned(),
///     },
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "failed to resolve the toolchain that runs on Node"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveNodeToolchainError {
    /// Mise reported no Node that the toolchain can run on
    ///
    /// The project pins no Node that the action accepts, mise did not install
    /// the pinned Node, or mise did not run. The directory of that Node can
    /// also be one that cannot be an entry of `PATH`. A toolchain without the
    /// pinned Node runs on the Node that comes first on the path. So the
    /// lookup stops here.
    #[error("failed to resolve the Node that the project pins")]
    UnresolvedNode {
        /// The cause of the failure
        source: ResolveToolchainError,
    },

    /// Mise reported no program of the toolchain that the project pins
    ///
    /// The project pins no version of the toolchain that the action accepts,
    /// mise did not install the pinned version, or mise did not run. The
    /// toolchain can also have no program by that name. Rakko installs
    /// nothing, so the lookup stops here.
    #[error("failed to resolve the toolchain that runs on Node")]
    UnresolvedToolchain {
        /// The cause of the failure
        source: ResolveToolchainError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_node_toolchain_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ResolveNodeToolchainError>();
    }
}
