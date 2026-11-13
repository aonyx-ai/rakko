//! The error that stops a run of the action

use aonyx_rakko_cargo::{DiscoverRootsError, PackageName};
use rakko_tool::ResolveRequirementError;
use thiserror::Error;

use crate::crates_io::LookUpOwnersError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A missing owner travels as a
/// finding in the outcome of the run, and so does an owner that a repair
/// could not add. The variants of this error describe a run whose answer
/// cannot be trusted, and such a run stops instead of reporting one.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_cargo::PackageName;
/// use aonyx_rakko_check_crate_owners::{CheckCrateOwnersError, LookUpOwnersError};
///
/// let error = CheckCrateOwnersError::UnreadableOwners {
///     name: PackageName::new("otter"),
///     source: LookUpOwnersError::UnexpectedStatus {
///         status: 503,
///         body: "service unavailable".to_owned(),
///     },
/// };
/// assert_eq!(
///     error.to_string(),
///     "failed to read the owners of otter from crates.io"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CheckCrateOwnersError {
    /// The client that reads the owners from crates.io could not be set up
    ///
    /// The client carries its own TLS, so this points at a broken build of
    /// the action and not at the network.
    #[error("failed to set up the client for crates.io")]
    ClientUnavailable {
        /// The cause of the failure
        source: std::io::Error,
    },

    /// The workspaces of the project could not be discovered
    ///
    /// A run checks every crate of every workspace of the project. So a run
    /// that does not know the workspaces would check less than the project
    /// and report a green result for the rest.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// The owners of a crate could not be read from crates.io
    ///
    /// A run that skipped the crate would report a green result for a crate
    /// that nothing checked.
    #[error("failed to read the owners of {name} from crates.io")]
    UnreadableOwners {
        /// The crate whose owners the run asked for
        name: PackageName,

        /// The cause of the failure
        source: LookUpOwnersError,
    },

    /// Mise reported no cargo for the project
    ///
    /// Cargo describes the workspaces that a run checks, and it adds an
    /// owner with `fix`. The project pins no stable Rust toolchain that the
    /// action accepts, or nothing installed the pin yet. The action installs
    /// nothing, so the run stops here.
    #[error("failed to resolve cargo")]
    UnresolvedCargo {
        /// The cause of the failure
        source: ResolveRequirementError,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_crate_owners_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<super::CheckCrateOwnersError>();
    }
}
