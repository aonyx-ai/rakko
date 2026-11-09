//! The cause of a failure to parse a file of the project as TOML

use thiserror::Error;

/// The cause of a failure to parse a file of the project as TOML
///
/// The error keeps the message of the TOML parser, which usually gives the
/// line and the column of the problem. It keeps no other part of the error of
/// the parser, so a later release can change the parser without a break.
///
/// # Examples
///
/// A caller reads the message of the parser from the error:
///
/// ```
/// use rakko_set_msrv::ParseTomlError;
///
/// fn message_of(error: &ParseTomlError) -> String {
///     error.to_string()
/// }
/// ```
#[derive(Debug, Error)]
#[error("{message}")]
pub struct ParseTomlError {
    /// What the TOML parser said about the file
    message: String,
}

impl ParseTomlError {
    /// Returns the cause with the message of an error of the TOML parser
    pub(crate) fn new(source: &toml_edit::TomlError) -> Self {
        Self {
            message: source.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toml_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ParseTomlError>();
    }
}
