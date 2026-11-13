//! The error that stops the lookup of the owners of a crate

use thiserror::Error;

/// An error that stops the lookup of the owners of a crate on crates.io
///
/// A crate that crates.io does not have is no error, because a run skips it.
/// The variants of this error describe an answer that the action cannot
/// trust, or no answer at all.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_check_crate_owners::LookUpOwnersError;
///
/// let error = LookUpOwnersError::UnexpectedStatus {
///     status: 503,
///     body: "service unavailable".to_owned(),
/// };
/// assert_eq!(
///     error.to_string(),
///     "crates.io answered with the status 503: service unavailable"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LookUpOwnersError {
    /// The request did not reach crates.io, or its answer did not arrive
    ///
    /// The network, the name service, the certificate of crates.io, a
    /// timeout, or a body that is not text can be the cause.
    #[error("failed to reach crates.io")]
    RegistryUnavailable {
        /// The cause of the failure
        source: std::io::Error,
    },

    /// crates.io answered with a status that the action does not expect
    ///
    /// The action expects the owners of the crate, or the answer that
    /// crates.io does not have the crate. Another status, such as the one of
    /// a request that came too fast, gives no answer about the owners.
    #[error("crates.io answered with the status {status}: {body}")]
    UnexpectedStatus {
        /// The HTTP status of the answer
        status: u16,

        /// The body of the answer
        body: String,
    },

    /// crates.io listed the owners in a shape that the action cannot read
    #[error("failed to read the owners that crates.io listed")]
    UnrecognizedOwners {
        /// The cause of the failure
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn look_up_owners_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<super::LookUpOwnersError>();
    }
}
